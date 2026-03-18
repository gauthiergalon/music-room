#![allow(unused_variables, unused_imports, dead_code)]

use axum::http::Method;
use dotenv::dotenv;
use std::env;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod db;
pub mod errors;
mod middleware;
pub mod routes;
pub mod services;

pub async fn run() {
	dotenv().ok();

	tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "backend=debug,tower_http=debug".into())).init();

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

	let pool = db::create_pool(&database_url).await;
	tracing::info!("Connected to PostgreSQL");

	let cors = CorsLayer::new().allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]).allow_headers(Any).allow_origin(Any);

	let app = routes::app_router().layer(cors).layer(TraceLayer::new_for_http()).with_state(pool);

	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

	tracing::info!("Backend running on port 3000");
	axum::serve(listener, app).await.unwrap();
}
