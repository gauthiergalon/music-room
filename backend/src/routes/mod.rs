use axum::Router;
use sqlx::PgPool;

mod auth;
mod rooms;
mod user;

pub fn app_router() -> Router<PgPool> {
	Router::new().nest("/auth", auth::router()).nest("/rooms", rooms::router()).nest("/users", user::router())
}
