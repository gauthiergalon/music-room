use axum::{
	Extension, Json, Router,
	extract::{Path, State},
	http::StatusCode,
	middleware,
	routing::{get, patch},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	errors::AppError,
	middleware::auth::{Claims, auth_middleware},
};

pub fn router() -> Router<PgPool> {
	let protected = Router::new().route("/{id}", get(get_user)).route("/me", get(get_me)).route("/me/username", patch(update_username)).route("/me/email", patch(update_email)).route("/me/password", patch(update_password)).layer(middleware::from_fn(auth_middleware));

	protected
}

#[derive(Serialize)]
struct UserResponse {
	id: Uuid,
	username: String,
	email: String,
	created_at: String,
}

#[derive(Serialize)]
struct PublicUserResponse {
	id: Uuid,
	username: String,
}

#[derive(Deserialize)]
struct UpdateUsernameRequest {
	username: String,
}

#[derive(Deserialize)]
struct UpdateEmailRequest {
	new_email: String,
}

#[derive(Deserialize)]
struct UpdatePasswordRequest {
	current_password: String,
	new_password: String,
}

async fn get_me(State(pool): State<PgPool>, Extension(claims): Extension<Claims>) -> Result<Json<UserResponse>, AppError> {
	todo!()
}

async fn get_user(State(pool): State<PgPool>, Path(user_id): Path<Uuid>) -> Result<Json<PublicUserResponse>, AppError> {
	todo!()
}

async fn update_username(State(pool): State<PgPool>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdateUsernameRequest>) -> Result<Json<UserResponse>, AppError> {
	todo!()
}

async fn update_email(Extension(claims): Extension<Claims>, State(pool): State<PgPool>, Json(payload): Json<UpdateEmailRequest>) -> Result<StatusCode, AppError> {
	todo!()
}

async fn update_password(State(pool): State<PgPool>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdatePasswordRequest>) -> Result<StatusCode, AppError> {
	todo!()
}
