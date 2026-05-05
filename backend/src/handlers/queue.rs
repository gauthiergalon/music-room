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
};

pub async fn list(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Queue>>, AppError> {
    let queues = queue_service::find_all_by_room_id(&state.pool, room_id, claims.user_id).await?;

    Ok(Json(queues))
}

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
    let _ = crate::services::hifi::get_track_info(&state.pool, payload.track_id).await;

    queue_service::create(&state.pool, room_id, claims.user_id, payload.track_id).await?;
    crate::handlers::rooms::broadcast_room_state(&state, room_id).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RemoveFromQueueRequest>,
) -> Result<StatusCode, AppError> {
    queue_service::remove(&state.pool, room_id, claims.user_id, payload.id).await?;
    crate::handlers::rooms::broadcast_room_state(&state, room_id).await;
    Ok(StatusCode::NO_CONTENT)
}

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
    crate::handlers::rooms::broadcast_room_state(&state, room_id).await;
    Ok(StatusCode::NO_CONTENT)
}
