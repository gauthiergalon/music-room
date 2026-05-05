use crate::errors::AppError;
use sqlx::PgPool;
use uuid::Uuid;

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

use crate::models::invitation::Invitation;

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
) -> Result<Vec<Invitation>, AppError> {
    sqlx::query_as!(
        Invitation,
        r#"
        SELECT id, room_id, inviter_id, invitee_id, is_pending, created_at
        FROM invitations
        WHERE invitee_id = $1 AND is_pending = true
        ORDER BY created_at DESC
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
    .map_err(AppError::Database)
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
