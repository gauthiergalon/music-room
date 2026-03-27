use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	dtos::rooms,
	errors::AppError,
	errors::ErrorMessage,
	models::queue::Queue,
	repositories::{queue as queue_repo, rooms as rooms_repo},
};

pub async fn get_queue(pool: &PgPool, room_id: Uuid) -> Result<Vec<Queue>, AppError> {
	queue_repo::get_queue(pool, room_id).await.map_err(AppError::Database)
}

pub async fn add_to_queue(pool: &PgPool, room_id: Uuid, user_id: Uuid, track_id: i64) -> Result<(), AppError> {
	let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

	if room.owner_id != user_id {
		return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
	}

	queue_repo::add_to_queue(pool, room_id, user_id, track_id).await.map_err(AppError::Database)
}

pub async fn remove_from_queue(pool: &PgPool, room_id: Uuid, _user_id: Uuid, queue_id: Uuid) -> Result<(), AppError> {
	let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

	if room.owner_id != _user_id {
		return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
	}

	queue_repo::remove_from_queue(pool, room_id, queue_id).await.map_err(AppError::Database)
}

pub async fn reorder_queue(pool: &PgPool, room_id: Uuid, _user_id: Uuid, queue_id: Uuid, new_position: f64) -> Result<(), AppError> {
	let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

	if room.owner_id != _user_id {
		return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
	}

	queue_repo::reorder_queue(pool, room_id, queue_id, new_position).await.map_err(AppError::Database)
}
