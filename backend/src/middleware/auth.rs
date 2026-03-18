use crate::errors::{AppError, ErrorMessage};
use axum::{Json, extract::Request, middleware::Next, response::Response};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
	pub user_id: Uuid,
	pub exp: usize,
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, AppError> {
	let token = req.headers().get("Authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

	let claims = decode::<Claims>(token, &DecodingKey::from_secret(get_secret().as_bytes()), &Validation::default()).map_err(|_| AppError::Unauthorized(ErrorMessage::TokenInvalid))?.claims;

	req.extensions_mut().insert(claims);

	Ok(next.run(req).await)
}

pub fn generate_access_token(user_id: Uuid) -> Result<String, AppError> {
	let exp = (chrono::Utc::now() + chrono::Duration::minutes(15)).timestamp() as usize;
	let claims = Claims { user_id, exp };

	encode(&Header::default(), &claims, &EncodingKey::from_secret(get_secret().as_bytes())).map_err(|_| AppError::Internal)
}

fn get_secret() -> String {
	std::env::var("JWT_SECRET").expect("JWT_SECRET must be set")
}
