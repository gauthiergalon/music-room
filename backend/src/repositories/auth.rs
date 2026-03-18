use crate::errors::{AppError, ErrorMessage};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub async fn store_refresh_token(pool: &PgPool, token_hash: String, user_id: Uuid, expires_at: DateTime<Utc>) -> Result<(), AppError> {
	sqlx::query!("INSERT INTO refresh_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, $3)", token_hash, user_id, expires_at).execute(pool).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn delete_refresh_token(pool: &PgPool, token_hash: String, user_id: &Uuid) -> Result<(), AppError> {
	sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1 AND user_id = $2", token_hash, user_id).execute(pool).await.map_err(AppError::Database)?;
	Ok(())
}

pub struct StoredRefreshToken {
	pub user_id: Uuid,
	pub expires_at: DateTime<Utc>,
}

pub async fn delete_and_return_refresh_token(pool: &PgPool, token_hash: String) -> Result<Option<StoredRefreshToken>, AppError> {
	let stored = sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1 RETURNING user_id, expires_at", token_hash).fetch_optional(pool).await.map_err(AppError::Database)?;

	Ok(stored.map(|s| StoredRefreshToken { user_id: s.user_id, expires_at: s.expires_at }))
}

pub async fn find_user_id_by_email(pool: &PgPool, email: &str) -> Result<Option<Uuid>, AppError> {
	let user = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email).fetch_optional(pool).await.map_err(AppError::Database)?;
	Ok(user)
}

pub async fn insert_reset_token(pool: &PgPool, token_hash: String, user_id: Uuid, expires_at: DateTime<Utc>) -> Result<(), AppError> {
	sqlx::query!("INSERT INTO reset_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, $3)", token_hash, user_id, expires_at).execute(pool).await.map_err(AppError::Database)?;
	Ok(())
}

pub struct StoredResetToken {
	pub user_id: Uuid,
	pub expires_at: DateTime<Utc>,
}

pub async fn delete_and_return_reset_token<'a>(tx: &mut Transaction<'a, Postgres>, token_hash: String) -> Result<Option<StoredResetToken>, AppError> {
	let stored = sqlx::query!("DELETE FROM reset_tokens WHERE token_hash = $1 RETURNING user_id, expires_at", token_hash).fetch_optional(&mut **tx).await.map_err(AppError::Database)?;

	Ok(stored.map(|s| StoredResetToken { user_id: s.user_id, expires_at: s.expires_at }))
}

pub async fn update_user_password<'a>(tx: &mut Transaction<'a, Postgres>, user_id: Uuid, password_hash: String) -> Result<(), AppError> {
	sqlx::query!("UPDATE users SET password_hash = $1 WHERE id = $2", password_hash, user_id).execute(&mut **tx).await.map_err(AppError::Database)?;
	Ok(())
}

pub async fn insert_user(pool: &PgPool, username: &str, email: &str, password_hash: String) -> Result<Uuid, AppError> {
	let id = sqlx::query_scalar!("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id", username, email, password_hash).fetch_one(pool).await.map_err(|e| {
		if let sqlx::Error::Database(ref db_err) = e {
			if db_err.code().as_deref() == Some("23505") {
				let error_msg = db_err.message();
				if error_msg.contains("email") {
					return AppError::Conflict(ErrorMessage::EmailTaken);
				} else if error_msg.contains("username") {
					return AppError::Conflict(ErrorMessage::UsernameTaken);
				}
			}
		}
		AppError::Database(e)
	})?;
	Ok(id)
}

pub struct UserCredentials {
	pub id: Uuid,
	pub password_hash: Option<String>,
}

pub async fn find_user_credentials_by_email(pool: &PgPool, email: &str) -> Result<Option<UserCredentials>, AppError> {
	let user = sqlx::query!("SELECT id, password_hash FROM users WHERE email = $1", email).fetch_optional(pool).await.map_err(AppError::Database)?;

	Ok(user.map(|u| UserCredentials { id: u.id, password_hash: u.password_hash }))
}
