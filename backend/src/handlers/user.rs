use axum::{
	Extension, Json,
	extract::{Path, State},
	http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	dtos::user::{PublicUserResponse, UpdateEmailRequest, UpdatePasswordRequest, UpdateUsernameRequest, UserResponse},
	errors::AppError,
	middleware::auth::Claims,
};

pub async fn get_me(State(pool): State<PgPool>, Extension(claims): Extension<Claims>) -> Result<Json<UserResponse>, AppError> {
	todo!()
}

pub async fn get_user(State(pool): State<PgPool>, Path(user_id): Path<Uuid>) -> Result<Json<PublicUserResponse>, AppError> {
	todo!()
}

pub async fn update_username(State(pool): State<PgPool>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdateUsernameRequest>) -> Result<Json<UserResponse>, AppError> {
	todo!()
}

pub async fn update_email(Extension(claims): Extension<Claims>, State(pool): State<PgPool>, Json(payload): Json<UpdateEmailRequest>) -> Result<StatusCode, AppError> {
	todo!()
}

pub async fn update_password(State(pool): State<PgPool>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdatePasswordRequest>) -> Result<StatusCode, AppError> {
	todo!()
}
