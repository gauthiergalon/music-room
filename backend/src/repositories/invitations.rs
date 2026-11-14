use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage, FOREIGN_KEY_VIOLATION, UNIQUE_VIOLATION},
    models::invitation::{Invitation, InvitationWithDetails},
};

pub async fn exists_accepted(
    pool: &PgPool,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<bool, AppError> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM invitations 
            WHERE room_id = $1 AND invitee_id = $2 AND is_pending = false
        ) as "exists!"
        "#,
        room_id,
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(result.exists)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Invitation>, AppError> {
    sqlx::query_as!(
        Invitation,
        r#"
        SELECT id, room_id, inviter_id, invitee_id, is_pending, created_at
        FROM invitations
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn find_pending_by_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<InvitationWithDetails>, AppError> {
    sqlx::query_as!(
        InvitationWithDetails,
        r#"
        SELECT i.id, i.room_id, i.inviter_id, i.invitee_id, i.is_pending, i.created_at,
               u.username AS "inviter_username!", r.name AS "room_name!"
        FROM invitations i
        JOIN users u ON u.id = i.inviter_id
        JOIN rooms r ON r.id = i.room_id
        WHERE i.invitee_id = $1 AND i.is_pending = true
        ORDER BY i.created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn create(
    pool: &PgPool,
    room_id: Uuid,
    inviter_id: Uuid,
    invitee_id: Uuid,
) -> Result<Invitation, AppError> {
    sqlx::query_as!(
        Invitation,
        r#"
        INSERT INTO invitations (room_id, inviter_id, invitee_id, is_pending)
        VALUES ($1, $2, $3, true)
        RETURNING id, room_id, inviter_id, invitee_id, is_pending, created_at
        "#,
        room_id,
        inviter_id,
        invitee_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if AppError::is_db_violation(&e, UNIQUE_VIOLATION) {
            AppError::Conflict(ErrorMessage::AlreadyInvited)
        } else if AppError::is_db_violation(&e, FOREIGN_KEY_VIOLATION) {
            // The invitee (or the room) was deleted in the meantime.
            AppError::NotFound(ErrorMessage::UserNotFound)
        } else {
            AppError::Database(e)
        }
    })
}

pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    is_pending: bool,
) -> Result<Invitation, AppError> {
    sqlx::query_as!(
        Invitation,
        r#"
        UPDATE invitations
        SET is_pending = $2
        WHERE id = $1
        RETURNING id, room_id, inviter_id, invitee_id, is_pending, created_at
        "#,
        id,
        is_pending
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM invitations WHERE id = $1", id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(())
}
