use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::invitations, middleware::auth::auth_middleware, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/rooms/{id}/invite/{invitee_id}", post(invitations::invite))
        .route("/me/invitations", get(invitations::list_pending))
        .route("/me/invitations/{id}/accept", post(invitations::accept))
        .route("/me/invitations/{id}/reject", post(invitations::reject))
        .route("/invitations/{id}/revoke", post(invitations::revoke))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
}
