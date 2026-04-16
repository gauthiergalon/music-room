use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    models::room::Room,
    repositories::rooms as rooms_repo,
    services::invitations as invitation_service,
};

pub async fn check_read_access(pool: &PgPool, room: &Room, user_id: Uuid) -> Result<(), AppError> {
    if room.is_public || room.owner_id == user_id {
        return Ok(());
    }

    let is_invited =
        invitation_service::check_accepted_invitation(pool, room.id, user_id).await?;
    if !is_invited {
        return Err(AppError::Forbidden(ErrorMessage::NotInvited));
    }

    Ok(())
}

pub async fn check_edit_queue_access(
    pool: &PgPool,
    room: &Room,
    user_id: Uuid,
) -> Result<(), AppError> {
    check_read_access(pool, room, user_id).await?;

    if !room.is_licensed || room.owner_id == user_id {
        return Ok(());
    }

    let is_invited =
        invitation_service::check_accepted_invitation(pool, room.id, user_id).await?;
    if !is_invited {
        return Err(AppError::Forbidden(ErrorMessage::MissingLicense));
    }

    Ok(())
}

pub fn check_is_owner(room: &Room, user_id: Uuid) -> Result<(), AppError> {
    if room.owner_id != user_id {
        return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
    }
    Ok(())
}

pub async fn list(pool: &PgPool) -> Result<Vec<Room>, AppError> {
    rooms_repo::find_all(pool).await
}

pub async fn get(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<Room, AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_read_access(pool, &room, user_id).await?;

    Ok(room)
}

pub async fn create(pool: &PgPool, owner_id: Uuid, name: &str) -> Result<Room, AppError> {
    rooms_repo::create(pool, owner_id, name).await
}

pub async fn delete(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_is_owner(&room, user_id)?;

    rooms_repo::delete(pool, room_id).await
}

pub async fn transfer_ownership(
    pool: &PgPool,
    room_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_is_owner(&room, current_owner_id)?;

    rooms_repo::update_ownership(pool, room_id, new_owner_id).await
}

pub async fn enable_license(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_is_owner(&room, user_id)?;

    rooms_repo::update_visibility(pool, room_id, room.is_public, true).await
}

pub async fn disable_license(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_is_owner(&room, user_id)?;

    rooms_repo::update_visibility(pool, room_id, room.is_public, false).await
}

pub async fn publish(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_is_owner(&room, user_id)?;

    rooms_repo::update_visibility(pool, room_id, true, room.is_licensed).await
}

pub async fn privatize(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_is_owner(&room, user_id)?;

    rooms_repo::update_visibility(pool, room_id, false, room.is_licensed).await
}
