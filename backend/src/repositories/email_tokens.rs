use crate::{errors::AppError, models::email_token::EmailToken, models::email_token::NewEmailToken};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub async fn create<'c, E>(executor: E, token: NewEmailToken) -> Result<(), AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	sqlx::query!("INSERT INTO email_tokens (token_hash, user_id, new_email, expires_at) VALUES ($1, $2, $3, $4)", token.token_hash, token.user_id, token.new_email, token.expires_at).execute(executor).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn delete_and_return<'c, E>(executor: E, token_hash: String) -> Result<Option<EmailToken>, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let token = sqlx::query_as!(EmailToken, "DELETE FROM email_tokens WHERE token_hash = $1 RETURNING *", token_hash).fetch_optional(executor).await.map_err(AppError::Database)?;

	Ok(token)
}

pub async fn find_valid_by_user_id<'c, E>(executor: E, user_id: Uuid) -> Result<Option<EmailToken>, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let token = sqlx::query_as!(EmailToken, "SELECT * FROM email_tokens WHERE user_id = $1 AND expires_at > NOW() ORDER BY expires_at DESC LIMIT 1", user_id).fetch_optional(executor).await.map_err(AppError::Database)?;

	Ok(token)
}
