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
pub enum WsEvent {
    Play { position: i32, timestamp: DateTime<Utc> },
    Pause { position: i32 },
    SeekTo { position: i32, timestamp: DateTime<Utc> },
    NextTrack { timestamp: DateTime<Utc> },
    QueueAdd { track_id: i64 },
    QueueRemove { track_id: i64 },
    QueueReorder { from_index: usize, to_index: usize },
    Error { message: String },
    UserJoin { user_id: Uuid, username: String },
    UserLeave { user_id: Uuid, username: String },
    SyncUsers { users: Vec<UserInfo> },
    UserOwnershipTransferred { new_owner_id: Uuid },
    RoomClosed,
}
