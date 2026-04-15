use crate::models::user::PrivacyLevel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub favorite_genres: Option<Vec<String>>,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Serialize)]
pub struct PublicUserResponse {
    pub id: Uuid,
    pub username: String,
    pub favorite_genres: Option<Vec<String>>,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFavoriteGenresRequest {
    pub favorite_genres: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePrivacyLevelRequest {
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmailRequest {
    pub new_email: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmEmailQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}
