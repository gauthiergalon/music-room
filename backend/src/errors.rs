use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
	#[error("Not Found")]
	NotFound(ErrorMessage),

	#[error("Unauthorized")]
	Unauthorized(ErrorMessage),

	#[error("Forbidden")]
	Forbidden(ErrorMessage),

	#[error("Conflict")]
	Conflict(ErrorMessage),

	#[error("Too Many Requests")]
	TooManyRequests(ErrorMessage),

	#[error("Validation Error")]
	Validation(Vec<ErrorMessage>),

	#[error("Database Error")]
	Database(#[from] sqlx::Error),

	#[error("Internal Server Error")]
	Internal,
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let (status, details): (StatusCode, Option<serde_json::Value>) = match &self {
			AppError::NotFound(msg) => (StatusCode::NOT_FOUND, Some(json!([msg.to_string()]))),
			AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, Some(json!([msg.to_string()]))),
			AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, Some(json!([msg.to_string()]))),
			AppError::Conflict(msg) => (StatusCode::CONFLICT, Some(json!([msg.to_string()]))),
			AppError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, Some(json!([msg.to_string()]))),
			AppError::Validation(msgs) => {
				let string_msgs: Vec<String> = msgs.iter().map(|m| m.to_string()).collect();
				(StatusCode::UNPROCESSABLE_ENTITY, Some(json!(string_msgs)))
			}
			AppError::Database(e) => {
				tracing::error!("DB error: {e}");
				(StatusCode::INTERNAL_SERVER_ERROR, None)
			}
			AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, None),
		};

		(status, Json(json!({ "error": self.to_string(), "details": details }))).into_response()
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
	TooManyEmails,

	// Validation
	UsernameInvalidLength,
	EmailInvalidFormat,
	PasswordInvalidPolicy,
	PasswordSameAsCurrent,
	TrackIdInvalid,

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
			Self::TooManyEmails => "An email was already sent recently, please check your inbox or try again later",

			// Validation
			Self::UsernameInvalidLength => "Username has invalid length (must be between 3 and 32 characters)",
			Self::EmailInvalidFormat => "Invalid email address",
			Self::PasswordInvalidPolicy => "Password does not meet the required policy (must be at least 8 characters)",
			Self::PasswordSameAsCurrent => "New password must be different from current password",
			Self::TrackIdInvalid => "Track ID must be a positive integer",

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
