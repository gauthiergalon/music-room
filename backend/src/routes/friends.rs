use axum::{
    Router, middleware,
    routing::{delete, get, post},
};

use crate::{handlers::friends, middleware::auth::auth_middleware, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(friends::list).post(friends::send_request))
        .route("/{friend_id}/accept", post(friends::accept_request))
        .route("/{friend_id}/reject", delete(friends::reject_request))
        .route("/{friend_id}", delete(friends::remove))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}
