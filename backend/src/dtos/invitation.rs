use crate::models::invitation::Invitation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

impl From<Invitation> for InvitationResponse {
    fn from(inv: Invitation) -> Self {
        Self {
            id: inv.id,
            room_id: inv.room_id,
            inviter_id: inv.inviter_id,
            invitee_id: inv.invitee_id,
            is_pending: inv.is_pending,
            created_at: inv.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct InvitationResponse {
    pub id: Uuid,
    pub room_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub is_pending: bool,
    pub created_at: DateTime<Utc>,
}
