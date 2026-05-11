use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, models::room::Room};

pub async fn find_all(pool: &PgPool) -> Result<Vec<Room>, AppError> {
    let rooms = sqlx::query_as!(Room, "SELECT id, owner_id, name, is_public, is_licensed, current_track, current_position, is_playing FROM rooms").fetch_all(pool).await.map_err(AppError::Database)?;
    Ok(rooms)
}

pub async fn create(pool: &PgPool, owner_id: Uuid, name: &str) -> Result<Room, AppError> {
    let room = sqlx::query_as!(Room, "INSERT INTO rooms (owner_id, name) VALUES ($1, $2) RETURNING id, owner_id, name, is_public, is_licensed, current_track, current_position, is_playing", owner_id, name).fetch_one(pool).await.map_err(AppError::Database)?;
    Ok(room)
}

pub async fn find_by_id(pool: &PgPool, room_id: Uuid) -> Result<Option<Room>, AppError> {
    let room = sqlx::query_as!(Room, "SELECT id, owner_id, name, is_public, is_licensed, current_track, current_position, is_playing FROM rooms WHERE id = $1", room_id).fetch_optional(pool).await.map_err(AppError::Database)?;
    Ok(room)
}

pub async fn delete(pool: &PgPool, room_id: Uuid) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM rooms WHERE id = $1", room_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_ownership(
    pool: &PgPool,
    room_id: Uuid,
    new_owner_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET owner_id = $1 WHERE id = $2",
        new_owner_id,
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_visibility(
    pool: &PgPool,
    room_id: Uuid,
    is_public: bool,
    is_licensed: bool,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET is_public = $1, is_licensed = $2 WHERE id = $3",
        is_public,
        is_licensed,
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_owner_id(pool: &PgPool, room_id: Uuid) -> Result<Option<Uuid>, AppError> {
    let owner = sqlx::query!("SELECT owner_id FROM rooms WHERE id = $1", room_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(owner.map(|row| row.owner_id))
}

pub async fn update_playback_play(
    pool: &PgPool,
    room_id: Uuid,
    position: i32,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET is_playing = true, played_at = $1, current_position = $2 WHERE id = $3",
        timestamp,
        position,
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_playback_pause(
    pool: &PgPool,
    room_id: Uuid,
    position: i32,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET is_playing = false, current_position = $1 WHERE id = $2",
        position,
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_playback_seek(
    pool: &PgPool,
    room_id: Uuid,
    position: i32,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET current_position = $1, played_at = $2 WHERE id = $3",
        position,
        timestamp,
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_current_track_and_play(
    pool: &PgPool,
    room_id: Uuid,
    track_id: i64,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET current_track = $1, played_at = $2, current_position = 0, is_playing = true WHERE id = $3",
        track_id,
        timestamp,
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn clear_current_track_and_pause(pool: &PgPool, room_id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET is_playing = false, current_track = NULL WHERE id = $1",
        room_id
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub struct RoomPlaybackState {
    pub current_track: Option<i64>,
    pub is_playing: bool,
    pub current_position: i32,
    pub played_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_room_playback_state(
    pool: &sqlx::PgPool,
    room_id: Uuid,
) -> sqlx::Result<Option<RoomPlaybackState>> {
    sqlx::query_as!(
        RoomPlaybackState,
        "SELECT current_track, is_playing, current_position, played_at FROM rooms WHERE id = $1",
        room_id
    )
    .fetch_optional(pool)
    .await
}
