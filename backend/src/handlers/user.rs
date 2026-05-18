use axum::{
    Extension, Json,
    extract::{State, Path},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    dtos::user::{
        ConfirmEmailRequest, PublicUserResponse, UpdateEmailRequest, UpdateFavoriteGenresRequest, UpdatePasswordRequest, UpdatePrivacyLevelRequest, UpdateUsernameRequest, UserResponse,
    },
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    models::user::PrivacyLevel,
    services::friends as friends_service,
    services::user as user_service,
    state::AppState,
};

#[utoipa::path(get, path = "/users/me", responses((status = 200, body = UserResponse)), tag = "Users")]
pub async fn get_me(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::get_me(&state.pool, claims.user_id).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(get, path = "/users/{id}", responses((status = 200, body = PublicUserResponse)), tag = "Users")]
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<PublicUserResponse>, AppError> {
    let user = user_service::get_user(&state.pool, user_id).await?;

    let show_genres = user_id == claims.user_id
        || match user.privacy_level {
            PrivacyLevel::Public => true,
            PrivacyLevel::Friends => {
                friends_service::are_friends(&state.pool, claims.user_id, user_id).await?
            }
            PrivacyLevel::Private => false,
        };

    Ok(Json(PublicUserResponse {
        id: user.id,
        username: user.username,
        favorite_genres: if show_genres {
            Some(user.favorite_genres)
        } else {
            None
        },
        privacy_level: user.privacy_level,
    }))
}

#[utoipa::path(patch, path = "/users/me/username", request_body = UpdateUsernameRequest, responses((status = 200, body = UserResponse)), tag = "Users")]
pub async fn update_username(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateUsernameRequest>,
) -> Result<Json<UserResponse>, AppError> {
    if payload.username.len() < 3 || payload.username.len() > 24 {
        return Err(AppError::Validation(vec![
            ErrorMessage::UsernameInvalidLength,
        ]));
    }

    let user =
        user_service::update_username(&state.pool, claims.user_id, &payload.username).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(patch, path = "/users/me/email", request_body = UpdateEmailRequest, responses((status = 200, body = UserResponse)), tag = "Users")]
pub async fn update_email(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateEmailRequest>,
) -> Result<Json<UserResponse>, AppError> {
    if !validator::ValidateEmail::validate_email(&payload.new_email) {
        return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
    }

    let user = user_service::update_email(&state.pool, claims.user_id, &payload.new_email).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(patch, path = "/users/me/password", request_body = UpdatePasswordRequest, responses((status = 204)), tag = "Users")]
pub async fn update_password(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdatePasswordRequest>,
) -> Result<StatusCode, AppError> {
    if payload.new_password.len() < 8 {
        return Err(AppError::Validation(vec![ ErrorMessage::PasswordInvalidPolicy ]));
    }

    user_service::update_password(&state.pool, claims.user_id, &payload.current_password, &payload.new_password).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(patch, path = "/users/me/confirm-email", request_body = ConfirmEmailRequest, responses((status = 204)), tag = "Users")]
pub async fn confirm_email(
    State(state): State<AppState>,
    Json(payload): Json<ConfirmEmailRequest>,
) -> Result<StatusCode, AppError> {
    user_service::confirm_email(&state.pool, &payload.token).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/users/me/send-confirmation-email", responses((status = 204)), tag = "Users")]
pub async fn send_email_confirmation_email(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    user_service::send_email_confirmation_email(&state.pool, claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(patch, path = "/users/me/favorite-genres", request_body = UpdateFavoriteGenresRequest, responses((status = 200, body = UserResponse)), tag = "Users")]
pub async fn update_favorite_genres(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateFavoriteGenresRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user =
        user_service::update_favorite_genres(&state.pool, claims.user_id, payload.favorite_genres)
            .await?;
    Ok(Json(user.into()))
}

#[utoipa::path(patch, path = "/users/me/privacy", request_body = UpdatePrivacyLevelRequest, responses((status = 200, body = UserResponse)), tag = "Users")]
pub async fn update_privacy_level(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdatePrivacyLevelRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user =
        user_service::update_privacy_level(&state.pool, claims.user_id, payload.privacy_level)
            .await?;
    Ok(Json(user.into()))
}
