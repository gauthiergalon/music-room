use sqlx::PgPool;
use uuid::Uuid;

use crate::models::queue::Queue;

pub async fn find_all_by_room_id(pool: &PgPool, room_id: Uuid) -> Result<Vec<Queue>, sqlx::Error> {
	let rows = sqlx::query!("SELECT id, room_id, track_id, position FROM queue WHERE room_id = $1 ORDER BY position ASC", room_id).fetch_all(pool).await?;

	let queue = rows.into_iter().map(|row| Queue { id: row.id, room_id: row.room_id, track_id: row.track_id, position: row.position }).collect();

	Ok(queue)
}

pub async fn create(pool: &PgPool, room_id: Uuid, track_id: i64) -> Result<(), sqlx::Error> {
	sqlx::query!("INSERT INTO queue (room_id, track_id, position) VALUES ($1, $2, COALESCE((SELECT MAX(position) + 1 FROM queue WHERE room_id = $1), 0))", room_id, track_id).execute(pool).await?;

	Ok(())
}

pub async fn remove(pool: &PgPool, room_id: Uuid, queue_id: Uuid) -> Result<(), sqlx::Error> {
	sqlx::query!("DELETE FROM queue WHERE room_id = $1 AND id = $2", room_id, queue_id).execute(pool).await?;

	Ok(())
}

pub async fn reorder(pool: &PgPool, room_id: Uuid, queue_id: Uuid, new_position: f64) -> Result<(), sqlx::Error> {
	sqlx::query!("UPDATE queue SET position = $1 WHERE room_id = $2 AND id = $3", new_position, room_id, queue_id).execute(pool).await?;

	Ok(())
}
