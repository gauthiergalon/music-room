use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct UserResponse {
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub created_at: String,
}

#[derive(Serialize)]
pub struct PublicUserResponse {
	pub id: Uuid,
	pub username: String,
}

#[derive(Deserialize)]
pub struct UpdateUsernameRequest {
	pub username: String,
}

#[derive(Deserialize)]
pub struct UpdateEmailRequest {
	pub new_email: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordRequest {
	pub current_password: String,
	pub new_password: String,
}
