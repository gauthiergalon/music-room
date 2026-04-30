use crate::{handlers::hifi, middleware::auth::auth_middleware, state::AppState};
use axum::{Router, middleware, routing::get};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/search/{query}", get(hifi::search))
        .route("/track/{id}", get(hifi::get_track))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}
