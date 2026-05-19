use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    dtos::queue::{AddToQueueRequest, RemoveFromQueueRequest, ReorderQueueRequest},
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    models::queue::Queue,
    services::queue as queue_service,
    state::AppState,
    ws::send_room_state,
};

#[utoipa::path(get, path = "/rooms/{id}/queue", responses((status = 200, body = [Queue])), tag = "Queue")]
pub async fn list(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Queue>>, AppError> {
    let queues = queue_service::find_all_by_room_id(&state.pool, room_id, claims.user_id).await?;

    Ok(Json(queues))
}

#[utoipa::path(post, path = "/rooms/{id}/queue", request_body = AddToQueueRequest, responses((status = 204)), tag = "Queue")]
pub async fn add(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AddToQueueRequest>,
) -> Result<StatusCode, AppError> {
    if payload.track_id <= 0 {
        return Err(AppError::Validation(vec![ErrorMessage::TrackIdInvalid]));
    }

    // Best-effort cache warmup so RoomState can include full metadata.
    let _ = state.music_provider.get_track_info(&state.pool, payload.track_id).await;

    queue_service::create(&state.pool, room_id, claims.user_id, payload.track_id).await?;
    send_room_state(&state, room_id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/rooms/{id}/queue", request_body = RemoveFromQueueRequest, responses((status = 204)), tag = "Queue")]
pub async fn delete(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RemoveFromQueueRequest>,
) -> Result<StatusCode, AppError> {
    queue_service::remove(&state.pool, room_id, claims.user_id, payload.id).await?;
    send_room_state(&state, room_id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(patch, path = "/rooms/{id}/queue", request_body = ReorderQueueRequest, responses((status = 204)), tag = "Queue")]
pub async fn reorder(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ReorderQueueRequest>,
) -> Result<StatusCode, AppError> {
    queue_service::reorder(
        &state.pool,
        room_id,
        claims.user_id,
        payload.id,
        payload.new_position,
    )
    .await?;
    send_room_state(&state, room_id).await;
    Ok(StatusCode::NO_CONTENT)
}
