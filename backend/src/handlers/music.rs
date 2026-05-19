use axum::{
    Extension, Json,
    extract::{Path, State},
};

use crate::{
    dtos::music::{SearchResponse, StreamUrlResponse, TrackResponse},
    errors::AppError,
    middleware::auth::Claims,
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

#[utoipa::path(get, path = "/hifi/track/{id}", responses((status = 200, body = TrackResponse)), tag = "Hifi")]
pub async fn get_track(
    State(state): State<AppState>,
    Path(track_id): Path<i64>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<TrackResponse>, AppError> {
    let track = state.music_provider.get_track_details(track_id).await?;

    Ok(Json(track))
}

#[utoipa::path(get, path = "/hifi/track/{id}/stream-url", responses((status = 200, body = StreamUrlResponse)), tag = "Hifi")]
pub async fn get_stream_url(
    State(state): State<AppState>,
    Path(track_id): Path<i64>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<StreamUrlResponse>, AppError> {
    let stream_url = state.music_provider.get_stream_url(track_id).await?;

    Ok(Json(StreamUrlResponse { stream_url }))
}
