use crate::{
    dtos::auth::{
        AuthResponse, ForgotPasswordRequest, GoogleLoginRequest, LoginRequest, LogoutRequest,
        RefreshRequest, RegisterRequest, ResetPasswordRequest,
    },
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    services::auth as auth_service,
    state::AppState,
};
use axum::{Extension, Json, extract::State, http::StatusCode};

#[utoipa::path(post, path = "/auth/register", request_body = RegisterRequest, responses((status = 201, body = AuthResponse)), tag = "Auth")]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let mut errors = Vec::new();

    if payload.username.len() < 3 || payload.username.len() > 24 {
        errors.push(ErrorMessage::UsernameInvalidLength);
    }
    if !validator::ValidateEmail::validate_email(&payload.email) {
        errors.push(ErrorMessage::EmailInvalidFormat);
    }
    if payload.password.len() < 8 || payload.password.len() > 128 {
        errors.push(ErrorMessage::PasswordInvalidPolicy);
    }

    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let (access_token, refresh_token) = auth_service::register(
        &state.pool,
        &state.jwt_secret,
        &payload.username,
        &payload.email,
        &payload.password,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            access_token,
            refresh_token,
        }),
    ))
}

#[utoipa::path(post, path = "/auth/login", request_body = LoginRequest, responses((status = 200, body = AuthResponse)), tag = "Auth")]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if !validator::ValidateEmail::validate_email(&payload.email) {
        return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
    }

    let (access_token, refresh_token) = auth_service::login(
        &state.pool,
        &state.jwt_secret,
        &payload.email,
        &payload.password,
    )
    .await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
    }))
}

#[utoipa::path(post, path = "/auth/logout", request_body = LogoutRequest, responses((status = 204)), tag = "Auth")]
pub async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    auth_service::logout(&state.pool, &payload.refresh_token, &claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/auth/refresh", request_body = RefreshRequest, responses((status = 200, body = AuthResponse)), tag = "Auth")]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (access_token, refresh_token) =
        auth_service::refresh(&state.pool, &state.jwt_secret, &payload.refresh_token).await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
    }))
}

#[utoipa::path(post, path = "/auth/forgot-password", request_body = ForgotPasswordRequest, responses((status = 204)), tag = "Auth")]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<StatusCode, AppError> {
    if !validator::ValidateEmail::validate_email(&payload.email) {
        return Err(AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]));
    }

    let _ = auth_service::forgot_password(&state.pool, &payload.email).await;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/auth/reset-password", request_body = ResetPasswordRequest, responses((status = 204)), tag = "Auth")]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<StatusCode, AppError> {
    if payload.new_password.len() < 8 {
        return Err(AppError::Validation(vec![
            ErrorMessage::PasswordInvalidPolicy,
        ]));
    }

    auth_service::reset_password(&state.pool, &payload.token, &payload.new_password).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/auth/google-login", request_body = GoogleLoginRequest, responses((status = 200, body = AuthResponse)), tag = "Auth")]
pub async fn google_login(
    State(state): State<AppState>,
    Json(payload): Json<GoogleLoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (access_token, refresh_token) = auth_service::google_login(
        &state.pool,
        &state.jwt_secret,
        &state.google_client_id,
        &state.google_auth_url,
        &payload.id_token,
    )
    .await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
    }))
}
