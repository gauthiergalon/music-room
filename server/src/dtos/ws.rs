use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dtos::music::TrackItem;

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub username: String,
    pub connection_id: Uuid,
    pub device_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct QueuedTrack {
    pub id: Uuid,
    pub position: f64,
    pub track: TrackItem,
}

/// Events sent by the device that controls playback. Positions are in
/// milliseconds. The server stamps every event with its own clock, so clients
/// do not send timestamps (unknown fields are ignored).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum WsEventClient {
    Play {
        position: i32,
    },
    Pause {
        position: i32,
    },
    SeekTo {
        position: i32,
    },
    /// `item_id` is the queue entry the client believes is playing
    /// (`current_item_id` of the room state, `null` if none). The event is
    /// ignored when it does not match the room, which makes a repeated or late
    /// `NextTrack` harmless, even when the same track is queued twice in a row.
    NextTrack {
        #[serde(default)]
        item_id: Option<Uuid>,
    },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum WsEventServer {
    /// First message on every connection.
    Welcome {
        connection_id: Uuid,
        server_time: DateTime<Utc>,
        /// Whether this user may edit the queue while the license mode is on
        /// (owner, or accepted invitation). With `is_licensed` of the room
        /// state: the queue is editable when `!is_licensed || has_license`.
        has_license: bool,
    },
    RoomState {
        /// License mode: only users with a license may edit the queue.
        is_licensed: bool,
        current_track: Option<TrackItem>,
        /// Identifies this playback of `current_track`: the queue entry it
        /// came from. To send back with `NextTrack`.
        current_item_id: Option<Uuid>,
        /// Increases with every state of a room; restarts when the room is
        /// re-activated, so clients reset it when they (re)connect.
        version: u64,
        is_playing: bool,
        /// Playback position in milliseconds at `timestamp`.
        current_position: i32,
        /// Server clock instant at which `current_position` was valid.
        timestamp: DateTime<Utc>,
        /// Server clock when this message was built, for clock-offset estimation.
        server_time: DateTime<Utc>,
        queue: Vec<QueuedTrack>,
    },
    UserState {
        user_list: Vec<UserInfo>,
        owner: Uuid,
        delegate_user_id: Option<Uuid>,
        delegate_connection_id: Option<Uuid>,
    },
    RoomClosed,
    Error {
        message: String,
    },
}
