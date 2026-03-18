use crate::{
	dtos::auth::{AuthResponse, ForgotPasswordRequest, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest, ResetPasswordRequest},
	errors::{AppError, ErrorMessage},
	middleware::auth::{Claims, generate_access_token},
	services::auth as auth_service,
};
use axum::{Extension, Json, extract::State, http::StatusCode};
use sqlx::PgPool;

pub async fn register(State(pool): State<PgPool>, Json(payload): Json<RegisterRequest>) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
	let mut errors = Vec::new();

	if payload.username.len() < 3 || payload.username.len() > 24 {
		errors.push(ErrorMessage::UsernameInvalidLength);
	}
	if !validator::ValidateEmail::validate_email(&payload.email) {
		errors.push(ErrorMessage::EmailInvalidFormat);
	}
	if payload.password.len() < 8 {
		errors.push(ErrorMessage::PasswordInvalidPolicy);
	}

	if !errors.is_empty() {
		return Err(AppError::Validation(errors));
	}

	let user_id = auth_service::create_user(&pool, &payload.username, &payload.email, &payload.password).await?;

	let access_token = generate_access_token(user_id)?;
	let refresh_token = auth_service::store_refresh_token(&pool, user_id).await?;

	Ok((StatusCode::CREATED, Json(AuthResponse { access_token, refresh_token })))
}

pub async fn login(State(pool): State<PgPool>, Json(payload): Json<LoginRequest>) -> Result<Json<AuthResponse>, AppError> {
	if !validator::ValidateEmail::validate_email(&payload.email) {
		return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
	}

	let user_id = auth_service::authenticate_user(&pool, &payload.email, &payload.password).await?;

	let access_token = generate_access_token(user_id)?;
	let refresh_token = auth_service::store_refresh_token(&pool, user_id).await?;

	Ok(Json(AuthResponse { access_token, refresh_token }))
}

pub async fn logout(State(pool): State<PgPool>, Extension(claims): Extension<Claims>, Json(payload): Json<LogoutRequest>) -> Result<StatusCode, AppError> {
	auth_service::delete_refresh_token(&pool, &payload.refresh_token, &claims.user_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn refresh(State(pool): State<PgPool>, Json(payload): Json<RefreshRequest>) -> Result<Json<AuthResponse>, AppError> {
	let user_id = auth_service::rotate_refresh_token(&pool, &payload.refresh_token).await?;

	let access_token = generate_access_token(user_id)?;
	let refresh_token = auth_service::store_refresh_token(&pool, user_id).await?;

	Ok(Json(AuthResponse { access_token, refresh_token }))
}

pub async fn forgot_password(State(pool): State<PgPool>, Json(payload): Json<ForgotPasswordRequest>) -> Result<StatusCode, AppError> {
	if !validator::ValidateEmail::validate_email(&payload.email) {
		return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
	}

	auth_service::create_reset_token(&pool, &payload.email).await?;

	Ok(StatusCode::OK)
}

pub async fn reset_password(State(pool): State<PgPool>, Json(payload): Json<ResetPasswordRequest>) -> Result<StatusCode, AppError> {
	if payload.new_password.len() < 8 {
		return Err(AppError::Validation(vec![ErrorMessage::PasswordInvalidPolicy]));
	}
	auth_service::update_password_with_token(&pool, &payload.token, &payload.new_password).await?;

	Ok(StatusCode::OK)
}
