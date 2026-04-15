use crate::{
    handlers::auth::{forgot_password, login, logout, refresh, register, reset_password, google_login},
    middleware::auth::auth_middleware,
    state::AppState,
};
use axum::{Router, middleware, routing::post};
use sqlx::PgPool;

pub fn router(state: AppState) -> Router<AppState> {
    let public = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/google-login", post(google_login))
        .route("/refresh", post(refresh))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password));

    let protected =
        Router::new()
            .route("/logout", post(logout))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ));

    Router::new().merge(public).merge(protected)
}
