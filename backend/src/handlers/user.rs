use axum::{
	Extension, Json,
	extract::{Path, Query, State},
	http::StatusCode,
};
use uuid::Uuid;

use crate::{
	dtos::user::{ConfirmEmailQuery, PublicUserResponse, UpdateEmailRequest, UpdatePasswordRequest, UpdateUsernameRequest, UserResponse},
	errors::{AppError, ErrorMessage},
	middleware::auth::Claims,
	services::user as user_service,
	state::AppState,
};

pub async fn get_me(State(state): State<AppState>, Extension(claims): Extension<Claims>) -> Result<Json<UserResponse>, AppError> {
	let user = user_service::get_me(&state.pool, claims.user_id).await?;

	Ok(Json(UserResponse { id: user.id, username: user.username, email: user.email }))
}

pub async fn get_user(State(state): State<AppState>, Path(user_id): Path<Uuid>) -> Result<Json<PublicUserResponse>, AppError> {
	let user = user_service::get_user(&state.pool, user_id).await?;

	Ok(Json(PublicUserResponse { id: user.id, username: user.username }))
}

pub async fn update_username(State(state): State<AppState>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdateUsernameRequest>) -> Result<Json<UserResponse>, AppError> {
	if payload.username.len() < 3 || payload.username.len() > 24 {
		return Err(AppError::Validation(vec![ErrorMessage::UsernameInvalidLength]));
	}

	let user = user_service::update_username(&state.pool, claims.user_id, &payload.username).await?;

	Ok(Json(UserResponse { id: user.id, username: user.username, email: user.email }))
}

pub async fn update_email(Extension(claims): Extension<Claims>, State(state): State<AppState>, Json(payload): Json<UpdateEmailRequest>) -> Result<Json<UserResponse>, AppError> {
	if !validator::ValidateEmail::validate_email(&payload.new_email) {
		return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
	}

	let user = user_service::update_email(&state.pool, claims.user_id, &payload.new_email).await?;

	Ok(Json(UserResponse { id: user.id, username: user.username, email: user.email }))
}

pub async fn update_password(State(state): State<AppState>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdatePasswordRequest>) -> Result<StatusCode, AppError> {
	if payload.new_password.len() < 8 {
		return Err(AppError::Validation(vec![ErrorMessage::PasswordInvalidPolicy]));
	}

	user_service::update_password(&state.pool, claims.user_id, &payload.current_password, &payload.new_password).await?;

	Ok(StatusCode::NO_CONTENT)
}

pub async fn confirm_email(State(state): State<AppState>, Query(query): Query<ConfirmEmailQuery>) -> Result<StatusCode, AppError> {
	user_service::confirm_email(&state.pool, &query.token).await?;

	Ok(StatusCode::NO_CONTENT)
}

pub async fn send_email_confirmation_email(State(state): State<AppState>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	user_service::send_email_confirmation_email(&state.pool, claims.user_id).await?;

	Ok(StatusCode::NO_CONTENT)
}
