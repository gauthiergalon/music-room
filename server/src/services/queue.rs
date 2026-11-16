use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{queue::Queue, room::Room},
    repositories::{queue as queue_repo, rooms as rooms_repo},
    services::{rooms as room_service, tracks as track_service},
    state::AppState,
};

pub async fn verify_queue_edit_access(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<Room, AppError> {
    let room = room_service::get(pool, room_id, user_id).await?;
    room_service::check_edit_queue_access(pool, &room, user_id).await?;
    Ok(room)
}

pub async fn find_all_by_room_id(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<Queue>, AppError> {
    let _room = room_service::get(pool, room_id, user_id).await?;
    queue_repo::find_all_by_room_id(pool, room_id).await
}

/// Appends a track to the queue. If nothing is playing in the room, the track
/// starts right away.
pub async fn create(
    state: &AppState,
    room_id: Uuid,
    user_id: Uuid,
    track_id: i64,
) -> Result<(), AppError> {
    let _ = verify_queue_edit_access(&state.pool, room_id, user_id).await?;

    // Permissions first: only someone allowed to edit may trigger a provider call.
    track_service::ensure_cached(state, track_id).await?;
    // Refuse a track nobody will be able to play instead of letting the room
    // get stuck on it. The resolved URL is cached, so the listeners' requests
    // that follow do not pay for it again.
    state.music_provider.get_stream_url(track_id, None).await?;
    queue_repo::create(&state.pool, room_id, track_id).await?;

    // No-op unless the room has no current track.
    rooms_repo::advance_track(&state.pool, room_id, None).await?;
    Ok(())
}

pub async fn remove(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
    queue_id: Uuid,
) -> Result<(), AppError> {
    let _ = verify_queue_edit_access(pool, room_id, user_id).await?;
    queue_repo::remove(pool, room_id, queue_id).await
}

pub async fn reorder(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
    queue_id: Uuid,
    new_position: f64,
) -> Result<(), AppError> {
    let _ = verify_queue_edit_access(pool, room_id, user_id).await?;
    queue_repo::reorder(pool, room_id, queue_id, new_position).await
}
