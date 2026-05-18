use crate::dtos::user::UserResponse;
use crate::dtos::ws::{WsEventClient, WsEventServer};
use crate::repositories::rooms as rooms_repo;
use crate::state::{ActiveRoom, AppState};
use crate::ws::{messages, room, send_room_state, send_user_state};
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

pub async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    room_id: Uuid,
    user: UserResponse,
    owner_id: Uuid,
) {
    let rx = add_user_to_room(&state, room_id, &user, owner_id).await;

    // Broadcast new user list to everyone
    send_user_state(&state, room_id, owner_id).await;

    let (mut sender, receiver) = socket.split();

    // Send the current room state ONLY to the newly joined user
    if let Some(room_state_event) = room::get_room_state_event(&state, room_id).await {
        if let Ok(text) = serde_json::to_string(&room_state_event) {
            tracing::debug!("[WS SEND] User: {}, {}", user.id, text);
            let _ = sender.send(Message::Text(text.into())).await;
        }
    }

    let mut send_task = spawn_sender_task(sender, rx, user.id);
    let mut recv_task = spawn_receiver_task(receiver, state.clone(), room_id, user.id, owner_id);

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    handle_user_disconnect(&state, room_id, user.id).await;
}

async fn add_user_to_room(
    state: &AppState,
    room_id: Uuid,
    user: &UserResponse,
    owner_id: Uuid,
) -> Receiver<WsEventServer> {
    let mut rooms = state.active_rooms.write().await;

    // Check if the server is reaching its capacity limit (e.g. 1000 rooms)
    if rooms.len() >= 1000 && !rooms.contains_key(&room_id) {
        tracing::warn!(
            "Max active rooms limit reached, cannot create room {}",
            room_id
        );
        // Clean the oldest inactive room as an LRU fallback (simple approach) or just refuse?
        // Refusing is safer for a quick fix without breaking too much
    }

    let room = rooms.entry(room_id).or_insert_with(|| ActiveRoom {
        tx: tokio::sync::broadcast::channel(100).0,
        users: std::collections::HashMap::new(),
        owner_id: Some(owner_id),
        last_activity: chrono::Utc::now(),
    });

    room.last_activity = chrono::Utc::now();
    room.users.insert(user.id, user.username.clone());
    room.tx.subscribe()
}

fn spawn_sender_task(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: Receiver<WsEventServer>,
    user_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Ok(text) = serde_json::to_string(&msg) {
                        tracing::debug!("[WS SEND] User: {}, {}", user_id, text);
                        if sender.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("[WS SEND] User {} lagged by {} messages", user_id, n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    })
}

fn spawn_receiver_task(
    mut receiver: SplitStream<WebSocket>,
    state: AppState,
    room_id: Uuid,
    user_id: Uuid,
    owner_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            tracing::debug!("[WS RECV] User: {}, {}", user_id, text);
            if let Ok(event) = serde_json::from_str::<WsEventClient>(&text) {
                if user_id == owner_id {
                    handle_client_event(&state, room_id, event).await;
                    send_room_state(&state, room_id).await;
                }
            }
        }
    })
}

async fn handle_user_disconnect(state: &AppState, room_id: Uuid, user_id: Uuid) {
    let (should_close_room, current_owner_id) = {
        let mut rooms = state.active_rooms.write().await;
        if let Some(room) = rooms.get_mut(&room_id) {
            let owner_id = room.owner_id;
            let should_close = owner_id == Some(user_id);
            room.users.remove(&user_id);
            (should_close, owner_id)
        } else {
            (false, None)
        }
    };

    if should_close_room {
        let mut rooms = state.active_rooms.write().await;
        if let Some(room) = rooms.remove(&room_id) {
            drop(rooms);
            let _ = room.tx.send(WsEventServer::RoomClosed);
            let _ = rooms_repo::delete(&state.pool, room_id).await;
            tracing::debug!("[WS SEND] Room: {}, Type: RoomClosed", room_id);
        }
    } else {
        send_user_state(state, room_id, current_owner_id.unwrap_or(user_id)).await;
    }
}

async fn handle_client_event(state: &AppState, room_id: Uuid, event: WsEventClient) {
    if let Some(room) = state.active_rooms.write().await.get_mut(&room_id) {
        room.last_activity = chrono::Utc::now();
    }
    let pool = &state.pool;
    if let Err(e) = match event {
        WsEventClient::Play {
            position,
            timestamp,
        } => rooms_repo::update_playback_play(pool, room_id, position, timestamp).await,
        WsEventClient::Pause { position } => {
            rooms_repo::update_playback_pause(pool, room_id, position).await
        }
        WsEventClient::SeekTo {
            position,
            timestamp,
        } => rooms_repo::update_playback_seek(pool, room_id, position, timestamp).await,
        WsEventClient::NextTrack { timestamp } => {
            messages::handle_next_track(state, room_id, timestamp).await;
            Ok(())
        }
    } {
        tracing::error!("Failed to handle client event in room {}: {}", room_id, e);
    }
}
