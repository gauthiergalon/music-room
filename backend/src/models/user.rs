use uuid::Uuid;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "privacy_level", rename_all = "lowercase")]
pub enum PrivacyLevel {
    Public,
    Friends,
    Private,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub email_confirmed: Option<bool>,
    pub google_id: Option<String>,
    pub favorite_genres: Option<Vec<String>>,
    pub privacy_level: PrivacyLevel,
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: Option<String>,
    pub email_confirmed: Option<bool>,
    pub google_id: Option<String>,
    pub favorite_genres: Option<Vec<String>>,
    pub privacy_level: PrivacyLevel,
}
