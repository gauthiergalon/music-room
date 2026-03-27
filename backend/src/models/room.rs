use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub is_public: bool,
    pub current_track: Option<i64>,
    pub current_position: i32,
    pub is_playing: bool,
}
