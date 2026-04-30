use axum::{
    Extension, Json,
    extract::{Path, State},
};

use crate::{
    dtos::hifi::{SearchResponse, TrackResponse},
    errors::AppError,
    middleware::auth::Claims,
    services::hifi as hifi_service,
    state::AppState,
};

pub async fn search(
    State(_state): State<AppState>,
    Path(query): Path<String>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<SearchResponse>, AppError> {
    let tracks = hifi_service::search_tracks(&query).await?;

    Ok(Json(tracks))
}

pub async fn get_track(
    State(_state): State<AppState>,
    Path(track_id): Path<i64>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<TrackResponse>, AppError> {
    let track = hifi_service::get_track_details(track_id).await?;

    Ok(Json(track))
}
