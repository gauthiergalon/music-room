use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invitation {
    pub id: Uuid,
    pub room_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub is_pending: bool,
    pub created_at: DateTime<Utc>,
}

/// A pending invitation with what the invitee needs to display it.
#[derive(Debug, Clone)]
pub struct InvitationWithDetails {
    pub id: Uuid,
    pub room_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub is_pending: bool,
    pub created_at: DateTime<Utc>,
    pub inviter_username: String,
    pub room_name: String,
}
