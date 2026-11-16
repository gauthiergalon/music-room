use axum::{
    Extension, Json,
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    dtos::rooms::{DelegateRoomRequest, ListRoomsQuery, RoomResponse},
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    services::{rooms as room_service, user as user_service},
    state::AppState,
    ws::{WsUser, handle_socket},
};

const DEFAULT_PAGE_SIZE: i64 = 50;
const MAX_PAGE_SIZE: i64 = 100;
const MAX_DEVICE_NAME_CHARS: usize = 64;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub device: Option<String>,
}

/// Rooms the caller may join: public rooms, their own, and private rooms they
/// accepted an invitation to.
#[utoipa::path(get, path = "/rooms", params(ListRoomsQuery), responses((status = 200, body = [RoomResponse])), tag = "Rooms")]
pub async fn list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListRoomsQuery>,
) -> Result<Json<Vec<RoomResponse>>, AppError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);

    let rooms = room_service::list(&state.pool, claims.user_id, limit, offset).await?;
    Ok(Json(rooms.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/rooms", responses((status = 201, body = RoomResponse)), tag = "Rooms")]
pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
    let room = room_service::create(&state, claims.user_id).await?;
    Ok((StatusCode::CREATED, Json(room.into())))
}

#[utoipa::path(get, path = "/rooms/{id}", responses((status = 200, body = RoomResponse), (status = 403), (status = 404)), tag = "Rooms")]
pub async fn get(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<RoomResponse>, AppError> {
    let room = room_service::get(&state.pool, room_id, claims.user_id).await?;
    Ok(Json(room.into()))
}

/// Closes the room: connected clients receive `RoomClosed`.
#[utoipa::path(delete, path = "/rooms/{id}", responses((status = 204), (status = 403), (status = 404)), tag = "Rooms")]
pub async fn delete(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::delete(&state, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/rooms/{id}/enable-license", responses((status = 204), (status = 403), (status = 404)), tag = "Rooms")]
pub async fn enable_license(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::enable_license(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/rooms/{id}/disable-license", responses((status = 204), (status = 403), (status = 404)), tag = "Rooms")]
pub async fn disable_license(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::disable_license(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Hands playback control to one connected device of another user.
#[utoipa::path(
    post,
    path = "/rooms/{id}/delegate",
    request_body = DelegateRoomRequest,
    responses((status = 204), (status = 403), (status = 404), (status = 422)),
    tag = "Rooms"
)]
pub async fn delegate_room(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DelegateRoomRequest>,
) -> Result<StatusCode, AppError> {
    room_service::delegate(
        &state,
        room_id,
        claims.user_id,
        payload.user_id,
        payload.connection_id,
    )
    .await?;
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
    room_service::revoke_delegate(&state, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/rooms/{id}/publish", responses((status = 204), (status = 403), (status = 404)), tag = "Rooms")]
pub async fn publish(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::publish(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Makes the room invitation-only. Requires a premium subscription.
#[utoipa::path(post, path = "/rooms/{id}/privatize", responses((status = 204), (status = 403), (status = 404)), tag = "Rooms")]
pub async fn privatize(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    room_service::privatize(&state.pool, room_id, claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Opens the room's WebSocket. Same access rule as reading the room over HTTP.
pub async fn ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
) -> Result<axum::response::Response, AppError> {
    let room = room_service::get(&state.pool, room_id, claims.user_id).await?;

    if !state.rooms.has_capacity_for(room_id).await {
        return Err(AppError::ServiceUnavailable(ErrorMessage::TooManyRooms));
    }

    // The username in the token can be up to 15 minutes old; show the current one.
    let user = user_service::get_user(&state.pool, claims.user_id).await?;

    let device_name: String = headers
        .get("x-device")
        .and_then(|value| value.to_str().ok())
        .or(query.device.as_deref())
        .unwrap_or("Unknown device")
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_DEVICE_NAME_CHARS)
        .collect();

    let ws_user = WsUser {
        id: user.id,
        username: user.username,
        device_name,
    };
    let owner_id = room.owner_id;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, room_id, owner_id, ws_user)))
}
