use crate::errors::AppError;
use crate::models::reset_token::{NewResetToken, ResetToken};
use sqlx::{Executor, Postgres};

pub async fn insert<'c, E>(executor: E, token: NewResetToken) -> Result<(), AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	sqlx::query!("INSERT INTO reset_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, $3)", token.token_hash, token.user_id, token.expires_at).execute(executor).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn delete_and_return<'c, E>(executor: E, token_hash: String) -> Result<Option<ResetToken>, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let stored = sqlx::query!("DELETE FROM reset_tokens WHERE token_hash = $1 RETURNING token_hash, user_id, created_at, expires_at", token_hash).fetch_optional(executor).await.map_err(AppError::Database)?;

	Ok(stored.map(|s| ResetToken { token_hash: s.token_hash, user_id: s.user_id, created_at: s.created_at, expires_at: s.expires_at }))
}
