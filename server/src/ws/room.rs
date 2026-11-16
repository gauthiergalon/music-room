use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    dtos::{
        music::TrackItem,
        ws::{QueuedTrack, WsEventServer},
    },
    errors::AppError,
    repositories::{queue as queue_repo, rooms as rooms_repo},
    state::AppState,
};

/// Reads the playback state and the queue of a room. Two queries, whatever the
/// queue length: track metadata is joined from the `tracks` cache.
async fn load_room_state(
    pool: &PgPool,
    room_id: Uuid,
    version: u64,
) -> Result<Option<WsEventServer>, AppError> {
    let Some(room) = rooms_repo::get_room_playback_state(pool, room_id).await? else {
        return Ok(None);
    };

    let current_track = room.current_track.map(|track_id| {
        TrackItem::from_cache(
            track_id,
            room.title,
            room.artist,
            room.album,
            room.cover,
            room.duration,
        )
    });

    let queue = queue_repo::get_queue_with_tracks(pool, room_id)
        .await?
        .into_iter()
        .map(|record| QueuedTrack {
            id: record.id,
            position: record.position,
            track: TrackItem::from_cache(
                record.track_id,
                record.title,
                record.artist,
                record.album,
                record.cover,
                record.duration,
            ),
        })
        .collect();

    let now = Utc::now();
    Ok(Some(WsEventServer::RoomState {
        current_track,
        current_item_id: room.current_item,
        version,
        is_playing: room.is_playing,
        current_position: room.current_position,
        timestamp: room.played_at.unwrap_or(now),
        server_time: now,
        queue,
    }))
}

/// Builds the next state of an active room, numbered after the previous one.
///
/// Reading the database and numbering the state happen under the room's state
/// lock, so states are always published in the order they were read: a slow
/// reader can never overwrite a newer state with an older one.
async fn next_room_state(state: &AppState, room_id: Uuid) -> Option<(WsEventServer, StateSender)> {
    let handle = state.rooms.state_handle(room_id).await?;
    let mut version = handle.version.clone().lock_owned().await;

    match load_room_state(&state.pool, room_id, *version + 1).await {
        Ok(Some(event)) => {
            *version += 1;
            Some((
                event,
                StateSender {
                    tx: handle.tx,
                    _guard: version,
                },
            ))
        }
        Ok(None) => None,
        Err(e) => {
            tracing::error!("Failed to load the state of room {}: {:?}", room_id, e);
            None
        }
    }
}

/// Keeps the state lock until the event has been handed to its recipients.
struct StateSender {
    tx: tokio::sync::broadcast::Sender<WsEventServer>,
    _guard: tokio::sync::OwnedMutexGuard<u64>,
}

/// Broadcasts the current state of the room to every connected client.
pub async fn send_room_state(state: &AppState, room_id: Uuid) {
    if let Some((event, sender)) = next_room_state(state, room_id).await {
        let _ = sender.tx.send(event);
    }
}

/// The current state of the room, for a single client (on connect or resync).
pub async fn get_room_state_event(state: &AppState, room_id: Uuid) -> Option<WsEventServer> {
    next_room_state(state, room_id)
        .await
        .map(|(event, _)| event)
}
