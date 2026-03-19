use axum::{
	Extension, Json,
	extract::{Path, State},
	http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	dtos::rooms::{RoomResponse, TransferOwnershipRequest},
	errors::AppError,
	middleware::auth::Claims,
	state::AppState,
};

pub async fn create_room(State(state): State<AppState>, Extension(claims): Extension<Claims>) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
	todo!();
}

pub async fn delete_room(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	todo!();
}

pub async fn get_room(State(state): State<AppState>, Path(room_id): Path<Uuid>) -> Result<Json<RoomResponse>, AppError> {
	todo!();
}

pub async fn transfer_ownership(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<TransferOwnershipRequest>) -> Result<StatusCode, AppError> {
	todo!();
}

pub async fn publish(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	todo!();
}

pub async fn privatize(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	todo!();
}
