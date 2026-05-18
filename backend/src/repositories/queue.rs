use sqlx::PgPool;
use uuid::Uuid;

use crate::models::queue::Queue;

pub async fn find_all_by_room_id(pool: &PgPool, room_id: Uuid) -> Result<Vec<Queue>, sqlx::Error> {
    let rows = sqlx::query!("SELECT id, room_id, track_id, position FROM queue WHERE room_id = $1 ORDER BY position ASC", room_id).fetch_all(pool).await?;

    let queue = rows
        .into_iter()
        .map(|row| Queue {
            id: row.id,
            room_id: row.room_id,
            track_id: row.track_id,
            position: row.position,
        })
        .collect();

    Ok(queue)
}

pub async fn create(pool: &PgPool, room_id: Uuid, track_id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!("SELECT id FROM rooms WHERE id = $1 FOR UPDATE", room_id)
        .fetch_optional(&mut *tx)
        .await?;

    sqlx::query!("INSERT INTO queue (room_id, track_id, position) VALUES ($1, $2, COALESCE((SELECT MAX(position) + 1 FROM queue WHERE room_id = $1), 0))", room_id, track_id).execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(())
}

pub async fn remove(pool: &PgPool, room_id: Uuid, queue_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM queue WHERE room_id = $1 AND id = $2",
        room_id,
        queue_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn reorder(
    pool: &PgPool,
    room_id: Uuid,
    queue_id: Uuid,
    new_position: f64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!("SELECT id FROM rooms WHERE id = $1 FOR UPDATE", room_id)
        .fetch_optional(&mut *tx)
        .await?;

    sqlx::query!(
        "UPDATE queue SET position = $1 WHERE room_id = $2 AND id = $3",
        new_position,
        room_id,
        queue_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn pop_next_track(pool: &sqlx::PgPool, room_id: uuid::Uuid) -> sqlx::Result<Option<i64>> {
    let mut tx = pool.begin().await?;

    let next_track = sqlx::query!(
        "SELECT id, track_id FROM queue WHERE room_id = $1 ORDER BY position ASC LIMIT 1 FOR UPDATE SKIP LOCKED",
        room_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(track) = next_track {
        sqlx::query!("DELETE FROM queue WHERE id = $1", track.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(track.track_id))
    } else {
        tx.rollback().await?;
        Ok(None)
    }
}

pub struct QueueRecord {
    pub id: uuid::Uuid,
    pub track_id: i64,
    pub position: f64,
}

pub async fn get_queue(pool: &sqlx::PgPool, room_id: uuid::Uuid) -> sqlx::Result<Vec<QueueRecord>> {
    sqlx::query_as!(
        QueueRecord,
        "SELECT id, track_id, position FROM queue WHERE room_id = $1 ORDER BY position ASC",
        room_id
    )
    .fetch_all(pool)
    .await
}
