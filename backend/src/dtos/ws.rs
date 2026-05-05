use crate::dtos::hifi::TrackItem;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum WsEventClient {
    Play {
        position: i32,
        timestamp: DateTime<Utc>,
    },
    Pause {
        position: i32,
    },
    SeekTo {
        position: i32,
        timestamp: DateTime<Utc>,
    },
    NextTrack {
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum WsEventServer {
    RoomState {
        current_track: Option<TrackItem>,
        is_playing: bool,
        current_position: i32,
        timestamp: DateTime<Utc>,
        queue: Vec<TrackItem>,
    },
    UserState {
        user_list: Vec<UserInfo>,
        owner: Uuid,
    },
    RoomClosed,
    Error {
        message: String,
    },
}
