use axum::{Router, http::Method};
use dotenv::dotenv;
use std::env;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod db;
// mod middleware;
// mod models;
// mod routes;

#[tokio::main]
async fn main() {
	dotenv().ok();

	// Initialise le logger avec un niveau par défaut, mais permet d'override avec RUST_LOG
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
		)
		.init();

	// Connexion PostgreSQL
	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

	let pool = db::create_pool(&database_url).await;
	tracing::info!("Connected to PostgreSQL");

	// CORS — permet à Flutter de parler au backend
	let cors = CorsLayer::new()
		.allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
		.allow_headers(Any)
		.allow_origin(Any);

	// Routes
	let app = Router::new()
		// .nest("/auth", routes::auth::router())
		// .nest("/rooms", routes::rooms::router())
		.layer(cors)
		.layer(TraceLayer::new_for_http())
		.with_state(pool);

	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

	tracing::info!("Backend running on port 3000");
	axum::serve(listener, app).await.unwrap();
}
