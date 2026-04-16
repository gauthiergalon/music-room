use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationResponse {
    pub id: Uuid,
    pub room_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub is_pending: bool,
    pub created_at: DateTime<Utc>,
}
