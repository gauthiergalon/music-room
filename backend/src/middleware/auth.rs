use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    state::AppState,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub username: String,
    pub user_id: Uuid,
    pub exp: usize,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(&req)
        .or_else(|| websocket_query_token(&req))
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    let jwt_secret = &state.jwt_secret;

    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    if token_data.claims.username.is_empty() || token_data.claims.user_id.is_nil() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenInvalid));
    }

    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}

/// The access token of the `Authorization: Bearer ...` header.
fn bearer_token(req: &Request) -> Option<String> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string)
}

/// The access token of the `?token=` query parameter, accepted on WebSocket
/// upgrades only: a browser cannot set headers on those. Anywhere else a
/// token in the URL would end up in histories and proxy logs for no reason.
fn websocket_query_token(req: &Request) -> Option<String> {
    let is_websocket_upgrade = req
        .headers()
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
    if !is_websocket_upgrade {
        return None;
    }

    req.uri().query()?.split('&').find_map(|param| {
        param
            .split_once('=')
            .filter(|(name, _)| *name == "token")
            .map(|(_, value)| value.to_string())
    })
}
