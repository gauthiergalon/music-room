use crate::dtos::hifi::{AlbumData, ArtistData, TrackItem};
use crate::dtos::ws::{QueuedTrack, WsEventServer};
use crate::repositories::{queue as queue_repo, rooms as rooms_repo, tracks as tracks_repo};
use crate::services::hifi;
use crate::state::AppState;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_room_state_event(state: &AppState, room_id: Uuid) -> Option<WsEventServer> {
    let pool = &state.pool;

    let room = rooms_repo::get_room_playback_state(pool, room_id)
        .await
        .unwrap_or(None)?;

    let current_track_item = get_current_track_item(pool, room.current_track).await;
    let queue_items = get_queue_items(pool, room_id).await;

    Some(WsEventServer::RoomState {
        current_track: current_track_item,
        is_playing: room.is_playing,
        current_position: room.current_position,
        timestamp: room.played_at.unwrap_or_else(Utc::now),
        queue: queue_items,
    })
}

pub async fn send_room_state(state: &AppState, room_id: Uuid) {
    let tx = {
        let rooms = state.active_rooms.read().await;
        if let Some(r) = rooms.get(&room_id) {
            r.tx.clone()
        } else {
            return;
        }
    };

    if let Some(event) = get_room_state_event(state, room_id).await {
        let _ = tx.send(event);
    }
}

async fn get_current_track_item(pool: &PgPool, track_id: Option<i64>) -> Option<TrackItem> {
    if let Some(id) = track_id {
        hifi::get_track_info(pool, id).await.ok()
    } else {
        None
    }
}

async fn get_queue_items(pool: &PgPool, room_id: Uuid) -> Vec<QueuedTrack> {
    let queue_records = queue_repo::get_queue(pool, room_id)
        .await
        .unwrap_or_default();

    let futures: Vec<_> = queue_records
        .into_iter()
        .map(|q| {
            let pool = pool.clone();
            async move {
                let track = match hifi::get_track_info(&pool, q.track_id).await {
                    Ok(track) => track,
                    Err(_) => get_fallback_track_item(&pool, q.track_id).await,
                };

                QueuedTrack {
                    id: q.id,
                    position: q.position,
                    track,
                }
            }
        })
        .collect();

    futures_util::future::join_all(futures).await
}

async fn get_fallback_track_item(pool: &PgPool, track_id: i64) -> TrackItem {
    let db_fallback = tracks_repo::get_track(pool, track_id)
        .await
        .ok()
        .flatten();

    if let Some(t) = db_fallback {
        TrackItem {
            id: track_id,
            title: t.title,
            duration: t.duration,
            audio_quality: None,
            album: Some(AlbumData {
                title: t.album,
                cover: t.cover,
            }),
            artists: t.artist.map(|a| vec![ArtistData { name: Some(a) }]),
        }
    } else {
        TrackItem {
            id: track_id,
            title: format!("Track {}", track_id),
            duration: 0,
            audio_quality: None,
            album: None,
            artists: None,
        }
    }
}
