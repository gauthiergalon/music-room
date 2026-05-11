use axum::{
    Extension, Json,
    extract::{Path, State},
};

use crate::{
    dtos::hifi::{SearchResponse, StreamUrlResponse, TrackResponse},
    errors::AppError,
    middleware::auth::Claims,
    services::hifi as hifi_service,
    state::AppState,
};

#[utoipa::path(get, path = "/hifi/search/{query}", responses((status = 200, body = SearchResponse)), tag = "Hifi")]
pub async fn search(
    State(_state): State<AppState>,
    Path(query): Path<String>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<SearchResponse>, AppError> {
    let tracks = hifi_service::search_tracks(&query).await?;

    Ok(Json(tracks))
}

#[utoipa::path(get, path = "/hifi/track/{id}", responses((status = 200, body = TrackResponse)), tag = "Hifi")]
pub async fn get_track(
    State(_state): State<AppState>,
    Path(track_id): Path<i64>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<TrackResponse>, AppError> {
    let track = hifi_service::get_track_details(track_id).await?;

    Ok(Json(track))
}

#[utoipa::path(get, path = "/hifi/track/{id}/stream-url", responses((status = 200, body = StreamUrlResponse)), tag = "Hifi")]
pub async fn get_stream_url(
    State(_state): State<AppState>,
    Path(track_id): Path<i64>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<StreamUrlResponse>, AppError> {
    let stream_url = hifi_service::get_stream_url(track_id).await?;

    Ok(Json(StreamUrlResponse { stream_url }))
}
