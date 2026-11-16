use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    dtos::friend::{FriendRequestDto, FriendResponseDto},
    errors::AppError,
    middleware::auth::Claims,
    services::friends as friends_service,
    state::AppState,
};

#[utoipa::path(get, path = "/friends", responses((status = 200, body = [FriendResponseDto])), tag = "Friends")]
pub async fn list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<FriendResponseDto>>, AppError> {
    let friends = friends_service::list_friends(&state.pool, claims.user_id).await?;
    Ok(Json(friends))
}

#[utoipa::path(post, path = "/friends", request_body = FriendRequestDto, responses((status = 201, body = FriendResponseDto)), tag = "Friends")]
pub async fn send_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<FriendRequestDto>,
) -> Result<(StatusCode, Json<FriendResponseDto>), AppError> {
    let friend =
        friends_service::send_request(&state.pool, claims.user_id, &payload.username).await?;
    Ok((StatusCode::CREATED, Json(friend)))
}

#[utoipa::path(post, path = "/friends/{friend_id}/accept", responses((status = 200, body = FriendResponseDto)), tag = "Friends")]
pub async fn accept_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(friend_id): Path<Uuid>,
) -> Result<Json<FriendResponseDto>, AppError> {
    let friend = friends_service::accept_request(&state.pool, claims.user_id, friend_id).await?;
    Ok(Json(friend))
}

#[utoipa::path(delete, path = "/friends/{friend_id}/reject", responses((status = 204)), tag = "Friends")]
pub async fn reject_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(friend_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    friends_service::reject_request(&state.pool, claims.user_id, friend_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/friends/{friend_id}", responses((status = 204)), tag = "Friends")]
pub async fn remove(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(friend_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    friends_service::remove_friend(&state.pool, claims.user_id, friend_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
