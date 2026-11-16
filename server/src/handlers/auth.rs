use axum::{Json, extract::State, http::StatusCode};

use crate::{
    dtos::auth::{
        AuthResponse, ForgotPasswordRequest, GoogleLoginRequest, LoginRequest, LogoutRequest,
        RefreshRequest, RegisterRequest, ResetPasswordRequest,
    },
    errors::AppError,
    services::auth as auth_service,
    state::AppState,
    validation,
};

#[utoipa::path(post, path = "/auth/register", request_body = RegisterRequest, responses((status = 201, body = AuthResponse), (status = 409), (status = 422), (status = 429)), security(()), tag = "Auth")]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let username = validation::username(&payload.username);
    let email = validation::email(&payload.email);
    let password = validation::password(&payload.password);

    // Report every invalid field at once.
    let (username, email) = match (username, email, password) {
        (Ok(username), Ok(email), Ok(())) => (username, email),
        (username, email, password) => {
            let errors = [username.err(), email.err(), password.err()]
                .into_iter()
                .flatten()
                .collect();
            return Err(AppError::Validation(errors));
        }
    };

    let tokens = auth_service::register(
        &state.pool,
        &state.jwt_secret,
        &username,
        &email,
        &payload.password,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(tokens.into())))
}

#[utoipa::path(post, path = "/auth/login", request_body = LoginRequest, responses((status = 200, body = AuthResponse), (status = 401), (status = 429)), security(()), tag = "Auth")]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = validation::email(&payload.email).map_err(validation::invalid)?;

    let tokens =
        auth_service::login(&state.pool, &state.jwt_secret, &email, &payload.password).await?;

    Ok(Json(tokens.into()))
}

/// Ends the session of the given refresh token. No access token is needed:
/// a client whose access token expired must still be able to log out.
#[utoipa::path(post, path = "/auth/logout", request_body = LogoutRequest, responses((status = 204)), security(()), tag = "Auth")]
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    auth_service::logout(&state.pool, &payload.refresh_token).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/auth/refresh", request_body = RefreshRequest, responses((status = 200, body = AuthResponse), (status = 401)), security(()), tag = "Auth")]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let tokens =
        auth_service::refresh(&state.pool, &state.jwt_secret, &payload.refresh_token).await?;

    Ok(Json(tokens.into()))
}

/// Sends a password reset link if the address belongs to an account.
///
/// The answer is always 204 and is sent before anything is looked up: the
/// work happens in the background, so neither the status nor the response
/// time tells whether an account exists for that address.
#[utoipa::path(post, path = "/auth/forgot-password", request_body = ForgotPasswordRequest, responses((status = 204), (status = 422), (status = 429)), security(()), tag = "Auth")]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<StatusCode, AppError> {
    let email = validation::email(&payload.email).map_err(validation::invalid)?;

    tokio::spawn(async move {
        if let Err(e) =
            auth_service::forgot_password(&state.pool, state.mailer.as_ref(), &email).await
        {
            tracing::info!("Password reset email not sent: {:?}", e);
        }
    });

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/auth/reset-password", request_body = ResetPasswordRequest, responses((status = 204), (status = 401), (status = 422)), security(()), tag = "Auth")]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<StatusCode, AppError> {
    validation::password(&payload.new_password).map_err(validation::invalid)?;

    auth_service::reset_password(&state.pool, &payload.token, &payload.new_password).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/auth/google-login", request_body = GoogleLoginRequest, responses((status = 200, body = AuthResponse), (status = 401), (status = 409), (status = 502)), security(()), tag = "Auth")]
pub async fn google_login(
    State(state): State<AppState>,
    Json(payload): Json<GoogleLoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let tokens = auth_service::google_login(
        &state.pool,
        &state.jwt_secret,
        &state.google,
        &payload.id_token,
    )
    .await?;

    Ok(Json(tokens.into()))
}
