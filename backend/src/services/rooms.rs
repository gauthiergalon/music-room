use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    models::room::Room,
    repositories::{rooms as rooms_repo, users as users_repo},
    services::{invitations as invitation_service, user as user_service},
    state::AppState,
};

pub async fn check_read_access(pool: &PgPool, room: &Room, user_id: Uuid) -> Result<(), AppError> {
    if room.is_public || room.owner_id == user_id {
        return Ok(());
    }

    let is_invited = invitation_service::check_accepted_invitation(pool, room.id, user_id).await?;
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

    let is_invited = invitation_service::check_accepted_invitation(pool, room.id, user_id).await?;
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

pub async fn get_room_for_owner_action(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<Room, AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;
    check_is_owner(&room, user_id)?;
    Ok(room)
}

/// Rooms the user may see, most recently active first.
pub async fn list(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Room>, AppError> {
    rooms_repo::find_visible(pool, user_id, limit, offset).await
}

/// The room, if the user is allowed to see it.
pub async fn get(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<Room, AppError> {
    let room = rooms_repo::find_by_id(pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    check_read_access(pool, &room, user_id).await?;

    Ok(room)
}

/// Creates the user's room. A user owns one room at a time, so any previous
/// one is closed first and its listeners are told.
pub async fn create(state: &AppState, owner_id: Uuid) -> Result<Room, AppError> {
    let owner = users_repo::find_by_id(&state.pool, owner_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

    for previous_room in rooms_repo::delete_by_owner(&state.pool, owner_id).await? {
        state.rooms.close(previous_room).await;
    }

    let name = format!("{}'s room", owner.username);
    rooms_repo::create(&state.pool, owner_id, &name).await
}

/// Deletes the room and tells every connected client. This is the only way a
/// room is removed, whoever decides it (owner, grace period, cleanup).
pub async fn close(state: &AppState, room_id: Uuid) -> Result<(), AppError> {
    rooms_repo::delete(&state.pool, room_id).await?;
    state.rooms.close(room_id).await;
    Ok(())
}

pub async fn delete(state: &AppState, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let _ = get_room_for_owner_action(&state.pool, room_id, user_id).await?;
    close(state, room_id).await
}

pub async fn delegate(
    state: &AppState,
    room_id: Uuid,
    owner_id: Uuid,
    target_user_id: Uuid,
    target_connection_id: Uuid,
) -> Result<(), AppError> {
    let _ = get_room_for_owner_action(&state.pool, room_id, owner_id).await?;

    if target_user_id == owner_id {
        return Err(AppError::Validation(vec![
            ErrorMessage::CannotDelegateToSelf,
        ]));
    }

    state
        .rooms
        .set_delegate(room_id, target_user_id, target_connection_id)
        .await
}

pub async fn revoke_delegate(
    state: &AppState,
    room_id: Uuid,
    owner_id: Uuid,
) -> Result<(), AppError> {
    let _ = get_room_for_owner_action(&state.pool, room_id, owner_id).await?;
    state.rooms.clear_delegate(room_id).await
}

pub async fn enable_license(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = get_room_for_owner_action(pool, room_id, user_id).await?;
    rooms_repo::update_visibility(pool, room_id, room.is_public, true).await
}

pub async fn disable_license(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = get_room_for_owner_action(pool, room_id, user_id).await?;
    rooms_repo::update_visibility(pool, room_id, room.is_public, false).await
}

pub async fn publish(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let room = get_room_for_owner_action(pool, room_id, user_id).await?;
    rooms_repo::update_visibility(pool, room_id, true, room.is_licensed).await
}

pub async fn privatize(pool: &PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let subscription_user = user_service::refresh_subscription_status(pool, user_id).await?;
    if !subscription_user.is_subscribed {
        return Err(AppError::Forbidden(ErrorMessage::SubscriptionRequired));
    }

    let room = get_room_for_owner_action(pool, room_id, user_id).await?;
    rooms_repo::update_visibility(pool, room_id, false, room.is_licensed).await
}
