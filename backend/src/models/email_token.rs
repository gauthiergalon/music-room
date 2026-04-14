use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct EmailToken {
    pub token_hash: String,
    pub user_id: Uuid,
    pub new_email: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct NewEmailToken {
    pub token_hash: String,
    pub user_id: Uuid,
    pub new_email: String,
    pub expires_at: DateTime<Utc>,
}
