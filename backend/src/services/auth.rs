use crate::errors::{AppError, ErrorMessage};
use argon2::{
	Argon2,
	password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{TimeDelta, Utc};
use hex;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub fn hash_token(token: &str) -> String {
	let mut hasher = Sha256::new();
	hasher.update(token.as_bytes());
	hex::encode(hasher.finalize())
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
	let salt = SaltString::generate(&mut OsRng);
	Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
	let parsed = PasswordHash::new(hash).map_err(|_| AppError::Internal)?;
	Argon2::default().verify_password(password.as_bytes(), &parsed).map_err(|_| AppError::Unauthorized(ErrorMessage::InvalidCredentials))
}

pub async fn store_refresh_token(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
	let token = Uuid::new_v4().to_string();
	let token_hash = hash_token(&token);
	let expires_at = Utc::now() + TimeDelta::days(7);

	sqlx::query!(
		"INSERT INTO refresh_tokens (token_hash, user_id, expires_at)
         VALUES ($1, $2, $3)",
		token_hash,
		user_id,
		expires_at
	)
	.execute(pool)
	.await
	.map_err(AppError::Database)?;

	Ok(token)
}

pub async fn delete_refresh_token(pool: &PgPool, token: &str, user_id: &Uuid) -> Result<(), AppError> {
	let token_hash = hash_token(token);
	sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1 AND user_id = $2", token_hash, user_id).execute(pool).await?;
	Ok(())
}

pub async fn rotate_refresh_token(pool: &PgPool, token: &str) -> Result<Uuid, AppError> {
	let token_hash = hash_token(token);
	let stored = sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1 RETURNING user_id, expires_at", token_hash).fetch_optional(pool).await?.ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	if stored.expires_at < Utc::now() {
		return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
	}

	Ok(stored.user_id)
}

pub async fn create_reset_token(pool: &PgPool, email: &str) -> Result<(), AppError> {
	let email = email.trim().to_lowercase();
	let user = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email).fetch_optional(pool).await?;

	if let Some(user_id) = user {
		let token = Uuid::new_v4().to_string();
		let token_hash = hash_token(&token);
		let expires_at = Utc::now() + TimeDelta::minutes(15);
		sqlx::query!("INSERT INTO reset_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, $3)", token_hash, user_id, expires_at).execute(pool).await?;

		todo!("Send email with reset link containing the token: {}", token);
	}

	Ok(())
}

pub async fn update_password_with_token(pool: &PgPool, token: &str, new_password: &str) -> Result<(), AppError> {
	let token_hash = hash_token(token);
	let password_hash = hash_password(new_password)?;

	let mut tx = pool.begin().await?;

	let stored = sqlx::query!("DELETE FROM reset_tokens WHERE token_hash = $1 RETURNING user_id, expires_at", token_hash).fetch_optional(&mut *tx).await?.ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	if stored.expires_at < Utc::now() {
		return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
	}

	sqlx::query!("UPDATE users SET password_hash = $1 WHERE id = $2", password_hash, stored.user_id).execute(&mut *tx).await?;

	tx.commit().await?;

	Ok(())
}

pub async fn create_user(pool: &PgPool, username: &str, email: &str, password: &str) -> Result<Uuid, AppError> {
	let username = username.trim();
	let email = email.trim().to_lowercase();
	let password_hash = hash_password(password)?;

	sqlx::query_scalar!("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id", username, email, password_hash).fetch_one(pool).await.map_err(|e| {
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
	})
}

pub async fn authenticate_user(pool: &PgPool, email: &str, password: &str) -> Result<Uuid, AppError> {
	let email = email.trim().to_lowercase();
	let user = sqlx::query!("SELECT id, password_hash FROM users WHERE email = $1", email).fetch_optional(pool).await?.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

	let password_hash = user.password_hash.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;
	verify_password(password, &password_hash)?;

	Ok(user.id)
}
