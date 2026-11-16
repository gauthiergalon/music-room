use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, models::room::Room};

/// Rooms the user is allowed to see: public ones, their own, and private ones
/// they accepted an invitation to.
pub async fn find_visible(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Room>, AppError> {
    let rooms = sqlx::query_as!(
        Room,
        r#"
        SELECT r.id, r.owner_id, r.name, r.is_public, r.is_licensed, r.current_track, r.current_position, r.is_playing
        FROM rooms r
        WHERE r.is_public
           OR r.owner_id = $1
           OR EXISTS (
               SELECT 1 FROM invitations i
               WHERE i.room_id = r.id AND i.invitee_id = $1 AND i.is_pending = false
           )
        ORDER BY r.last_active_at DESC, r.id
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;
    Ok(rooms)
}

pub async fn create(pool: &PgPool, owner_id: Uuid, name: &str) -> Result<Room, AppError> {
    let room = sqlx::query_as!(
        Room,
        "INSERT INTO rooms (owner_id, name) VALUES ($1, $2) RETURNING id, owner_id, name, is_public, is_licensed, current_track, current_position, is_playing",
        owner_id,
        name
    )
    .fetch_one(pool)
    .await?;
    Ok(room)
}

pub async fn find_by_id(pool: &PgPool, room_id: Uuid) -> Result<Option<Room>, AppError> {
    let room = sqlx::query_as!(
        Room,
        "SELECT id, owner_id, name, is_public, is_licensed, current_track, current_position, is_playing FROM rooms WHERE id = $1",
        room_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(room)
}

pub async fn delete(pool: &PgPool, room_id: Uuid) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM rooms WHERE id = $1", room_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Deletes every room of a user and returns their ids so connected clients can be notified.
pub async fn delete_by_owner(pool: &PgPool, owner_id: Uuid) -> Result<Vec<Uuid>, AppError> {
    let ids = sqlx::query_scalar!(
        "DELETE FROM rooms WHERE owner_id = $1 RETURNING id",
        owner_id
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
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
    .await?;
    Ok(())
}

/// Marks the room as recently used so the cleanup task leaves it alone.
pub async fn touch(pool: &PgPool, room_id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET last_active_at = now() WHERE id = $1",
        room_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Gives every room a fresh grace window, used when the server starts and no
/// client has had the chance to reconnect yet.
pub async fn touch_all(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query!("UPDATE rooms SET last_active_at = now()")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_inactive_since(
    pool: &PgPool,
    threshold: DateTime<Utc>,
) -> Result<Vec<Uuid>, AppError> {
    let ids = sqlx::query_scalar!("SELECT id FROM rooms WHERE last_active_at < $1", threshold)
        .fetch_all(pool)
        .await?;
    Ok(ids)
}

// Playback. `played_at` is always the server clock instant at which
// `current_position` was valid, whatever the state.

pub async fn update_playback_play(
    pool: &PgPool,
    room_id: Uuid,
    position: i32,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET is_playing = true, played_at = now(), current_position = $1 WHERE id = $2 AND current_track IS NOT NULL",
        position,
        room_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_playback_pause(
    pool: &PgPool,
    room_id: Uuid,
    position: i32,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET is_playing = false, played_at = now(), current_position = $1 WHERE id = $2",
        position,
        room_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_playback_seek(
    pool: &PgPool,
    room_id: Uuid,
    position: i32,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE rooms SET played_at = now(), current_position = $1 WHERE id = $2",
        position,
        room_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Moves the room to the next queued track, or clears the current track and
/// pauses when the queue is empty. Runs in one transaction with the room row
/// locked, so two concurrent calls cannot skip two tracks.
///
/// `expected_item` is the queue entry the caller believes is playing (`None` =
/// nothing playing). Entries are unique even when a track is queued several
/// times. Returns `false` and changes nothing when the room has moved on,
/// which makes a duplicated or late request a no-op.
pub async fn advance_track(
    pool: &PgPool,
    room_id: Uuid,
    expected_item: Option<Uuid>,
) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;

    let Some(current_item) = sqlx::query_scalar!(
        "SELECT current_item FROM rooms WHERE id = $1 FOR UPDATE",
        room_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(false);
    };

    if current_item != expected_item {
        return Ok(false);
    }

    let next = sqlx::query!(
        r#"
        DELETE FROM queue
        WHERE id = (SELECT id FROM queue WHERE room_id = $1 ORDER BY position ASC LIMIT 1)
        RETURNING id, track_id
        "#,
        room_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    match next {
        Some(next) => {
            sqlx::query!(
                "UPDATE rooms SET current_track = $1, current_item = $2, played_at = now(), current_position = 0, is_playing = true WHERE id = $3",
                next.track_id,
                next.id,
                room_id
            )
            .execute(&mut *tx)
            .await?;
        }
        None => {
            sqlx::query!(
                "UPDATE rooms SET current_track = NULL, current_item = NULL, played_at = now(), current_position = 0, is_playing = false WHERE id = $1",
                room_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(true)
}

pub struct RoomPlaybackState {
    pub current_track: Option<i64>,
    pub current_item: Option<Uuid>,
    pub is_playing: bool,
    pub current_position: i32,
    pub played_at: Option<DateTime<Utc>>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover: Option<String>,
    pub duration: Option<i32>,
}

/// Playback state of the room with the metadata of its current track.
pub async fn get_room_playback_state(
    pool: &PgPool,
    room_id: Uuid,
) -> Result<Option<RoomPlaybackState>, AppError> {
    let state = sqlx::query_as!(
        RoomPlaybackState,
        r#"
        SELECT r.current_track, r.current_item, r.is_playing, r.current_position, r.played_at,
               t.title AS "title?", t.artist AS "artist?", t.album AS "album?",
               t.cover AS "cover?", t.duration AS "duration?"
        FROM rooms r
        LEFT JOIN tracks t ON t.id = r.current_track
        WHERE r.id = $1
        "#,
        room_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(state)
}
