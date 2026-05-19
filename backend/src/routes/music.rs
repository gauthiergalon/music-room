use axum::{Router, middleware, routing::get};

use crate::{handlers::music, middleware::auth::auth_middleware, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/search/{query}", get(music::search))
        .route("/track/{id}", get(music::get_track))
        .route("/track/{id}/stream-url", get(music::get_stream_url))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}
