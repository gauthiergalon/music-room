use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    dtos::invitation::InvitationResponse, errors::AppError, middleware::auth::Claims,
    services::invitations as invitation_service, state::AppState,
};

#[utoipa::path(post, path = "/rooms/{id}/invite/{invitee_id}", responses((status = 201, body = InvitationResponse)), tag = "Invitations")]
pub async fn invite(
    State(state): State<AppState>,
    Path((room_id, invitee_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<InvitationResponse>), AppError> {
    let invitation =
        invitation_service::invite(&state.pool, room_id, claims.user_id, invitee_id).await?;

    Ok((StatusCode::CREATED, Json(invitation.into())))
}

#[utoipa::path(get, path = "/me/invitations", responses((status = 200, body = [InvitationResponse])), tag = "Invitations")]
pub async fn list_pending(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<InvitationResponse>>, AppError> {
    let invitations =
        invitation_service::get_my_pending_invitations(&state.pool, claims.user_id).await?;

    Ok(Json(invitations.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/me/invitations/{id}/accept", responses((status = 204)), tag = "Invitations")]
pub async fn accept(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    invitation_service::accept(&state.pool, invitation_id, claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/me/invitations/{id}/reject", responses((status = 204)), tag = "Invitations")]
pub async fn reject(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    invitation_service::reject(&state.pool, invitation_id, claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/invitations/{id}/revoke", responses((status = 204)), tag = "Invitations")]
pub async fn revoke(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    invitation_service::revoke(&state.pool, invitation_id, claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
