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

pub async fn get_me(State(state): State<crate::state::AppState>, Extension(claims): Extension<Claims>) -> Result<Json<UserResponse>, AppError> {
	todo!()
}

pub async fn get_user(State(state): State<crate::state::AppState>, Path(user_id): Path<Uuid>) -> Result<Json<PublicUserResponse>, AppError> {
	todo!()
}

pub async fn update_username(State(state): State<crate::state::AppState>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdateUsernameRequest>) -> Result<Json<UserResponse>, AppError> {
	todo!()
}

pub async fn update_email(Extension(claims): Extension<Claims>, State(state): State<crate::state::AppState>, Json(payload): Json<UpdateEmailRequest>) -> Result<StatusCode, AppError> {
	todo!()
}

pub async fn update_password(State(state): State<crate::state::AppState>, Extension(claims): Extension<Claims>, Json(payload): Json<UpdatePasswordRequest>) -> Result<StatusCode, AppError> {
	todo!()
}
