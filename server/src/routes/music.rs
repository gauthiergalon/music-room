use axum::{Router, middleware, routing::get};

use crate::{handlers::music, middleware::auth::auth_middleware, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    let authenticated = Router::new()
        .route("/search/{query}", get(music::search))
        .route("/track/{id}/stream-url", get(music::get_stream_url))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware));

    // A media element cannot send an Authorization header: these links are
    // handed out by `stream-url` above and carry their own signature.
    let relay = Router::new()
        .route("/stream/playlist.m3u8", get(music::stream_playlist))
        .route("/stream/segment", get(music::stream_segment));

    Router::new().merge(authenticated).merge(relay)
}
