use crate::errors::{AppError, ErrorMessage};
use crate::state::AppState;
use axum::extract::State;
use axum::{extract::Request, middleware::Next, response::Response};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    // Try to get token from Authorization header
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        // If not in header, try to get from query params
        .or_else(|| {
            req.uri().query().and_then(|q| {
                q.split('&').find_map(|param| {
                    let mut parts = param.split('=');
                    if parts.next() == Some("token") {
                        parts.next().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
        })
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
