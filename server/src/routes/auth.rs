use axum::{
    Router, middleware,
    routing::{patch, post},
};

use crate::{
    handlers::auth::{
        confirm_email, forgot_password, google_login, login, logout, refresh, register,
        reset_password,
    },
    middleware::rate_limit::auth_rate_limit,
    state::AppState,
};

pub fn router(state: AppState) -> Router<AppState> {
    // Everything that lets an anonymous caller guess credentials or trigger
    // an email is rate limited per client.
    let rate_limited = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/google-login", post(google_login))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .route("/confirm-email", patch(confirm_email))
        .route_layer(middleware::from_fn_with_state(state, auth_rate_limit));

    let session = Router::new()
        .route("/refresh", post(refresh))
        .route("/logout", post(logout));

    Router::new().merge(rate_limited).merge(session)
}
