use crate::errors::AppError;
use crate::errors::ErrorMessage;
use crate::models::invitation::Invitation;
use crate::repositories::invitations as invitation_repo;
use crate::services::rooms as room_service;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn check_accepted_invitation(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<bool, AppError> {
    invitation_repo::exists_accepted(pool, room_id, user_id).await
}

pub async fn invite(
    pool: &PgPool,
    room_id: Uuid,
    inviter_id: Uuid,
    invitee_id: Uuid,
) -> Result<Invitation, AppError> {
    let room = room_service::get(pool, room_id, inviter_id).await?;
    if room.owner_id != inviter_id {
        return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
    }

    invitation_repo::create(pool, room_id, inviter_id, invitee_id).await
}

pub async fn get_my_pending_invitations(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Invitation>, AppError> {
    invitation_repo::find_pending_by_user(pool, user_id).await
}

pub async fn accept(
    pool: &PgPool,
    invitation_id: Uuid,
    user_id: Uuid,
) -> Result<Invitation, AppError> {
    let invitation = invitation_repo::find_by_id(pool, invitation_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::InvitationNotFound))?;

    if invitation.invitee_id != user_id {
        return Err(AppError::Forbidden(ErrorMessage::NotInvitedUser));
    }

    invitation_repo::update_status(pool, invitation_id, false).await
}

pub async fn reject(pool: &PgPool, invitation_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let invitation = invitation_repo::find_by_id(pool, invitation_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::InvitationNotFound))?;

    if invitation.invitee_id != user_id {
        return Err(AppError::Forbidden(ErrorMessage::NotInvitedUser));
    }

    invitation_repo::delete(pool, invitation_id).await
}

pub async fn revoke(pool: &PgPool, invitation_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let invitation = invitation_repo::find_by_id(pool, invitation_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::InvitationNotFound))?;

    if invitation.inviter_id != user_id {
        let room = room_service::get(pool, invitation.room_id, user_id).await?;
        if room.owner_id != user_id {
            return Err(AppError::Forbidden(ErrorMessage::NotRoomOwner));
        }
    }

    invitation_repo::delete(pool, invitation_id).await
}
