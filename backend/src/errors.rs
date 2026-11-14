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

    /// An upstream service (music provider, Google, SMTP) failed or answered garbage.
    #[error("Bad Gateway")]
    BadGateway(ErrorMessage),

    #[error("Service Unavailable")]
    ServiceUnavailable(ErrorMessage),

    #[error("{0}")]
    InternalError(String),

    #[error("Database Error")]
    Database(#[from] sqlx::Error),

    #[error("Bad Gateway")]
    Request(#[from] reqwest::Error),

    #[error("Internal Server Error")]
    Internal,
}

impl AppError {
    /// True when the underlying database error is a violation of the given
    /// SQLSTATE class (`23505` unique, `23503` foreign key, ...).
    pub fn is_db_violation(error: &sqlx::Error, code: &str) -> bool {
        matches!(error, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some(code))
    }
}

pub const UNIQUE_VIOLATION: &str = "23505";
pub const FOREIGN_KEY_VIOLATION: &str = "23503";

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let single = |msg: &ErrorMessage| Some(json!([msg.to_string()]));

        let (status, details): (StatusCode, Option<serde_json::Value>) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, single(msg)),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, single(msg)),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, single(msg)),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, single(msg)),
            AppError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, single(msg)),
            AppError::Validation(msgs) => {
                let string_msgs: Vec<String> = msgs.iter().map(|m| m.to_string()).collect();
                (StatusCode::UNPROCESSABLE_ENTITY, Some(json!(string_msgs)))
            }
            AppError::BadGateway(msg) => (StatusCode::BAD_GATEWAY, single(msg)),
            AppError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, single(msg)),
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, None)
            }
            AppError::Database(e) => {
                tracing::error!("DB error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, None)
            }
            AppError::Request(e) => {
                tracing::error!("Upstream request error: {e}");
                (
                    StatusCode::BAD_GATEWAY,
                    single(&ErrorMessage::UpstreamUnavailable),
                )
            }
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, None),
        };

        (
            status,
            Json(json!({ "error": self.to_string(), "details": details })),
        )
            .into_response()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorMessage {
    // Auth
    InvalidCredentials,
    EmailTaken,
    UsernameTaken,
    TokenExpired,
    TokenInvalid,
    TooManyEmails,
    EmailAlreadyVerified,
    RateLimited,
    GoogleEmailNotVerified,
    GoogleAccountAlreadyLinked,
    AccountEmailNotVerified,

    // Validation
    UsernameInvalidLength,
    UsernameInvalidCharacters,
    EmailInvalidFormat,
    PasswordInvalidPolicy,
    PasswordSameAsCurrent,
    TrackIdInvalid,
    CannotDelegateToSelf,
    CannotInviteSelf,

    // Resources
    UserNotFound,
    RoomNotFound,
    ActiveRoomNotFound,
    DeviceNotInRoom,
    FriendNotFound,
    FriendAlreadyExists,
    SelfFriendRequest,
    SenderCannotAcceptOwn,
    InvitationNotFound,
    AlreadyInvited,
    NotInvitedUser,
    TrackNotFound,
    TrackNotStreamable,
    QueueItemNotFound,
    QueuePositionTaken,

    // Permissions
    NotRoomOwner,
    NotInvited,
    MissingLicense,
    SubscriptionRequired,
    StreamLinkInvalid,
    StreamLinkExpired,

    // Server
    UpstreamUnavailable,
    TooManyRooms,
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
            Self::TooManyEmails => {
                "An email was already sent recently, please check your inbox or try again later"
            }
            Self::EmailAlreadyVerified => "Email is already verified",
            Self::RateLimited => "Too many attempts, please try again in a minute",
            Self::GoogleEmailNotVerified => "This Google account has no verified email address",
            Self::GoogleAccountAlreadyLinked => {
                "This Google account is already linked to another user"
            }
            Self::AccountEmailNotVerified => {
                "An account with this email already exists but its email is not verified. Log in with your password and link Google from your profile"
            }

            // Validation
            Self::UsernameInvalidLength => {
                "Username has invalid length (must be between 3 and 24 characters)"
            }
            Self::UsernameInvalidCharacters => "Username contains invalid characters",
            Self::EmailInvalidFormat => "Invalid email address",
            Self::PasswordInvalidPolicy => {
                "Password does not meet the required policy (must be at least 8 and at most 128 characters)"
            }
            Self::PasswordSameAsCurrent => "New password must be different from current password",
            Self::TrackIdInvalid => "Track ID must be a positive integer",
            Self::CannotDelegateToSelf => "The room owner cannot delegate playback to themselves",
            Self::CannotInviteSelf => "You cannot invite yourself",

            // Resources
            Self::UserNotFound => "User not found",
            Self::RoomNotFound => "Room not found",
            Self::ActiveRoomNotFound => "Room is not currently active",
            Self::DeviceNotInRoom => "Device is not in this room",
            Self::FriendNotFound => "Friend or request not found",
            Self::FriendAlreadyExists => "Friendship or request already exists",
            Self::SelfFriendRequest => "Cannot be friend with yourself",
            Self::SenderCannotAcceptOwn => "Sender cannot accept their own request",
            Self::InvitationNotFound => "Invitation not found",
            Self::AlreadyInvited => "User is already invited",
            Self::NotInvitedUser => "You are not the invited user",
            Self::TrackNotFound => "Track not found",
            Self::TrackNotStreamable => "This track is not available for streaming",
            Self::QueueItemNotFound => "This track is no longer in the queue",
            Self::QueuePositionTaken => "Another track already has this position in the queue",

            // Permissions
            Self::NotRoomOwner => "You are not the owner of this room",
            Self::NotInvited => "You are not invited to this room",
            Self::MissingLicense => "You need to be invited to edit this room's playlist",
            Self::SubscriptionRequired => "This feature is available for premium subscribers only",
            Self::StreamLinkInvalid => "Invalid stream link",
            Self::StreamLinkExpired => "This stream link has expired",

            // Server
            Self::UpstreamUnavailable => "An external service is unavailable, try again later",
            Self::TooManyRooms => "Too many active rooms, try again later",
        }
    }
}

impl std::fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
