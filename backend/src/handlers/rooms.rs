use axum::{
    Extension, Json,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    dtos::{
        rooms::{RoomResponse, TransferOwnershipRequest},
        user::UserResponse,
        ws::{UserInfo, WsEventClient, WsEventServer},
    },
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    models::user::PrivacyLevel,
    services::cleanup::cleanup_user_rooms,
    services::invitations as invitation_service,
    services::rooms as room_service,
    state::AppState,
    ws::{handle_socket, send_room_state},
};

#[utoipa::path(get, path = "/rooms", responses((status = 200, body = [RoomResponse])), tag = "Rooms")]
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

#[utoipa::path(post, path = "/rooms", responses((status = 201, body = RoomResponse)), tag = "Rooms")]
pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
    let _ = cleanup_user_rooms(&state.pool, claims.user_id).await;

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
        let users: Vec<UserInfo> = tx
            .users
            .iter()
            .map(|(id, name)| UserInfo {
                user_id: *id,
                username: name.clone(),
            })
            .collect();
        let _ = tx.tx.send(WsEventServer::UserState {
            user_list: users,
            owner: payload.new_owner_id,
        });
        tracing::info!(
            "[WS USER STATE] Room: {}, Ownership transferred to {}",
            room_id,
            payload.new_owner_id
        );
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
    let owner_id = sqlx::query!("SELECT owner_id FROM rooms WHERE id = $1", room_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.owner_id)
        .unwrap_or(claims.user_id);

    let user_info = UserResponse {
        id: claims.user_id,
        email: "".to_string(),
        username: claims.username.clone(),
        email_confirmed: false,
        favorite_genres: vec![],
        privacy_level: PrivacyLevel::Public,
    };

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, room_id, user_info, owner_id)))
}
