use axum::{Router, middleware, routing::get};

use crate::{
    handlers::pages::{confirm_email, confirm_email_page, reset_password, reset_password_page},
    middleware::rate_limit::auth_rate_limit,
    state::AppState,
};

/// Pages opened from the links of the emails, at the root so the links stay short.
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/confirm-email",
            get(confirm_email_page).post(confirm_email),
        )
        .route(
            "/reset-password",
            get(reset_password_page).post(reset_password),
        )
        .route_layer(middleware::from_fn_with_state(state, auth_rate_limit))
}
