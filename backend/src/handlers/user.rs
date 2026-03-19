use axum::{
	Extension, Json,
	extract::{Path, State},
	http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	dtos::user::{PublicUserResponse, UpdateEmailRequest, UpdatePasswordRequest, UpdateUsernameRequest, UserResponse},
	errors::{AppError, ErrorMessage},
	middleware::auth::Claims,
	repositories::users,
	services::auth::{hash_password, verify_password},
	state::AppState,
};

pub async fn get_me(State(state): State<AppState>, Extension(claims): Extension<Claims>) -> Result<Json<UserResponse>, AppError> {
	let user = users::find_by_id(&state.pool, claims.user_id).await?.ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

	Ok(Json(UserResponse { id: user.id, username: user.username, email: user.email }))
}

pub async fn get_user(State(state): State<AppState>, Path(user_id): Path<Uuid>) -> Result<Json<PublicUserResponse>, AppError> {
	let user = users::find_by_id(&state.pool, user_id).await?.ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

	Ok(Json(PublicUserResponse { id: user.id, username: user.username }))
}

pub async fn update_username(State(state): State<AppState>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdateUsernameRequest>) -> Result<Json<UserResponse>, AppError> {
	if payload.username.len() < 3 || payload.username.len() > 24 {
		return Err(AppError::Validation(vec![ErrorMessage::UsernameInvalidLength]));
	}

	let user = users::update_username(&state.pool, claims.user_id, &payload.username).await?;

	Ok(Json(UserResponse { id: user.id, username: user.username, email: user.email }))
}

pub async fn update_email(Extension(claims): Extension<Claims>, State(state): State<AppState>, Json(payload): Json<UpdateEmailRequest>) -> Result<Json<UserResponse>, AppError> {
	if !validator::ValidateEmail::validate_email(&payload.new_email) {
		return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
	}

	// todo!("Send confirmation email to new address");

	let user = users::update_email(&state.pool, claims.user_id, &payload.new_email).await?;

	Ok(Json(UserResponse { id: user.id, username: user.username, email: user.email }))
}

pub async fn update_password(State(state): State<AppState>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdatePasswordRequest>) -> Result<StatusCode, AppError> {
	if payload.new_password.len() < 8 {
		return Err(AppError::Validation(vec![ErrorMessage::PasswordInvalidPolicy]));
	}

	let user = users::find_by_id(&state.pool, claims.user_id).await?.ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;
	let current_hash: String = user.password_hash.ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

	verify_password(&payload.current_password, &current_hash)?;
	let new_password_hash = hash_password(&payload.new_password)?;

	users::update_password(&state.pool, claims.user_id, new_password_hash).await?;

	Ok(StatusCode::NO_CONTENT)
}
