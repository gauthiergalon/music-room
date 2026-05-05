use axum::{
    Extension, Json,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

use crate::{
    dtos::{
        rooms::{RoomResponse, TransferOwnershipRequest},
        ws::WsEventClient,
        ws::WsEventServer,
    },
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    services::invitations as invitation_service,
    services::rooms as room_service,
    state::AppState,
};

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<RoomResponse>>, AppError> {
    let rooms = room_service::list(&state.pool).await?;
    let responses = rooms
        .into_iter()
        .map(|room| RoomResponse {
            id: room.id,
            owner_id: room.owner_id,
            name: room.name,
            is_public: room.is_public,
            is_licensed: room.is_licensed,
            current_track: room.current_track,
            current_position: room.current_position,
            is_playing: room.is_playing,
        })
        .collect();
    Ok(Json(responses))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
    let name = format!("{}'s room", claims.username);
    let room = room_service::create(&state.pool, claims.user_id, &name).await?;
    Ok((
        StatusCode::CREATED,
        Json(RoomResponse {
            id: room.id,
            owner_id: room.owner_id,
            name: room.name,
            is_public: room.is_public,
            is_licensed: room.is_licensed,
            current_track: room.current_track,
            current_position: room.current_position,
            is_playing: room.is_playing,
        }),
    ))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::delete(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn enable_license(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::enable_license(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn disable_license(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::disable_license(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<RoomResponse>, AppError> {
    let room = room_service::get(&state.pool, room_id, claims.user_id).await?;
    Ok(Json(RoomResponse {
        id: room.id,
        owner_id: room.owner_id,
        name: room.name,
        is_public: room.is_public,
        is_licensed: room.is_licensed,
        current_track: room.current_track,
        current_position: room.current_position,
        is_playing: room.is_playing,
    }))
}

pub async fn transfer_ownership(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<TransferOwnershipRequest>,
) -> Result<StatusCode, AppError> {
    room_service::transfer_ownership(&state.pool, room_id, claims.user_id, payload.new_owner_id)
        .await?;

    if let Some(tx) = state.active_rooms.read().await.get(&room_id) {
        let users: Vec<crate::dtos::ws::UserInfo> = tx
            .users
            .iter()
            .map(|(id, name)| crate::dtos::ws::UserInfo {
                user_id: *id,
                username: name.clone(),
            })
            .collect();
        let _ = tx.tx.send(WsEventServer::UserState {
            user_list: users,
            owner: payload.new_owner_id,
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn publish(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::publish(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn privatize(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::privatize(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<axum::response::Response, AppError> {
    let room = room_service::get(&state.pool, room_id, claims.user_id).await?;
    let is_owner = room.owner_id == claims.user_id;

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            state,
            room_id,
            is_owner,
            claims.user_id,
            claims.username,
        )
    }))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    room_id: Uuid,
    is_owner: bool,
    user_id: Uuid,
    username: String,
) {
    let (mut sender, mut receiver) = socket.split();

    let (tx, users_list) = {
        let mut channels = state.active_rooms.write().await;
        let room_entry = channels
            .entry(room_id)
            .or_insert_with(|| crate::state::ActiveRoom {
                tx: tokio::sync::broadcast::channel(100).0,
                users: std::collections::HashMap::new(),
            });
        room_entry.users.insert(user_id, username.clone());
        let list: Vec<crate::dtos::ws::UserInfo> = room_entry
            .users
            .iter()
            .map(|(id, name)| crate::dtos::ws::UserInfo {
                user_id: *id,
                username: name.clone(),
            })
            .collect();
        (room_entry.tx.clone(), list)
    };

    let real_owner_id = sqlx::query!("SELECT owner_id FROM rooms WHERE id = $1", room_id)
        .fetch_one(&state.pool)
        .await
        .map(|r| r.owner_id)
        .unwrap_or(user_id);

    let user_state_msg = WsEventServer::UserState {
        user_list: users_list,
        owner: real_owner_id,
    };

    if let Ok(sync_msg) = serde_json::to_string(&user_state_msg) {
        let _ = sender.send(Message::Text(sync_msg.into())).await;
    }

    let _ = tx.send(user_state_msg);

    let tx_clone = tx.clone();
    let mut rx = tx.subscribe();
    let pool = state.pool.clone();

    let state_for_recv = state.clone();
    let mut recv_task = tokio::spawn(async move {
        let state = state_for_recv;
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(event) = serde_json::from_str::<WsEventClient>(&text) {
                let is_authorized = match &event {
                    WsEventClient::Play { .. }
                    | WsEventClient::Pause { .. }
                    | WsEventClient::SeekTo { .. }
                    | WsEventClient::NextTrack { .. } => is_owner,
                };

                if is_authorized {
                    let update_result = match &event {
                        WsEventClient::Play {
                            timestamp,
                            position,
                        } => {
                            sqlx::query!(
                                "UPDATE rooms SET is_playing = true, played_at = $1, current_position = $2 WHERE id = $3",
                                timestamp,
                                position,
                                room_id
                            )
                            .execute(&pool)
                            .await
                        }
                        WsEventClient::Pause { position } => {
                            sqlx::query!(
                                "UPDATE rooms SET is_playing = false, current_position = $1 WHERE id = $2",
                                position,
                                room_id
                            )
                            .execute(&pool)
                            .await
                        }
                        WsEventClient::SeekTo {
                            position,
                            timestamp,
                        } => {
                            sqlx::query!(
                                "UPDATE rooms SET current_position = $1, played_at = $2 WHERE id = $3",
                                position,
                                timestamp,
                                room_id
                            )
                            .execute(&pool)
                            .await
                        }
                        WsEventClient::NextTrack { timestamp } => {
                            let next_in_queue = sqlx::query!(
                                "DELETE FROM queue WHERE id = (
                                    SELECT id FROM queue WHERE room_id = $1 ORDER BY position ASC LIMIT 1
                                ) RETURNING track_id",
                                room_id
                            )
                            .fetch_optional(&pool)
                            .await
                            .unwrap_or(None);

                            if let Some(next) = next_in_queue {
                                sqlx::query!(
                                    "UPDATE rooms SET current_track = $1, played_at = $2, current_position = 0, is_playing = true WHERE id = $3",
                                    next.track_id,
                                    timestamp,
                                    room_id
                                )
                                .execute(&pool)
                                .await
                            } else {
                                sqlx::query!(
                                    "UPDATE rooms SET is_playing = false, current_track = NULL WHERE id = $1",
                                    room_id
                                )
                                .execute(&pool)
                                .await
                            }
                        }
                    };

                    if let Err(e) = update_result {
                        tracing::error!("Failed to update room state in db: {}", e);
                    }

                    broadcast_room_state(&state, room_id).await;
                }
            } else {
                let error_event = WsEventServer::Error {
                    message: "Invalid event format".to_string(),
                };
                let _ = tx_clone.send(error_event);
            }
        }
    });

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg)
                && sender.send(Message::Text(text.into())).await.is_err()
            {
                break;
            }
        }
    });

    tokio::select! {
    _ = (&mut recv_task) => send_task.abort(),
    _ = (&mut send_task) => recv_task.abort(),
    };

    let mut is_empty = false;
    let mut updated_users_list = vec![];
    {
        let mut channels = state.active_rooms.write().await;
        if let std::collections::hash_map::Entry::Occupied(mut entry) = channels.entry(room_id) {
            entry.get_mut().users.remove(&user_id);
            if entry.get().users.is_empty() {
                is_empty = true;
                entry.remove();
            } else {
                updated_users_list = entry
                    .get()
                    .users
                    .iter()
                    .map(|(id, name)| crate::dtos::ws::UserInfo {
                        user_id: *id,
                        username: name.clone(),
                    })
                    .collect();
            }
        }
    }

    if !is_empty {
        let current_owner_id = sqlx::query!("SELECT owner_id FROM rooms WHERE id = $1", room_id)
            .fetch_one(&state.pool)
            .await
            .map(|r| r.owner_id)
            .unwrap_or(user_id);

        let _ = tx.send(WsEventServer::UserState {
            user_list: updated_users_list,
            owner: current_owner_id,
        });
    }

    if is_empty {
        let _ = sqlx::query!("DELETE FROM rooms WHERE id = $1", room_id)
            .execute(&state.pool)
            .await;
        let _ = tx.send(WsEventServer::RoomClosed);
    }
}
pub async fn broadcast_room_state(state: &crate::state::AppState, room_id: uuid::Uuid) {
    // Check if room is active first so we don't query db unnecessarily if no one is listening
    let tx = {
        let rooms = state.active_rooms.read().await;
        if let Some(r) = rooms.get(&room_id) {
            r.tx.clone()
        } else {
            return;
        }
    };

    let pool = &state.pool;

    let room = sqlx::query!(
        "SELECT current_track, is_playing, current_position, played_at FROM rooms WHERE id = $1",
        room_id
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if let Some(r) = room {
        let current_track_item = if let Some(track_id) = r.current_track {
            crate::services::hifi::get_track_info(pool, track_id)
                .await
                .ok()
        } else {
            None
        };

        let queue_track_ids = sqlx::query!(
            "SELECT track_id FROM queue WHERE room_id = $1 ORDER BY position ASC",
            room_id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|q| q.track_id)
        .collect::<Vec<i64>>();

        let futures: Vec<_> = queue_track_ids
            .into_iter()
            .map(|id| {
                let pool = pool.clone();
                async move {
                    match crate::services::hifi::get_track_info(&pool, id).await {
                        Ok(track) => track,
                        Err(_) => {
                            let db_fallback = sqlx::query!(
                                "SELECT title, artist, album, cover, duration FROM tracks WHERE id = $1",
                                id
                            )
                            .fetch_optional(&pool)
                            .await
                            .ok()
                            .flatten();

                            if let Some(t) = db_fallback {
                                crate::dtos::hifi::TrackItem {
                                    id,
                                    title: t.title,
                                    duration: t.duration,
                                    audio_quality: None,
                                    album: Some(crate::dtos::hifi::AlbumData {
                                        title: t.album,
                                        cover: t.cover,
                                    }),
                                    artists: Some(vec![crate::dtos::hifi::ArtistData {
                                        name: Some(t.artist),
                                    }]),
                                }
                            } else {
                                crate::dtos::hifi::TrackItem {
                                    id,
                                    title: format!("Track {}", id),
                                    duration: 0,
                                    audio_quality: None,
                                    album: None,
                                    artists: None,
                                }
                            }
                        }
                    }
                }
            })
            .collect();

        let queue_items: Vec<_> = futures_util::future::join_all(futures).await;

        let _ = tx.send(crate::dtos::ws::WsEventServer::RoomState {
            current_track: current_track_item,
            is_playing: r.is_playing,
            current_position: r.current_position,
            timestamp: r.played_at.unwrap_or_else(chrono::Utc::now),
            queue: queue_items,
        });
    }
}
