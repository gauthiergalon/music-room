use crate::errors::AppError;
use crate::models::refresh_token::{NewRefreshToken, RefreshToken};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub async fn insert<'c, E>(executor: E, token: NewRefreshToken) -> Result<(), AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	sqlx::query!("INSERT INTO refresh_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, $3)", token.token_hash, token.user_id, token.expires_at).execute(executor).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn delete<'c, E>(executor: E, token_hash: String, user_id: &Uuid) -> Result<(), AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1 AND user_id = $2", token_hash, user_id).execute(executor).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn delete_and_return<'c, E>(executor: E, token_hash: String) -> Result<Option<RefreshToken>, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let stored = sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1 RETURNING token_hash, user_id, created_at, expires_at", token_hash).fetch_optional(executor).await.map_err(AppError::Database)?;

	Ok(stored.map(|s| RefreshToken { token_hash: s.token_hash, user_id: s.user_id, created_at: s.created_at, expires_at: s.expires_at }))
}
