use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage, UNIQUE_VIOLATION},
    models::queue::Queue,
};

pub async fn find_all_by_room_id(pool: &PgPool, room_id: Uuid) -> Result<Vec<Queue>, AppError> {
    let queue = sqlx::query_as!(
        Queue,
        "SELECT id, room_id, track_id, position FROM queue WHERE room_id = $1 ORDER BY position ASC",
        room_id
    )
    .fetch_all(pool)
    .await?;
    Ok(queue)
}

pub async fn create(pool: &PgPool, room_id: Uuid, track_id: i64) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    // Serialise concurrent appends so two tracks cannot get the same position.
    sqlx::query!("SELECT id FROM rooms WHERE id = $1 FOR UPDATE", room_id)
        .fetch_optional(&mut *tx)
        .await?;

    sqlx::query!(
        "INSERT INTO queue (room_id, track_id, position) VALUES ($1, $2, COALESCE((SELECT MAX(position) + 1 FROM queue WHERE room_id = $1), 0))",
        room_id,
        track_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn remove(pool: &PgPool, room_id: Uuid, queue_id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query!(
        "DELETE FROM queue WHERE room_id = $1 AND id = $2",
        room_id,
        queue_id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(ErrorMessage::QueueItemNotFound));
    }
    Ok(())
}

pub async fn reorder(
    pool: &PgPool,
    room_id: Uuid,
    queue_id: Uuid,
    new_position: f64,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        "UPDATE queue SET position = $1 WHERE room_id = $2 AND id = $3",
        new_position,
        room_id,
        queue_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        if AppError::is_db_violation(&e, UNIQUE_VIOLATION) {
            AppError::Conflict(ErrorMessage::QueuePositionTaken)
        } else {
            AppError::Database(e)
        }
    })?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(ErrorMessage::QueueItemNotFound));
    }
    Ok(())
}

pub struct QueuedTrackRecord {
    pub id: Uuid,
    pub track_id: i64,
    pub position: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover: Option<String>,
    pub duration: Option<i32>,
}

/// The queue with the cached metadata of every track, in one query.
pub async fn get_queue_with_tracks(
    pool: &PgPool,
    room_id: Uuid,
) -> Result<Vec<QueuedTrackRecord>, AppError> {
    let records = sqlx::query_as!(
        QueuedTrackRecord,
        r#"
        SELECT q.id, q.track_id, q.position,
               t.title AS "title?", t.artist AS "artist?", t.album AS "album?",
               t.cover AS "cover?", t.duration AS "duration?"
        FROM queue q
        LEFT JOIN tracks t ON t.id = q.track_id
        WHERE q.room_id = $1
        ORDER BY q.position ASC
        "#,
        room_id
    )
    .fetch_all(pool)
    .await?;
    Ok(records)
}
