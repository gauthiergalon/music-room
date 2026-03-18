use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct RoomResponse {
	pub id: Uuid,
	pub owner_id: Uuid,
}

#[derive(Deserialize)]
pub struct TransferOwnershipRequest {
	pub new_owner_id: Uuid,
}
