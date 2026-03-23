use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum WsEvent {
	Play,
	Pause,
	SyncPosition { position: i32 },
	NextTrack,
	PreviousTrack,
	QueueAdd { track_id: i64 },
	QueueRemove { track_id: i64 },
	Error { message: String },
	NewUser { user_id: Uuid },
}
