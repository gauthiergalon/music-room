use crate::errors::{AppError, ErrorMessage};
use crate::repositories::auth as auth_repo;
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

	auth_repo::store_refresh_token(pool, token_hash, user_id, expires_at).await?;

	Ok(token)
}

pub async fn delete_refresh_token(pool: &PgPool, token: &str, user_id: &Uuid) -> Result<(), AppError> {
	let token_hash = hash_token(token);
	auth_repo::delete_refresh_token(pool, token_hash, user_id).await?;
	Ok(())
}

pub async fn rotate_refresh_token(pool: &PgPool, token: &str) -> Result<Uuid, AppError> {
	let token_hash = hash_token(token);
	let stored = auth_repo::delete_and_return_refresh_token(pool, token_hash).await?.ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	if stored.expires_at < Utc::now() {
		return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
	}

	Ok(stored.user_id)
}

pub async fn create_reset_token(pool: &PgPool, email: &str) -> Result<(), AppError> {
	let email = email.trim().to_lowercase();
	let user = auth_repo::find_user_id_by_email(pool, &email).await?;

	if let Some(user_id) = user {
		let token = Uuid::new_v4().to_string();
		let token_hash = hash_token(&token);
		let expires_at = Utc::now() + TimeDelta::minutes(15);

		auth_repo::insert_reset_token(pool, token_hash, user_id, expires_at).await?;

		todo!("Send email with reset link containing the token: {}", token);
	}

	Ok(())
}

pub async fn update_password_with_token(pool: &PgPool, token: &str, new_password: &str) -> Result<(), AppError> {
	let token_hash = hash_token(token);
	let password_hash = hash_password(new_password)?;

	let mut tx = pool.begin().await.map_err(AppError::Database)?;

	let stored = auth_repo::delete_and_return_reset_token(&mut tx, token_hash).await?.ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	if stored.expires_at < Utc::now() {
		return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
	}

	auth_repo::update_user_password(&mut tx, stored.user_id, password_hash).await?;

	tx.commit().await.map_err(AppError::Database)?;

	Ok(())
}

pub async fn create_user(pool: &PgPool, username: &str, email: &str, password: &str) -> Result<Uuid, AppError> {
	let username = username.trim();
	let email = email.trim().to_lowercase();
	let password_hash = hash_password(password)?;

	auth_repo::insert_user(pool, username, &email, password_hash).await
}

pub async fn authenticate_user(pool: &PgPool, email: &str, password: &str) -> Result<Uuid, AppError> {
	let email = email.trim().to_lowercase();
	let user = auth_repo::find_user_credentials_by_email(pool, &email).await?.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

	let password_hash = user.password_hash.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;
	verify_password(password, &password_hash)?;

	Ok(user.id)
}
