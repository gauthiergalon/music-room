use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::sync::broadcast::{Receiver, error::RecvError};
use uuid::Uuid;

use crate::{
    dtos::ws::{WsEventClient, WsEventServer},
    repositories::rooms as rooms_repo,
    services::rooms as room_service,
    state::AppState,
    ws::{manager::JoinRequest, room},
};

/// The server pings every connection so dead peers are noticed even when they
/// never send anything (listeners only receive).
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// A peer that stays silent this long (no message, no pong) is considered gone.
const IDLE_TIMEOUT: Duration = Duration::from_secs(75);

pub struct WsUser {
    pub id: Uuid,
    pub username: String,
    pub device_name: String,
}

pub async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    room_id: Uuid,
    owner_id: Uuid,
    user: WsUser,
) {
    let connection_id = Uuid::new_v4();
    let user_id = user.id;
    let (mut sender, receiver) = socket.split();

    let rx = match state
        .rooms
        .join(JoinRequest {
            room_id,
            owner_id,
            user_id,
            connection_id,
            username: user.username,
            device_name: user.device_name,
        })
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            let _ = send_event(
                &mut sender,
                &WsEventServer::Error {
                    message: e.to_string(),
                },
            )
            .await;
            let _ = sender.close().await;
            return;
        }
    };

    if user_id == owner_id {
        let _ = rooms_repo::touch(&state.pool, room_id).await;
    }

    // Tell the client who it is, then give it the current state of the room.
    // The user list was already queued for it by `join`.
    let welcome = WsEventServer::Welcome {
        connection_id,
        server_time: Utc::now(),
    };
    let mut connected = send_event(&mut sender, &welcome).await;
    if connected && let Some(room_state) = room::get_room_state_event(&state, room_id).await {
        connected = send_event(&mut sender, &room_state).await;
    }

    if connected {
        let mut send_task = spawn_sender_task(sender, rx, state.clone(), room_id, user_id);
        let mut recv_task =
            spawn_receiver_task(receiver, state.clone(), room_id, user_id, connection_id);

        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        };
    }

    handle_disconnect(&state, room_id, owner_id, user_id, connection_id).await;
}

/// Serialises and sends one event. Returns `false` when the socket is gone.
async fn send_event(sender: &mut SplitSink<WebSocket, Message>, event: &WsEventServer) -> bool {
    match serde_json::to_string(event) {
        Ok(text) => sender.send(Message::Text(text.into())).await.is_ok(),
        Err(e) => {
            tracing::error!("Failed to serialise a WS event: {}", e);
            true
        }
    }
}

fn spawn_sender_task(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: Receiver<WsEventServer>,
    state: AppState,
    room_id: Uuid,
    user_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ping = tokio::time::interval(PING_INTERVAL);
        ping.tick().await; // the first tick completes immediately

        loop {
            tokio::select! {
                received = rx.recv() => match received {
                    Ok(event) => {
                        tracing::debug!("[WS SEND] User: {}, {:?}", user_id, event);
                        if !send_event(&mut sender, &event).await {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(missed)) => {
                        // This client missed events: give it a full, fresh picture.
                        tracing::warn!("[WS SEND] User {} lagged by {} messages, resyncing", user_id, missed);
                        if let Some(users) = state.rooms.user_state(room_id).await
                            && !send_event(&mut sender, &users).await
                        {
                            break;
                        }
                        if let Some(room_state) = room::get_room_state_event(&state, room_id).await
                            && !send_event(&mut sender, &room_state).await
                        {
                            break;
                        }
                    }
                    // The room was closed; `RoomClosed` has already been forwarded.
                    Err(RecvError::Closed) => break,
                },
                _ = ping.tick() => {
                    if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
            }
        }

        let _ = sender.close().await;
    })
}

fn spawn_receiver_task(
    mut receiver: SplitStream<WebSocket>,
    state: AppState,
    room_id: Uuid,
    user_id: Uuid,
    connection_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let message = match tokio::time::timeout(IDLE_TIMEOUT, receiver.next()).await {
                Ok(Some(Ok(message))) => message,
                // Stream ended, protocol error, or the peer went silent.
                _ => break,
            };

            match message {
                Message::Text(text) => {
                    tracing::debug!("[WS RECV] User: {}, {}", user_id, text);
                    let Ok(event) = serde_json::from_str::<WsEventClient>(&text) else {
                        tracing::debug!("[WS RECV] User {} sent an unknown event", user_id);
                        continue;
                    };
                    // Only the device in control may change playback.
                    if state
                        .rooms
                        .can_control(room_id, user_id, connection_id)
                        .await
                    {
                        handle_client_event(&state, room_id, event).await;
                    }
                }
                Message::Close(_) => break,
                // Pings are answered by the WebSocket layer; pongs only prove liveness.
                Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => {}
            }
        }
    })
}

async fn handle_disconnect(
    state: &AppState,
    room_id: Uuid,
    owner_id: Uuid,
    user_id: Uuid,
    connection_id: Uuid,
) {
    // Refresh the room's activity *before* the owner is marked as gone: the
    // cleanup task deletes rooms that are both stale and ownerless, and it
    // must never see that combination during the grace period.
    if user_id == owner_id {
        let _ = rooms_repo::touch(&state.pool, room_id).await;
    }

    let Some(outcome) = state.rooms.leave(room_id, user_id, connection_id).await else {
        return;
    };
    let Some(epoch) = outcome.owner_left_epoch else {
        return;
    };

    // The owner has no device connected any more. Keep the room for a grace
    // period so a network blip or an app restart does not destroy it.
    let state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(state.rooms.owner_grace()).await;
        if state.rooms.owner_still_absent(room_id, epoch).await {
            tracing::info!("Owner did not come back, closing room {}", room_id);
            if let Err(e) = room_service::close(&state, room_id).await {
                tracing::error!("Failed to close room {}: {:?}", room_id, e);
            }
        }
    });
}

async fn handle_client_event(state: &AppState, room_id: Uuid, event: WsEventClient) {
    state.rooms.touch(room_id).await;
    let pool = &state.pool;

    let result = match event {
        WsEventClient::Play { position } => {
            rooms_repo::update_playback_play(pool, room_id, position.max(0)).await
        }
        WsEventClient::Pause { position } => {
            rooms_repo::update_playback_pause(pool, room_id, position.max(0)).await
        }
        WsEventClient::SeekTo { position } => {
            rooms_repo::update_playback_seek(pool, room_id, position.max(0)).await
        }
        WsEventClient::NextTrack { item_id } => {
            match rooms_repo::advance_track(pool, room_id, item_id).await {
                // A stale or duplicated request: nothing changed, nothing to announce.
                Ok(false) => return,
                other => other.map(|_| ()),
            }
        }
    };

    match result {
        Ok(()) => room::send_room_state(state, room_id).await,
        Err(e) => tracing::error!("Failed to handle client event in room {}: {:?}", room_id, e),
    }
}
