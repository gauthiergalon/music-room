use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Room {
	pub id: Uuid,
	pub owner_id: Uuid,
	pub is_public: bool,
	pub current_track: Option<i64>,
	pub current_position: i32,
	pub is_playing: bool,
}
