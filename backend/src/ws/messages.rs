use crate::state::AppState;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub async fn handle_next_track(state: &AppState, room_id: Uuid, timestamp: DateTime<Utc>) {
    let pool = &state.pool;
    let next_in_queue = crate::repositories::queue::pop_next_track(pool, room_id)
        .await
        .unwrap_or(None);

    if let Some(track_id) = next_in_queue {
        let _ = crate::repositories::rooms::update_current_track_and_play(
            pool, room_id, track_id, timestamp,
        )
        .await;
    } else {
        let _ = crate::repositories::rooms::clear_current_track_and_pause(pool, room_id).await;
    }
}
