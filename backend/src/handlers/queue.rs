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

pub async fn get_queue(State(state): State<AppState>, Path(room_id): Path<Uuid>) -> Result<Json<Vec<Queue>>, AppError> {
	let queues = queue_service::get_queue(&state.pool, room_id).await?;
	Ok(Json(queues))
}

pub async fn add_to_queue(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<AddToQueueRequest>) -> Result<StatusCode, AppError> {
	if payload.track_id <= 0 {
		return Err(AppError::Validation(vec![ErrorMessage::TrackIdInvalid]));
	}

	queue_service::add_to_queue(&state.pool, room_id, claims.user_id, payload.track_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_from_queue(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<RemoveFromQueueRequest>) -> Result<StatusCode, AppError> {
	queue_service::remove_from_queue(&state.pool, room_id, claims.user_id, payload.id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn reorder_queue(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<ReorderQueueRequest>) -> Result<StatusCode, AppError> {
	queue_service::reorder_queue(&state.pool, room_id, claims.user_id, payload.id, payload.new_position).await?;
	Ok(StatusCode::NO_CONTENT)
}
