use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    dtos::rooms,
    errors::AppError,
    errors::ErrorMessage,
    models::queue::Queue,
    repositories::{queue as queue_repo, rooms as rooms_repo},
    services::rooms as room_service,
};

pub async fn find_all_by_room_id(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<Queue>, AppError> {
    let _room = room_service::get(pool, room_id, user_id).await?;

    queue_repo::find_all_by_room_id(pool, room_id)
        .await
        .map_err(AppError::Database)
}

pub async fn create(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
    track_id: i64,
) -> Result<(), AppError> {
    let room = room_service::get(pool, room_id, user_id).await?;
    room_service::check_edit_queue_access(pool, &room, user_id).await?;

    queue_repo::create(pool, room_id, track_id)
        .await
        .map_err(AppError::Database)
}

pub async fn remove(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
    queue_id: Uuid,
) -> Result<(), AppError> {
    let room = room_service::get(pool, room_id, user_id).await?;
    room_service::check_edit_queue_access(pool, &room, user_id).await?;

    queue_repo::remove(pool, room_id, queue_id)
        .await
        .map_err(AppError::Database)
}

pub async fn reorder(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
    queue_id: Uuid,
    new_position: f64,
) -> Result<(), AppError> {
    let room = room_service::get(pool, room_id, user_id).await?;
    room_service::check_edit_queue_access(pool, &room, user_id).await?;

    queue_repo::reorder(pool, room_id, queue_id, new_position)
        .await
        .map_err(AppError::Database)
}
