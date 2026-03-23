use axum::{
	Extension, Json,
	extract::{Path, State},
	http::StatusCode,
};
use uuid::Uuid;

use crate::{
	dtos::rooms::{RoomResponse, TransferOwnershipRequest},
	errors::AppError,
	middleware::auth::Claims,
	services::rooms as room_service,
	state::AppState,
};

pub async fn create_room(State(state): State<AppState>, Extension(claims): Extension<Claims>) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
	let room = room_service::create_room(&state.pool, claims.user_id).await?;
	Ok((StatusCode::CREATED, Json(RoomResponse { id: room.id, owner_id: room.owner_id })))
}

pub async fn delete_room(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>) -> Result<StatusCode, AppError> {
	room_service::delete_room(&state.pool, room_id, claims.user_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn get_room(State(state): State<AppState>, Path(room_id): Path<Uuid>) -> Result<Json<RoomResponse>, AppError> {
	let room = room_service::get_room(&state.pool, room_id).await?;
	Ok(Json(RoomResponse { id: room.id, owner_id: room.owner_id })) // Map more fields later?
}

pub async fn transfer_ownership(State(state): State<AppState>, Path(room_id): Path<Uuid>, Extension(claims): Extension<Claims>, Json(payload): Json<TransferOwnershipRequest>) -> Result<StatusCode, AppError> {
	room_service::transfer_ownership(&state.pool, room_id, claims.user_id, payload.new_owner_id).await?;
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

use crate::dtos::ws::WsEvent;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::{SinkExt, StreamExt};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>, Path(room_id): Path<Uuid>) -> axum::response::Response {
	ws.on_upgrade(move |socket| handle_socket(socket, state, room_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, room_id: Uuid) {
	let (mut sender, mut receiver) = socket.split();

	// Find or create the room broadcast channel
	let tx = {
		let mut channels = state.room_channels.write().await;
		channels.entry(room_id).or_insert_with(|| tokio::sync::broadcast::channel(100).0).clone()
	};

	let mut rx = tx.subscribe();

	// Task to receive messages from the user and broadcast to others
	let tx_clone = tx.clone();
	let mut recv_task = tokio::spawn(async move {
		while let Some(Ok(Message::Text(text))) = receiver.next().await {
			if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
				let _ = tx_clone.send(event); // broadcast it
			}
		}
	});

	// Task to send messages from other users back to this user
	let mut send_task = tokio::spawn(async move {
		while let Ok(msg) = rx.recv().await {
			if let Ok(text) = serde_json::to_string(&msg) {
				if sender.send(Message::Text(text.into())).await.is_err() {
					break;
				}
			}
		}
	});

	// If any task exits, abort the other.
	tokio::select! {
	_ = (&mut recv_task) => send_task.abort(),
	_ = (&mut send_task) => recv_task.abort(),
	};
}
