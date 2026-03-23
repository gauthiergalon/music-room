use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct QueueItem {
	pub id: Uuid,
	pub room_id: Uuid,
	pub track_id: i64,
	pub added_by: Uuid,
	pub position: f64,
}
