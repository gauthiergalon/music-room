use axum::{
    Extension, Json,
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub device: Option<String>,
}

use crate::{
    dtos::{
        rooms::{DelegateRoomRequest, RoomResponse},
        user::UserResponse,
        ws::WsEventServer,
    },
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    models::user::PrivacyLevel,
    repositories::rooms as rooms_repo,
    services::{cleanup::cleanup_user_rooms, rooms as room_service},
    state::AppState,
    ws::handle_socket,
};

#[utoipa::path(get, path = "/rooms", responses((status = 200, body = [RoomResponse])), tag = "Rooms")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<RoomResponse>>, AppError> {
    let rooms = room_service::list(&state.pool).await?;
    let responses = rooms.into_iter().map(|r| r.into()).collect();
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
    Ok((StatusCode::CREATED, Json(room.into())))
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
    Ok(Json(room.into()))
}

#[utoipa::path(
    post,
    path = "/rooms/{id}/delegate",
    request_body = DelegateRoomRequest,
    responses((status = 204), (status = 403), (status = 404)),
    tag = "Rooms"
)]
pub async fn delegate_room(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DelegateRoomRequest>,
) -> Result<StatusCode, AppError> {
    let room = rooms_repo::find_by_id(&state.pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;
    room_service::check_is_owner(&room, claims.user_id)?;

    if payload.user_id == claims.user_id {
        return Err(AppError::Validation(vec![ErrorMessage::CannotDelegateToSelf]));
    }

    if payload.device_name.trim().is_empty() {
        return Err(AppError::Validation(vec![ErrorMessage::DeviceNameInvalid]));
    }

    let mut rooms = state.active_rooms.write().await;
    let active_room = rooms
        .get_mut(&room_id)
        .ok_or(AppError::NotFound(ErrorMessage::ActiveRoomNotFound))?;
    let target_key = crate::state::RoomMemberKey {
        user_id: payload.user_id,
        device_name: payload.device_name.clone(),
    };
    if !active_room.users.contains_key(&target_key) {
        return Err(AppError::NotFound(ErrorMessage::DeviceNotInRoom));
    }

    active_room.delegate = Some(crate::state::RoomDelegate {
        user_id: payload.user_id,
        device_name: payload.device_name.clone(),
    });
    let _ = active_room.tx.send(WsEventServer::UserState {
        user_list: active_room.user_list(),
        owner: active_room.owner_id.unwrap_or(claims.user_id),
        delegate_user_id: Some(payload.user_id),
        delegate_device: Some(payload.device_name.clone()),
    });

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/rooms/{id}/delegate",
    responses((status = 204), (status = 403), (status = 404)),
    tag = "Rooms"
)]
pub async fn revoke_delegate(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    let room = rooms_repo::find_by_id(&state.pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;
    room_service::check_is_owner(&room, claims.user_id)?;

    let mut rooms = state.active_rooms.write().await;
    let room = rooms
        .get_mut(&room_id)
        .ok_or(AppError::NotFound(ErrorMessage::ActiveRoomNotFound))?;
    if room.delegate.is_some() {
        room.delegate = None;
        let _ = room.tx.send(WsEventServer::UserState {
            user_list: room.user_list(),
            owner: room.owner_id.unwrap_or(claims.user_id),
            delegate_user_id: None,
            delegate_device: None,
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
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
) -> Result<axum::response::Response, AppError> {
    let room = rooms_repo::find_by_id(&state.pool, room_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::RoomNotFound))?;

    let owner_id = room.owner_id;

    let user_info = UserResponse {
        id: claims.user_id,
        email: "".to_string(),
        username: claims.username.clone(),
        email_confirmed: false,
        favorite_genres: vec![],
        privacy_level: PrivacyLevel::Public,
        is_subscribed: false,
        end_subscription_date: None,
    };

    let device_name = headers
        .get("x-device")
        .and_then(|value| value.to_str().ok())
        .or(query.device.as_deref())
        .unwrap_or("Unknown device")
        .to_string();

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(socket, state, room_id, user_info, owner_id, device_name)
    }))
}
