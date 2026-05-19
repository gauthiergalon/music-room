use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::room::Room;

impl From<Room> for RoomResponse {
    fn from(room: Room) -> Self {
        Self {
            id: room.id,
            owner_id: room.owner_id,
            name: room.name,
            is_public: room.is_public,
            is_licensed: room.is_licensed,
            current_track: room.current_track,
            current_position: room.current_position,
            is_playing: room.is_playing,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RoomResponse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub is_public: bool,
    pub is_licensed: bool,
    pub current_track: Option<i64>,
    pub current_position: i32,
    pub is_playing: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TransferOwnershipRequest {
    pub new_owner_id: Uuid,
}
