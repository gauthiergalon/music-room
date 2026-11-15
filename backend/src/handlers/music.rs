use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};

use crate::{
    dtos::music::{SearchResponse, StreamUrlResponse},
    errors::AppError,
    middleware::auth::Claims,
    services::stream_proxy::{SignedLink, StreamProxy},
    state::AppState,
};

#[utoipa::path(get, path = "/hifi/search/{query}", responses((status = 200, body = SearchResponse)), tag = "Hifi")]
pub async fn search(
    State(state): State<AppState>,
    Path(query): Path<String>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<SearchResponse>, AppError> {
    let tracks = state.music_provider.search_tracks(&query).await?;

    Ok(Json(tracks))
}

/// Where the client can load the audio of a track.
///
/// Native apps get the provider's URL. The web app gets a path on this API
/// (starting with `/`) when the stream is HLS, because the provider's CDN
/// refuses requests coming from a browser.
#[utoipa::path(get, path = "/hifi/track/{id}/stream-url", responses((status = 200, body = StreamUrlResponse), (status = 404, description = "Unknown track, or track not available for streaming"), (status = 502)), tag = "Hifi")]
pub async fn get_stream_url(
    State(state): State<AppState>,
    Path(track_id): Path<i64>,
    headers: HeaderMap,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<StreamUrlResponse>, AppError> {
    let platform = headers
        .get("x-platform")
        .and_then(|value| value.to_str().ok());
    let is_browser = platform.is_some_and(|platform| platform.eq_ignore_ascii_case("web"));

    let mut stream_url = state
        .music_provider
        .get_stream_url(track_id, platform)
        .await?;

    if is_browser && StreamProxy::is_hls(&stream_url) {
        stream_url = state.stream_proxy.playlist_path(&stream_url)?;
    }

    Ok(Json(StreamUrlResponse { stream_url }))
}

/// An HLS playlist of the provider, with its URLs rewritten to this relay.
#[utoipa::path(get, path = "/hifi/stream/playlist.m3u8", params(("u" = String, Query), ("exp" = i64, Query), ("sig" = String, Query)), responses((status = 200, content_type = "application/vnd.apple.mpegurl"), (status = 403), (status = 502)), tag = "Hifi")]
pub async fn stream_playlist(
    State(state): State<AppState>,
    Query(link): Query<SignedLink>,
) -> Result<Response, AppError> {
    let playlist = state.stream_proxy.playlist(&link).await?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/vnd.apple.mpegurl"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        playlist,
    )
        .into_response())
}

/// A media segment of the provider, fetched on behalf of the browser.
#[utoipa::path(get, path = "/hifi/stream/segment", params(("u" = String, Query), ("exp" = i64, Query), ("sig" = String, Query)), responses((status = 200), (status = 206), (status = 403), (status = 502)), tag = "Hifi")]
pub async fn stream_segment(
    State(state): State<AppState>,
    Query(link): Query<SignedLink>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let segment = state
        .stream_proxy
        .segment(&link, headers.get(header::RANGE))
        .await?;

    Ok((segment.status, segment.headers, segment.body).into_response())
}
