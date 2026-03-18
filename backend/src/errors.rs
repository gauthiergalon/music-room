use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
	#[error("{0}")]
	NotFound(ErrorMessage),

	#[error("{0}")]
	Unauthorized(ErrorMessage),

	#[error("{0}")]
	Forbidden(ErrorMessage),

	#[error("{0}")]
	Conflict(ErrorMessage),

	#[error("{0}")]
	Validation(ErrorMessage),

	#[error("Internal server error")]
	Internal,

	#[error("Database error")]
	Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let (status, message, details): (StatusCode, String, Option<serde_json::Value>) = match &self {
			AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string(), None),
			AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string(), None),
			AppError::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string(), None),
			AppError::Conflict(_) => (StatusCode::CONFLICT, self.to_string(), None),
			AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string(), None),
			AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string(), None),
			AppError::Database(e) => {
				tracing::error!("DB error: {e}");
				(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into(), None)
			}
		};

		(status, Json(json!({ "error": message, "details": details }))).into_response()
	}
}

#[derive(Debug)]
pub enum ErrorMessage {
	// Auth
	InvalidCredentials,
	EmailTaken,
	UsernameTaken,
	TokenExpired,
	TokenInvalid,

	// Validation
	UsernameInvalidLength,
	EmailInvalidFormat,
	PasswordInvalidPolicy,
	PasswordSameAsCurrent,

	// Ressources
	UserNotFound,
	RoomNotFound,

	// Droits
	NotRoomOwner,

	// Serveur
	InternalError,
}

impl ErrorMessage {
	pub fn as_str(&self) -> &'static str {
		match self {
			// Auth
			Self::InvalidCredentials => "Invalid email or password",
			Self::EmailTaken => "Email already in use",
			Self::UsernameTaken => "Username already taken",
			Self::TokenExpired => "Token expired",
			Self::TokenInvalid => "Invalid token",

			// Validation
			Self::UsernameInvalidLength => "Username has invalid length (must be between 3 and 32 characters)",
			Self::EmailInvalidFormat => "Invalid email address",
			Self::PasswordInvalidPolicy => "Password does not meet the required policy (must be at least 8 characters)",
			Self::PasswordSameAsCurrent => "New password must be different from current password",

			// Resources
			Self::UserNotFound => "User not found",
			Self::RoomNotFound => "Room not found",

			// Permissions
			Self::NotRoomOwner => "You are not the owner of this room",

			// Server
			Self::InternalError => "Internal server error",
		}
	}
}

impl std::fmt::Display for ErrorMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}
