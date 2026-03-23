use crate::{
	errors::{AppError, ErrorMessage},
	middleware::auth::Claims,
	models::refresh_token::NewRefreshToken,
	models::reset_token::NewResetToken,
	models::user::NewUser,
	repositories::{refresh_tokens, reset_tokens, users},
	services::tokens::TokenPair,
};
use argon2::{
	Argon2,
	password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{TimeDelta, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::PgPool;

use uuid::Uuid;

pub fn hash_password(password: &str) -> Result<String, AppError> {
	let salt = SaltString::generate(&mut OsRng);
	Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
	let parsed = PasswordHash::new(hash).map_err(|_| AppError::Internal)?;
	Argon2::default().verify_password(password.as_bytes(), &parsed).map_err(|_| AppError::Unauthorized(ErrorMessage::InvalidCredentials))
}

fn generate_access_token(user_id: Uuid, secret: &str) -> Result<String, AppError> {
	let exp = (Utc::now() + TimeDelta::minutes(15)).timestamp() as usize;
	let claims = Claims { user_id, exp };

	encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())).map_err(|_| AppError::Internal)
}

async fn store_refresh_token(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
	let token_pair = TokenPair::generate();
	let expires_at = Utc::now() + TimeDelta::days(7);

	refresh_tokens::create(pool, NewRefreshToken { token_hash: token_pair.hash, user_id, expires_at }).await?;

	Ok(token_pair.plain)
}

pub async fn register(pool: &PgPool, jwt_secret: &str, username: &str, email: &str, password: &str) -> Result<(String, String), AppError> {
	let username = username.trim();
	let email = email.trim().to_lowercase();
	let password_hash = hash_password(password)?;

	let user_id = users::create(pool, NewUser { username, email: &email, password_hash: Some(password_hash), google_id: None }).await?;

	let access_token = generate_access_token(user_id, jwt_secret)?;
	let refresh_token = store_refresh_token(pool, user_id).await?;

	Ok((access_token, refresh_token))
}

pub async fn login(pool: &PgPool, jwt_secret: &str, email: &str, password: &str) -> Result<(String, String), AppError> {
	let email = email.trim().to_lowercase();
	let user = users::find_by_email(pool, &email).await?.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

	let password_hash = user.password_hash.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;
	verify_password(password, &password_hash)?;

	let access_token = generate_access_token(user.id, jwt_secret)?;
	let refresh_token = store_refresh_token(pool, user.id).await?;

	Ok((access_token, refresh_token))
}

pub async fn logout(pool: &PgPool, token: &str, user_id: &Uuid) -> Result<(), AppError> {
	let token_hash = TokenPair::hash(token);
	refresh_tokens::delete(pool, token_hash, user_id).await?;
	Ok(())
}

pub async fn refresh(pool: &PgPool, jwt_secret: &str, token: &str) -> Result<(String, String), AppError> {
	let token_hash = TokenPair::hash(token);
	let stored = refresh_tokens::delete_and_return(pool, token_hash).await?.ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	if stored.expires_at < Utc::now() {
		return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
	}

	let access_token = generate_access_token(stored.user_id, jwt_secret)?;
	let refresh_token = store_refresh_token(pool, stored.user_id).await?;

	Ok((access_token, refresh_token))
}

pub async fn forgot_password(pool: &PgPool, email: &str) -> Result<(), AppError> {
	let email_address = email.trim().to_lowercase();
	let user = users::find_by_email(pool, &email_address).await?;

	if let Some(user_model) = user {
		let token_pair = crate::services::tokens::TokenPair::generate();
		let expires_at = Utc::now() + TimeDelta::minutes(15);

		reset_tokens::create(pool, NewResetToken { token_hash: token_pair.hash, user_id: user_model.id, expires_at }).await?;

		let email_to_send = crate::services::email::Email::for_password_reset(&token_pair.plain);
		email_to_send.send(&email_address)?;
	}

	Ok(())
}

pub async fn reset_password(pool: &PgPool, token: &str, new_password: &str) -> Result<(), AppError> {
	let token_hash = TokenPair::hash(token);
	let password_hash = hash_password(new_password)?;

	let mut tx = pool.begin().await.map_err(AppError::Database)?;

	let stored = reset_tokens::delete_and_return(&mut *tx, token_hash).await?.ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	if stored.expires_at < Utc::now() {
		return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
	}

	users::update_password(&mut *tx, stored.user_id, password_hash).await?;

	tx.commit().await.map_err(AppError::Database)?;

	Ok(())
}
