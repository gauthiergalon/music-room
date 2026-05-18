use crate::models::user::PrivacyLevel;
use crate::models::user::User;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            email_confirmed: user.email_confirmed,
            favorite_genres: user.favorite_genres,
            privacy_level: user.privacy_level,
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_confirmed: bool,
    pub favorite_genres: Vec<String>,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PublicUserResponse {
    pub id: Uuid,
    pub username: String,
    pub favorite_genres: Option<Vec<String>>,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateFavoriteGenresRequest {
    pub favorite_genres: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdatePrivacyLevelRequest {
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateEmailRequest {
    pub new_email: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ConfirmEmailRequest {
    pub token: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}
