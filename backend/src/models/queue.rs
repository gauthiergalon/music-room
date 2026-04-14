use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Queue {
    pub id: Uuid,
    pub room_id: Uuid,
    pub track_id: i64,
    pub position: f64,
}
