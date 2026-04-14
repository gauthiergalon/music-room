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
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?
        .to_string();

    let jwt_secret = &state.jwt_secret;

    let claims = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized(ErrorMessage::TokenInvalid))?
    .claims;

    let user_exists = sqlx::query!("SELECT id FROM users WHERE id = $1", claims.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::Database)?;

    if user_exists.is_none() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenInvalid));
    }

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
