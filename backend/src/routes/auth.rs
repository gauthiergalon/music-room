use crate::{
	errors::{AppError, ErrorMessage},
	middleware::auth::{Claims, auth_middleware, generate_access_token},
	services::auth as auth_service,
};
use axum::{Extension, Json, Router, extract::State, http::StatusCode, middleware, routing::post};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn router() -> Router<PgPool> {
	let public = Router::new().route("/register", post(register)).route("/login", post(login)).route("/refresh", post(refresh)).route("/forgot-password", post(forgot_password)).route("/reset-password", post(reset_password));

	let protected = Router::new().route("/logout", post(logout)).layer(middleware::from_fn(auth_middleware));

	Router::new().merge(public).merge(protected)
}

#[derive(Deserialize)]
struct RegisterRequest {
	username: String,
	email: String,
	password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
	email: String,
	password: String,
}

#[derive(Deserialize)]
struct LogoutRequest {
	refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse {
	access_token: String,
	refresh_token: String,
}

#[derive(Deserialize)]
struct RefreshRequest {
	refresh_token: String,
}

#[derive(Deserialize)]
struct ForgotPasswordRequest {
	email: String,
}

#[derive(Deserialize)]
struct ResetPasswordRequest {
	token: String,
	new_password: String,
}

async fn register(State(pool): State<PgPool>, Json(payload): Json<RegisterRequest>) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
	if payload.username.len() < 3 || payload.username.len() > 24 {
		return Err(AppError::Validation(ErrorMessage::UsernameInvalidLength));
	}
	if !validator::ValidateEmail::validate_email(&payload.email) {
		return Err(AppError::Validation(ErrorMessage::EmailInvalidFormat));
	}
	if payload.password.len() < 8 {
		return Err(AppError::Validation(ErrorMessage::PasswordInvalidPolicy));
	}

	let user_id = auth_service::create_user(&pool, &payload.username, &payload.email, &payload.password).await?;

	let access_token = generate_access_token(user_id)?;
	let refresh_token = auth_service::store_refresh_token(&pool, user_id).await?;

	Ok((StatusCode::CREATED, Json(AuthResponse { access_token, refresh_token })))
}

async fn login(State(pool): State<PgPool>, Json(payload): Json<LoginRequest>) -> Result<Json<AuthResponse>, AppError> {
	if !validator::ValidateEmail::validate_email(&payload.email) {
		return Err(AppError::Validation(ErrorMessage::EmailInvalidFormat));
	}

	let user_id = auth_service::authenticate_user(&pool, &payload.email, &payload.password).await?;

	let access_token = generate_access_token(user_id)?;
	let refresh_token = auth_service::store_refresh_token(&pool, user_id).await?;

	Ok(Json(AuthResponse { access_token, refresh_token }))
}

async fn logout(State(pool): State<PgPool>, Extension(claims): Extension<Claims>, Json(payload): Json<LogoutRequest>) -> Result<StatusCode, AppError> {
	auth_service::delete_refresh_token(&pool, &payload.refresh_token, &claims.user_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

async fn refresh(State(pool): State<PgPool>, Json(payload): Json<RefreshRequest>) -> Result<Json<AuthResponse>, AppError> {
	let user_id = auth_service::rotate_refresh_token(&pool, &payload.refresh_token).await?;

	let access_token = generate_access_token(user_id)?;
	let refresh_token = auth_service::store_refresh_token(&pool, user_id).await?;

	Ok(Json(AuthResponse { access_token, refresh_token }))
}

async fn forgot_password(State(pool): State<PgPool>, Json(payload): Json<ForgotPasswordRequest>) -> Result<StatusCode, AppError> {
	if !validator::ValidateEmail::validate_email(&payload.email) {
		return Err(AppError::Validation(ErrorMessage::EmailInvalidFormat));
	}

	auth_service::create_reset_token(&pool, &payload.email).await?;

	Ok(StatusCode::OK)
}

async fn reset_password(State(pool): State<PgPool>, Json(payload): Json<ResetPasswordRequest>) -> Result<StatusCode, AppError> {
	if payload.new_password.len() < 8 {
		return Err(AppError::Validation(ErrorMessage::PasswordInvalidPolicy));
	}
	auth_service::update_password_with_token(&pool, &payload.token, &payload.new_password).await?;

	Ok(StatusCode::OK)
}
