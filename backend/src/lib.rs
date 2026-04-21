#![allow(unused_variables, unused_imports, dead_code)]

use crate::state::AppState;
use axum::http::Method;
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod db;
pub mod dtos;
pub mod errors;
pub mod handlers;
mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;

pub async fn run() {
    dotenv().ok();
    setup_tracing();

    let pool = db::create_pool(&get_env("DATABASE_URL")).await;
    tracing::info!("Connected to PostgreSQL");

    let state = AppState {
        pool: pool.clone(),
        jwt_secret: get_env("JWT_SECRET"),
        google_client_id: get_env("GOOGLE_CLIENT_ID"),
        google_client_secret: get_env("GOOGLE_CLIENT_SECRET"),
        google_auth_url: env::var("GOOGLE_AUTH_URL")
            .unwrap_or_else(|_| "https://oauth2.googleapis.com".to_string()),
        active_rooms: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = build_router(state);

    services::cleanup::spawn_token_cleanup_task(pool);

    start_server(app, "0.0.0.0:3000").await;
}

fn get_env(var_name: &str) -> String {
    env::var(var_name).unwrap_or_else(|_| panic!("{} must be set", var_name))
}

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .init();
}

fn build_router(state: AppState) -> axum::Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(Any);

    routes::app_router(state.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn start_server(app: axum::Router, addr: &str) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Backend running on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
