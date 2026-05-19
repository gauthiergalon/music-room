use std::{collections::HashMap, env, sync::Arc};

use axum::http::{HeaderName, HeaderValue, Method};
use dotenv::dotenv;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{middleware::logging::request_logger, state::AppState};

mod db;
pub mod dtos;
pub mod errors;
pub mod handlers;
mod middleware;
pub mod models;
pub mod openapi;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;
pub mod ws;

pub async fn run() {
    dotenv().ok();
    setup_tracing();

    let pool = db::create_pool(&get_env("DATABASE_URL")).await;
    tracing::info!("Connected to PostgreSQL");

    let active_rooms = Arc::new(RwLock::new(HashMap::new()));
    let state = AppState {
        pool: pool.clone(),
        jwt_secret: get_env("JWT_SECRET"),
        google_client_id: get_env("GOOGLE_CLIENT_ID"),
        google_client_secret: get_env("GOOGLE_CLIENT_SECRET"),
        google_auth_url: env::var("GOOGLE_AUTH_URL")
            .unwrap_or_else(|_| "https://oauth2.googleapis.com".to_string()),
        active_rooms: active_rooms.clone(),
    };

    let app = build_router(state);

    services::cleanup::spawn_token_cleanup_task(pool.clone());
    services::cleanup::spawn_room_cleanup_task(pool.clone(), active_rooms);

    start_server(app, "0.0.0.0:3000").await;
}

fn get_env(var_name: &str) -> String {
    env::var(var_name).unwrap_or_else(|_| panic!("{} must be set", var_name))
}

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=info,tower_http=info".into()),
        )
        .init();
}

fn build_router(state: AppState) -> axum::Router {
    let allowed_origins: Vec<HeaderValue> = env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost".to_string())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let cors = CorsLayer::new()
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
        ])
        .allow_origin(allowed_origins);

    routes::app_router(state.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(request_logger))
        .with_state(state)
}

async fn start_server(app: axum::Router, addr: &str) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Backend running on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
