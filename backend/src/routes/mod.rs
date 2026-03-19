use axum::Router;
use sqlx::PgPool;

mod auth;
mod rooms;
mod user;

pub fn app_router(state: crate::state::AppState) -> Router<crate::state::AppState> {
	Router::new().nest("/auth", auth::router(state.clone())).nest("/rooms", rooms::router(state.clone())).nest("/users", user::router(state.clone()))
}
