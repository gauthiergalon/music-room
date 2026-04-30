use axum::Router;
use sqlx::PgPool;

use crate::state::AppState;

mod auth;
mod friends;
mod hifi;
mod invitations;
mod rooms;
mod user;

pub fn app_router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router(state.clone()))
        .nest("/rooms", rooms::router(state.clone()))
        .nest("/users", user::router(state.clone()))
        .nest("/friends", friends::router(state.clone()))
        .nest("/hifi", hifi::router(state.clone()))
        .merge(invitations::router(state.clone()))
}
