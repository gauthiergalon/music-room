use crate::errors::AppError;
use crate::models::reset_token::{NewResetToken, ResetToken};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub async fn insert(pool: &PgPool, token: NewResetToken) -> Result<(), AppError> {
    sqlx::query!(
        "INSERT INTO reset_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, $3)",
        token.token_hash,
        token.user_id,
        token.expires_at
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_and_return<'a>(
    tx: &mut Transaction<'a, Postgres>,
    token_hash: String,
) -> Result<Option<ResetToken>, AppError> {
    let stored = sqlx::query!(
        "DELETE FROM reset_tokens WHERE token_hash = $1 RETURNING token_hash, user_id, created_at, expires_at",
        token_hash
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(AppError::Database)?;

    Ok(stored.map(|s| ResetToken {
        token_hash: s.token_hash,
        user_id: s.user_id,
        created_at: s.created_at,
        expires_at: s.expires_at,
    }))
}
