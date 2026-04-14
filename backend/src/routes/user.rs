use axum::{
    Router, middleware,
    routing::{get, patch, post},
};
use sqlx::PgPool;

use crate::{
    handlers::user::{
        confirm_email, get_me, get_user, send_email_confirmation_email, update_email,
        update_password, update_username,
    },
    middleware::auth::auth_middleware,
    state::AppState,
};

pub fn router(state: AppState) -> Router<AppState> {
    let protected = Router::new()
        .route("/{id}", get(get_user))
        .route("/me", get(get_me))
        .route("/me/username", patch(update_username))
        .route("/me/email", patch(update_email))
        .route("/me/password", patch(update_password))
        .route("/me/confirm-email", patch(confirm_email))
        .route(
            "/me/send-confirmation-email",
            post(send_email_confirmation_email),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    protected
}
