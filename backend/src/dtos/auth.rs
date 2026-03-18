use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterRequest {
	pub username: String,
	pub email: String,
	pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
	pub email: String,
	pub password: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
	pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthResponse {
	pub access_token: String,
	pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
	pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
	pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
	pub token: String,
	pub new_password: String,
}
