use crate::{
    errors::{AppError, ErrorMessage},
    models::friend::Friend,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    user_id_1: Uuid,
    user_id_2: Uuid,
    sender_id: Uuid,
) -> Result<Friend, AppError> {
    let friend = sqlx::query_as!(
        Friend,
        r#"
        INSERT INTO friends (user_id_1, user_id_2, sender_id, is_pending)
        VALUES ($1, $2, $3, true)
        ON CONFLICT (user_id_1, user_id_2) DO UPDATE
        SET is_pending = false
        WHERE friends.is_pending = true AND friends.sender_id != EXCLUDED.sender_id
        RETURNING *
        "#,
        user_id_1,
        user_id_2,
        sender_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::Conflict(ErrorMessage::FriendAlreadyExists),
        _ => AppError::Database(e),
    })?;

    Ok(friend)
}

pub async fn update_accept(
    pool: &PgPool,
    user_id_1: Uuid,
    user_id_2: Uuid,
) -> Result<Friend, AppError> {
    let friend = sqlx::query_as!(
        Friend,
        r#"
        UPDATE friends
        SET is_pending = false
        WHERE user_id_1 = $1 AND user_id_2 = $2 AND is_pending = true
        RETURNING *
        "#,
        user_id_1,
        user_id_2
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::NotFound(ErrorMessage::FriendNotFound),
        _ => AppError::Database(e),
    })?;

    Ok(friend)
}

pub async fn delete_pending(
    pool: &PgPool,
    user_id_1: Uuid,
    user_id_2: Uuid,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM friends
        WHERE user_id_1 = $1 AND user_id_2 = $2 AND is_pending = true
        "#,
        user_id_1,
        user_id_2
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(ErrorMessage::FriendNotFound));
    }

    Ok(())
}

pub async fn delete(pool: &PgPool, user_id_1: Uuid, user_id_2: Uuid) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM friends
        WHERE user_id_1 = $1 AND user_id_2 = $2
        "#,
    )
    .bind(user_id_1)
    .bind(user_id_2)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(ErrorMessage::FriendNotFound));
    }

    Ok(())
}

pub async fn find_by_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Friend>, AppError> {
    let friends = sqlx::query_as!(
        Friend,
        r#"
        SELECT * FROM friends
        WHERE (user_id_1 = $1 OR user_id_2 = $1)
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(friends)
}

pub async fn find_by_users(
    pool: &PgPool,
    user_id_1: Uuid,
    user_id_2: Uuid,
) -> Result<Option<Friend>, AppError> {
    let friend = sqlx::query_as!(
        Friend,
        r#"
        SELECT * FROM friends
        WHERE user_id_1 = $1 AND user_id_2 = $2
        "#,
        user_id_1,
        user_id_2
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(friend)
}
