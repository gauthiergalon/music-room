use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AddToQueueRequest {
	pub track_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveFromQueueRequest {
	pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReorderQueueRequest {
	pub id: Uuid,
	pub new_position: f64,
}
