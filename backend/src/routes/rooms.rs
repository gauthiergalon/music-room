use crate::{
	errors::AppError,
	middleware::auth::{Claims, auth_middleware},
};
use axum::{
	Extension, Json, Router,
	extract::{Path, State},
	http::StatusCode,
	middleware,
	routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

pub fn router() -> Router<PgPool> {
	let protected = Router::new().route("/", post(create_room)).route("/{id}", get(get_room).delete(delete_room)).route("/{id}/transfer-ownership", post(transfer_ownership)).route("/{id}/publish", post(publish)).route("/{id}/privatize", post(privatize)).layer(middleware::from_fn(auth_middleware));

	protected
}

#[derive(Serialize)]
struct RoomResponse {
	id: Uuid,
	owner_id: Uuid,
}

#[derive(Deserialize)]
struct TransferOwnershipRequest {
	new_owner_id: Uuid,
}

async fn create_room(State(pool): State<PgPool>, Extension(claims): Extension<Claims>) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
	todo!();
}

async fn delete_room(State(pool): State<PgPool>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	todo!();
}

async fn get_room(State(pool): State<PgPool>, Path(room_id): Path<Uuid>) -> Result<Json<RoomResponse>, AppError> {
	todo!();
}

async fn transfer_ownership(State(pool): State<PgPool>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<TransferOwnershipRequest>) -> Result<StatusCode, AppError> {
	todo!();
}

async fn publish(State(pool): State<PgPool>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	todo!();
}

async fn privatize(State(pool): State<PgPool>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	todo!();
}
