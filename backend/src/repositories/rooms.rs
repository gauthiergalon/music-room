use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	errors::AppError,
	models::room::{self, Room},
};

pub async fn create(pool: &PgPool, owner_id: Uuid) -> Result<Room, AppError> {
	let room = sqlx::query!("INSERT INTO rooms (owner_id) VALUES ($1) RETURNING id, owner_id, is_public, current_track, current_position, is_playing", owner_id).fetch_one(pool).await.map_err(AppError::Database)?;

	Ok(Room { id: room.id, owner_id: room.owner_id, is_public: room.is_public, current_track: room.current_track, current_position: room.current_position, is_playing: room.is_playing })
}

pub async fn find_by_id(pool: &PgPool, room_id: Uuid) -> Result<Option<Room>, AppError> {
	let room = sqlx::query!("SELECT id, owner_id, is_public, current_track, current_position, is_playing FROM rooms WHERE id = $1", room_id).fetch_optional(pool).await.map_err(AppError::Database)?;

	Ok(room.map(|r| Room { id: r.id, owner_id: r.owner_id, is_public: r.is_public, current_track: r.current_track, current_position: r.current_position, is_playing: r.is_playing }))
}

pub async fn delete(pool: &PgPool, room_id: Uuid) -> Result<(), AppError> {
	sqlx::query!("DELETE FROM rooms WHERE id = $1", room_id).execute(pool).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn update_ownership(pool: &PgPool, room_id: Uuid, new_owner_id: Uuid) -> Result<(), AppError> {
	sqlx::query!("UPDATE rooms SET owner_id = $1 WHERE id = $2", new_owner_id, room_id).execute(pool).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn update_visibility(pool: &PgPool, room_id: Uuid, is_public: bool) -> Result<(), AppError> {
	sqlx::query!("UPDATE rooms SET is_public = $1 WHERE id = $2", is_public, room_id).execute(pool).await.map_err(AppError::Database)?;
	Ok(())
}
