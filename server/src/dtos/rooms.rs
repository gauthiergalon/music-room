use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{dtos::music::TrackItem, models::room::Room};

/// Pagination of the room list.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListRoomsQuery {
    /// Maximum number of rooms to return (1 to 100, default 50).
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DelegateRoomRequest {
    pub user_id: Uuid,
    pub connection_id: Uuid,
}

impl From<Room> for RoomResponse {
    fn from(room: Room) -> Self {
        Self {
            id: room.id,
            owner_id: room.owner_id,
            name: room.name,
            is_public: room.is_public,
            is_licensed: room.is_licensed,
            now_playing: room.current_track.map(|id| {
                TrackItem::from_cache(
                    id,
                    room.track_title,
                    room.track_artist,
                    room.track_album,
                    room.track_cover,
                    room.track_duration,
                )
            }),
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
    /// The current track with its metadata, for the room list.
    pub now_playing: Option<TrackItem>,
    pub current_position: i32,
    pub is_playing: bool,
}
