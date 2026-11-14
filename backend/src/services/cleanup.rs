use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use tokio::time;
use uuid::Uuid;

use crate::{repositories::rooms as rooms_repo, services::rooms as room_service, state::AppState};

const TOKEN_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24);
const SUBSCRIPTION_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const ROOM_CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// A room nobody controls any more (no play, pause, seek or skip) is closed after this long.
const ROOM_IDLE_TIMEOUT_HOURS: i64 = 2;
/// A room whose owner is not connected is deleted once it has been untouched
/// this long. It covers rooms that were created but never joined, and rooms
/// left behind by a server restart whose owner never came back.
const ROOM_ABANDONED_AFTER_MINUTES: i64 = 2;

pub fn spawn_token_cleanup_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = time::interval(TOKEN_CLEANUP_INTERVAL);

        loop {
            interval.tick().await;

            for table in ["refresh_tokens", "reset_tokens", "email_tokens"] {
                // The table name comes from the fixed list above, never from user input.
                let query = format!("DELETE FROM {table} WHERE expires_at < now()");
                match sqlx::query(&query).execute(&pool).await {
                    Ok(result) => tracing::info!(
                        "Cleaned up {} expired rows from {}",
                        result.rows_affected(),
                        table
                    ),
                    Err(e) => tracing::error!("Failed to clean up {}: {}", table, e),
                }
            }
        }
    });
}

pub fn spawn_subscription_cleanup_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = time::interval(SUBSCRIPTION_CLEANUP_INTERVAL);

        loop {
            interval.tick().await;

            let result = sqlx::query!(
                r#"
                UPDATE users
                SET is_subscribed = FALSE
                WHERE is_subscribed = TRUE
                  AND end_subscription_date IS NOT NULL
                  AND end_subscription_date <= NOW()
                "#
            )
            .execute(&pool)
            .await;

            match result {
                Ok(result) => tracing::info!(
                    "Cleaned up {} expired subscriptions",
                    result.rows_affected()
                ),
                Err(e) => tracing::error!("Failed to clean up subscriptions: {}", e),
            }
        }
    });
}

pub fn spawn_room_cleanup_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = time::interval(ROOM_CLEANUP_INTERVAL);

        loop {
            interval.tick().await;
            for room_id in rooms_to_close(&state).await {
                tracing::info!("Cleaning up inactive room {}", room_id);
                if let Err(e) = room_service::close(&state, room_id).await {
                    tracing::error!("Failed to clean up room {}: {:?}", room_id, e);
                }
            }
        }
    });
}

/// Rooms that should not exist any more. A room whose owner is connected is
/// only ever closed for being idle.
pub async fn rooms_to_close(state: &AppState) -> Vec<Uuid> {
    let mut to_close = state
        .rooms
        .idle_rooms(chrono::Duration::hours(ROOM_IDLE_TIMEOUT_HOURS))
        .await;

    let threshold = Utc::now() - chrono::Duration::minutes(ROOM_ABANDONED_AFTER_MINUTES);
    match rooms_repo::find_inactive_since(&state.pool, threshold).await {
        Ok(candidates) => {
            for room_id in candidates {
                if !state.rooms.owner_connected(room_id).await && !to_close.contains(&room_id) {
                    to_close.push(room_id);
                }
            }
        }
        Err(e) => tracing::error!("Failed to list inactive rooms: {:?}", e),
    }

    to_close
}
