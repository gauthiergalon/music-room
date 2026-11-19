use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub is_public: bool,
    pub is_licensed: bool,
    pub current_track: Option<i64>,
    pub current_position: i32,
    pub is_playing: bool,
    // Metadata of the current track, cached in `tracks` when it was queued.
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub track_album: Option<String>,
    pub track_cover: Option<String>,
    pub track_duration: Option<i32>,
}
