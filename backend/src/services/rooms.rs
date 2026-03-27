use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    models::room::Room,
    repositories::rooms as rooms_repo,
};

pub async fn list(pool: &PgPool) -> Result<Vec<Room>, AppError> {
    rooms_repo::find_all(pool).await
}

pub async fn get(pool: &PgPool, room_id: Uuid) -> Result<Room, AppError> {
    rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))
}

pub async fn create(pool: &PgPool, owner_id: Uuid, name: &str) -> Result<Room, AppError> {
    rooms_repo::create(pool, owner_id, name).await
}

pub async fn delete(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    if room.owner_id != user_id {
        return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
    }

    rooms_repo::delete(pool, room_id).await
}

pub async fn transfer_ownership(pool: &PgPool, room_id: Uuid, current_owner_id: Uuid, new_owner_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    if room.owner_id != current_owner_id {
        return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
    }

    rooms_repo::update_ownership(pool, room_id, new_owner_id).await
}

pub async fn publish(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    if room.owner_id != user_id {
        return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
    }

    rooms_repo::update_visibility(pool, room_id, true).await
}

pub async fn privatize(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id).await?.ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    if room.owner_id != user_id {
        return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
    }

    rooms_repo::update_visibility(pool, room_id, false).await
}
