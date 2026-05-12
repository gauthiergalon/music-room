use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use uuid::Uuid;

use crate::state::ActiveRoom;

pub fn spawn_token_cleanup_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60 * 60 * 24));

        loop {
            interval.tick().await;

            let res_refresh = sqlx::query!(
                "DELETE FROM refresh_tokens WHERE expires_at < $1",
                Utc::now()
            )
            .execute(&pool)
            .await;

            if let Ok(result) = res_refresh {
                tracing::info!(
                    "Cleaned up {} expired refresh tokens",
                    result.rows_affected()
                );
            }

            let res_reset =
                sqlx::query!("DELETE FROM reset_tokens WHERE expires_at < $1", Utc::now())
                    .execute(&pool)
                    .await;

            if let Ok(result) = res_reset {
                tracing::info!("Cleaned up {} expired reset tokens", result.rows_affected());
            }
        }
    });
}

pub fn spawn_room_cleanup_task(
    pool: PgPool,
    active_rooms: Arc<RwLock<std::collections::HashMap<Uuid, ActiveRoom>>>,
) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60 * 5));

        loop {
            interval.tick().await;

            let rooms = sqlx::query!("SELECT id, owner_id FROM rooms")
                .fetch_all(&pool)
                .await;

            if let Ok(rooms) = rooms {
                for room in rooms {
                    let active = {
                        let rooms_guard = active_rooms.read().await;
                        rooms_guard.contains_key(&room.id)
                    };

                    if !active {
                        tracing::info!("Cleaning up orphaned room: {}", room.id);
                        let _ = sqlx::query!("DELETE FROM rooms WHERE id = $1", room.id)
                            .execute(&pool)
                            .await;
                    }
                }
            }
        }
    });
}

pub async fn cleanup_user_rooms(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let rooms = sqlx::query!("SELECT id FROM rooms WHERE owner_id = $1", user_id)
        .fetch_all(pool)
        .await?;

    for room in rooms {
        sqlx::query!("DELETE FROM rooms WHERE id = $1", room.id)
            .execute(pool)
            .await?;
    }

    Ok(())
}
