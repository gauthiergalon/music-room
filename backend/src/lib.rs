use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use axum::http::{HeaderName, HeaderValue, Method};
use dotenv::dotenv;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    middleware::{logging::request_logger, rate_limit::RateLimiter},
    repositories::rooms as rooms_repo,
    services::{
        email::{SmtpConfig, SmtpMailer},
        google::{DEFAULT_JWKS_URL, GoogleAuth},
        music_provider::hifi::HifiProvider,
        stream_proxy::StreamProxy,
    },
    state::AppState,
    ws::manager::RoomManager,
};

mod db;
pub mod dtos;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;
pub mod validation;
pub mod ws;

const DEFAULT_AUTH_RATE_LIMIT_PER_MINUTE: u32 = 30;
/// How long in-flight requests get to finish once the server is asked to stop.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

pub async fn run() {
    dotenv().ok();
    setup_tracing();

    let pool = match db::create_pool(&required_env("DATABASE_URL")).await {
        Ok(pool) => pool,
        Err(e) => exit_with(format!("Failed to connect to PostgreSQL: {e}")),
    };
    tracing::info!("Connected to PostgreSQL");

    if let Err(e) = db::run_migrations(&pool).await {
        exit_with(format!("Failed to apply database migrations: {e}"));
    }

    // No client is connected yet: give every room the time to see its owner
    // reconnect before the cleanup task considers it abandoned.
    if let Err(e) = rooms_repo::touch_all(&pool).await {
        tracing::warn!("Failed to refresh room activity at startup: {:?}", e);
    }

    let mailer = match SmtpMailer::new(SmtpConfig::from_env()) {
        Ok(mailer) => mailer,
        Err(e) => exit_with(e),
    };

    let jwt_secret = required_env("JWT_SECRET");
    let state = AppState {
        pool: pool.clone(),
        stream_proxy: Arc::new(StreamProxy::from_env(&jwt_secret)),
        jwt_secret,
        google: Arc::new(GoogleAuth::new(
            required_env("GOOGLE_CLIENT_ID"),
            env::var("GOOGLE_JWKS_URL").unwrap_or_else(|_| DEFAULT_JWKS_URL.to_string()),
        )),
        rooms: Arc::new(RoomManager::default()),
        music_provider: Arc::new(HifiProvider::from_env()),
        mailer: Arc::new(mailer),
        auth_rate_limiter: auth_rate_limiter_from_env(),
    };

    services::cleanup::spawn_token_cleanup_task(pool.clone());
    services::cleanup::spawn_subscription_cleanup_task(pool);
    services::cleanup::spawn_room_cleanup_task(state.clone());

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    start_server(build_router(state), &format!("0.0.0.0:{port}")).await;
}

fn required_env(var_name: &str) -> String {
    env::var(var_name).unwrap_or_else(|_| exit_with(format!("{var_name} must be set")))
}

fn exit_with(message: String) -> ! {
    tracing::error!("{message}");
    std::process::exit(1);
}

/// `AUTH_RATE_LIMIT_PER_MINUTE` requests per client and per minute on the
/// public auth routes; `0` disables the limit (load tests).
fn auth_rate_limiter_from_env() -> Option<Arc<RateLimiter>> {
    let per_minute = env::var("AUTH_RATE_LIMIT_PER_MINUTE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_AUTH_RATE_LIMIT_PER_MINUTE);

    (per_minute > 0).then(|| Arc::new(RateLimiter::new(per_minute, Duration::from_secs(60))))
}

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=info,tower_http=info".into()),
        )
        .init();
}

/// `CORS_ALLOWED_ORIGINS` is a comma-separated list of origins; unset or `*` allows any.
fn cors_layer() -> CorsLayer {
    let allowed_origin = match env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins) if origins.trim() != "*" => AllowOrigin::list(
            origins
                .split(',')
                .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok()),
        ),
        _ => AllowOrigin::any(),
    };

    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
            HeaderName::from_static("x-platform"),
            HeaderName::from_static("x-device"),
            HeaderName::from_static("x-app-version"),
        ])
        .allow_origin(allowed_origin)
}

fn build_router(state: AppState) -> axum::Router {
    routes::app_router(state.clone())
        .layer(cors_layer())
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(request_logger))
        .with_state(state)
}

async fn start_server(app: axum::Router, addr: &str) {
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => exit_with(format!("Failed to listen on {addr}: {e}")),
    };
    tracing::info!("Backend running on {}", addr);

    let (shutdown_started, shutdown_started_rx) = tokio::sync::oneshot::channel::<()>();
    let server = std::future::IntoFuture::into_future(
        axum::serve(
            listener,
            // The peer address is what the rate limiter falls back to without a proxy.
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = shutdown_started.send(());
        }),
    );

    // A graceful shutdown waits for every connection to end, and a WebSocket
    // never ends by itself: give in-flight requests a moment, then stop.
    let forced_stop = async move {
        if shutdown_started_rx.await.is_ok() {
            tokio::time::sleep(SHUTDOWN_GRACE).await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    tokio::select! {
        served = server => {
            if let Err(e) = served {
                exit_with(format!("Server error: {e}"));
            }
        }
        _ = forced_stop => tracing::info!("Closing the remaining connections"),
    }
}

/// Resolves on Ctrl+C or SIGTERM so in-flight requests can finish before exit.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutting down");
}
