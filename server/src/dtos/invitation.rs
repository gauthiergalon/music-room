use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::invitation::{Invitation, InvitationWithDetails};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InvitationResponse {
    pub id: Uuid,
    pub room_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub is_pending: bool,
    pub created_at: DateTime<Utc>,
    /// Present when listing the invitations a user received.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inviter_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_name: Option<String>,
}

impl From<Invitation> for InvitationResponse {
    fn from(inv: Invitation) -> Self {
        Self {
            id: inv.id,
            room_id: inv.room_id,
            inviter_id: inv.inviter_id,
            invitee_id: inv.invitee_id,
            is_pending: inv.is_pending,
            created_at: inv.created_at,
            inviter_username: None,
            room_name: None,
        }
    }
}

impl From<InvitationWithDetails> for InvitationResponse {
    fn from(inv: InvitationWithDetails) -> Self {
        Self {
            id: inv.id,
            room_id: inv.room_id,
            inviter_id: inv.inviter_id,
            invitee_id: inv.invitee_id,
            is_pending: inv.is_pending,
            created_at: inv.created_at,
            inviter_username: Some(inv.inviter_username),
            room_name: Some(inv.room_name),
        }
    }
}
