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
		ws::WsEvent,
	},
	errors::AppError,
	middleware::auth::Claims,
	services::rooms as room_service,
	state::AppState,
};

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<RoomResponse>>, AppError> {
    let rooms = room_service::list(&state.pool).await?;
    let responses = rooms.into_iter().map(|room| RoomResponse {
        id: room.id, owner_id: room.owner_id, name: room.name,
        is_public: room.is_public, current_track: room.current_track,
        current_position: room.current_position, is_playing: room.is_playing
    }).collect();
    Ok(Json(responses))
}

pub async fn create(State(state): State<AppState>, Extension(claims): Extension<Claims>) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
        let name = format!("{}'s room", claims.username);
        let room = room_service::create(&state.pool, claims.user_id, &name).await?;
        Ok((StatusCode::CREATED, Json(RoomResponse { 
            id: room.id, owner_id: room.owner_id, name: room.name, 
            is_public: room.is_public, current_track: room.current_track,
            current_position: room.current_position, is_playing: room.is_playing
        })))
}

pub async fn delete(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	room_service::delete(&state.pool, room_id, claims.user_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn get(State(state): State<AppState>, Path(room_id): Path<Uuid>) -> Result<Json<RoomResponse>, AppError> {
        let room = room_service::get(&state.pool, room_id).await?;
        Ok(Json(RoomResponse { 
            id: room.id, owner_id: room.owner_id, name: room.name, 
            is_public: room.is_public, current_track: room.current_track,
            current_position: room.current_position, is_playing: room.is_playing
        }))
}

pub async fn transfer_ownership(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<TransferOwnershipRequest>) -> Result<StatusCode, AppError> {
	room_service::transfer_ownership(&state.pool, room_id, claims.user_id, payload.new_owner_id).await?;

	if let Some(tx) = state.active_rooms.read().await.get(&room_id) {
		let _ = tx.tx.send(WsEvent::UserOwnershipTransferred { new_owner_id: payload.new_owner_id });
	}

	Ok(StatusCode::NO_CONTENT)
}

pub async fn publish(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	room_service::publish(&state.pool, room_id, claims.user_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn privatize(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	room_service::privatize(&state.pool, room_id, claims.user_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn ws(ws: WebSocketUpgrade, State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<axum::response::Response, AppError> {
	let room = room_service::get(&state.pool, room_id).await?;
	let is_owner = room.owner_id == claims.user_id;

	Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, room_id, is_owner, claims.user_id, claims.username)))
}

async fn handle_socket(socket: WebSocket, state: AppState, room_id: Uuid, is_owner: bool, user_id: Uuid, username: String) {
	let (mut sender, mut receiver) = socket.split();

	let (tx, users_list) = {
                let mut channels = state.active_rooms.write().await;
                let room = channels.entry(room_id).or_insert_with(|| crate::state::ActiveRoom {
                        tx: tokio::sync::broadcast::channel(100).0,
                        users: std::collections::HashMap::new(),
                });
                room.users.insert(user_id, username.clone());
                let list: Vec<crate::dtos::ws::UserInfo> = room.users.iter().map(|(id, name)| crate::dtos::ws::UserInfo { user_id: *id, username: name.clone() }).collect();
                (room.tx.clone(), list)
        };
        if let Ok(sync_msg) = serde_json::to_string(&WsEvent::SyncUsers { users: users_list }) {
                let _ = sender.send(Message::Text(sync_msg.into())).await;
        }

	let _ = tx.send(WsEvent::UserJoin { user_id, username: username.clone() });

	let tx_clone = tx.clone();
	let mut rx = tx.subscribe();
	let pool = state.pool.clone();

	let mut recv_task = tokio::spawn(async move {
		while let Some(Ok(Message::Text(text))) = receiver.next().await {
			if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
				let is_authorized = match &event {
					WsEvent::Play { .. } | WsEvent::Pause { .. } | WsEvent::SeekTo { .. } | WsEvent::NextTrack { .. } => is_owner,
					WsEvent::QueueAdd { .. } | WsEvent::QueueRemove { .. } | WsEvent::QueueReorder { .. } => true,
					_ => false,
				};

				if is_authorized {
					let update_result = match &event {
						WsEvent::Play { timestamp, position } => sqlx::query!("UPDATE rooms SET is_playing = true, played_at = $1, current_position = $2 WHERE id = $3", timestamp, position, room_id).execute(&pool).await,
						WsEvent::Pause { position } => sqlx::query!("UPDATE rooms SET is_playing = false, current_position = $1 WHERE id = $2", position, room_id).execute(&pool).await,
						WsEvent::SeekTo { position, timestamp } => sqlx::query!("UPDATE rooms SET current_position = $1, played_at = $2 WHERE id = $3", position, timestamp, room_id).execute(&pool).await,
						WsEvent::NextTrack { timestamp } => {
							let next_in_queue = sqlx::query!(
								"DELETE FROM queue WHERE id = (
									SELECT id FROM queue WHERE room_id = $1 ORDER BY position ASC LIMIT 1
								) RETURNING track_id",
								room_id
							)
							.fetch_optional(&pool)
							.await
							.unwrap_or(None);

							if let Some(next) = next_in_queue { sqlx::query!("UPDATE rooms SET current_track = $1, played_at = $2, current_position = 0, is_playing = true WHERE id = $3", next.track_id, timestamp, room_id).execute(&pool).await } else { sqlx::query!("UPDATE rooms SET is_playing = false, current_track = NULL WHERE id = $1", room_id).execute(&pool).await }
						}
						_ => Ok(sqlx::postgres::PgQueryResult::default()),
					};

					if let Err(e) = update_result {
						tracing::error!("Failed to update room state in db: {}", e);
					} else {
						let _ = tx_clone.send(event);
					}
				}
			} else {
				let error_event = WsEvent::Error { message: "Invalid event format".to_string() };
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
        {
                let mut channels = state.active_rooms.write().await;
                if let std::collections::hash_map::Entry::Occupied(mut entry) = channels.entry(room_id) {
                        entry.get_mut().users.remove(&user_id);
                        if entry.get().users.is_empty() {
                                is_empty = true;
                                entry.remove();
                        }
                }
        }
        let _ = tx.send(WsEvent::UserLeave { user_id, username });

        if is_empty {
                let _ = sqlx::query!("DELETE FROM rooms WHERE id = $1", room_id).execute(&state.pool).await;
                let _ = tx.send(WsEvent::RoomClosed);
        }
}
