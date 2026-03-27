use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, models::room::Room};

pub async fn find_all(pool: &PgPool) -> Result<Vec<Room>, AppError> {
	let rooms = sqlx::query_as!(Room, "SELECT id, owner_id, name, is_public, current_track, current_position, is_playing FROM rooms").fetch_all(pool).await.map_err(AppError::Database)?;
	Ok(rooms)
}

pub async fn create(pool: &PgPool, owner_id: Uuid, name: &str) -> Result<Room, AppError> {
	let room = sqlx::query_as!(Room, "INSERT INTO rooms (owner_id, name) VALUES ($1, $2) RETURNING id, owner_id, name, is_public, current_track, current_position, is_playing", owner_id, name).fetch_one(pool).await.map_err(AppError::Database)?;
	Ok(room)
}

pub async fn find_by_id(pool: &PgPool, room_id: Uuid) -> Result<Option<Room>, AppError> {
	let room = sqlx::query_as!(Room, "SELECT id, owner_id, name, is_public, current_track, current_position, is_playing FROM rooms WHERE id = $1", room_id).fetch_optional(pool).await.map_err(AppError::Database)?;
	Ok(room)
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
