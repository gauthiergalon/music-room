use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ResetToken {
    pub token_hash: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct NewResetToken {
    pub token_hash: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}
