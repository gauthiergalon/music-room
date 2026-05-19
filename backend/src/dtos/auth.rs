use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug, utoipa::ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct GoogleLoginRequest {
    pub id_token: String,
}
