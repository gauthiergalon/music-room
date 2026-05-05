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

pub async fn invite(
    State(state): State<AppState>,
    Path((room_id, invitee_id)): Path<(Uuid, Uuid)>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<InvitationResponse>), AppError> {
    let invitation =
        invitation_service::invite(&state.pool, room_id, claims.user_id, invitee_id).await?;

    let response = InvitationResponse {
        id: invitation.id,
        room_id: invitation.room_id,
        inviter_id: invitation.inviter_id,
        invitee_id: invitation.invitee_id,
        is_pending: invitation.is_pending,
        created_at: invitation.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_pending(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<InvitationResponse>>, AppError> {
    let invitations =
        invitation_service::get_my_pending_invitations(&state.pool, claims.user_id).await?;

    let responses: Vec<InvitationResponse> = invitations
        .into_iter()
        .map(|i| InvitationResponse {
            id: i.id,
            room_id: i.room_id,
            inviter_id: i.inviter_id,
            invitee_id: i.invitee_id,
            is_pending: i.is_pending,
            created_at: i.created_at,
        })
        .collect();

    Ok(Json(responses))
}

pub async fn accept(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<InvitationResponse>, AppError> {
    let invitation = invitation_service::accept(&state.pool, invitation_id, claims.user_id).await?;

    let response = InvitationResponse {
        id: invitation.id,
        room_id: invitation.room_id,
        inviter_id: invitation.inviter_id,
        invitee_id: invitation.invitee_id,
        is_pending: invitation.is_pending,
        created_at: invitation.created_at,
    };

    Ok(Json(response))
}

pub async fn reject(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    invitation_service::reject(&state.pool, invitation_id, claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    invitation_service::revoke(&state.pool, invitation_id, claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
