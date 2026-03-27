use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	dtos::rooms,
	errors::AppError,
	errors::ErrorMessage,
	models::queue::Queue,
	repositories::{queue as queue_repo, rooms as rooms_repo},
};

pub async fn find_all_by_room_id(pool: &PgPool, room_id: Uuid) -> Result<Vec<Queue>, AppError> {
	queue_repo::find_all_by_room_id(pool, room_id).await.map_err(AppError::Database)
}

pub async fn create(pool: &PgPool, room_id: Uuid, user_id: Uuid, track_id: i64) -> Result<(), AppError> {
	let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

	if room.owner_id != user_id {
		return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
	}

	queue_repo::create(pool, room_id, track_id).await.map_err(AppError::Database)
}

pub async fn remove(pool: &PgPool, room_id: Uuid, _user_id: Uuid, queue_id: Uuid) -> Result<(), AppError> {
	let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

	if room.owner_id != _user_id {
		return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
	}

	queue_repo::remove(pool, room_id, queue_id).await.map_err(AppError::Database)
}

pub async fn reorder(pool: &PgPool, room_id: Uuid, _user_id: Uuid, queue_id: Uuid, new_position: f64) -> Result<(), AppError> {
	let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

	if room.owner_id != _user_id {
		return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
	}

	queue_repo::reorder(pool, room_id, queue_id, new_position).await.map_err(AppError::Database)
}
