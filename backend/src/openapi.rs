use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // Auth endpoints
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::logout,
        crate::handlers::auth::refresh,
        crate::handlers::auth::forgot_password,
        crate::handlers::auth::reset_password,
        crate::handlers::auth::google_login,

        // Rooms endpoints
        crate::handlers::rooms::list,
        crate::handlers::rooms::create,

        // Users endpoints
        crate::handlers::user::get_me,
        crate::handlers::user::get_user,
        crate::handlers::user::update_username,
        crate::handlers::user::update_email,
        crate::handlers::user::update_password,
        crate::handlers::user::confirm_email,
        crate::handlers::user::send_email_confirmation_email,
        crate::handlers::user::update_favorite_genres,
        crate::handlers::user::update_privacy_level,

        // Friends endpoints
        crate::handlers::friends::list,
        crate::handlers::friends::send_request,
        crate::handlers::friends::accept_request,
        crate::handlers::friends::reject_request,
        crate::handlers::friends::remove,

        // Hifi endpoints
        crate::handlers::hifi::search,
        crate::handlers::hifi::get_track,
        crate::handlers::hifi::get_stream_url,

        // Invitations endpoints
        crate::handlers::invitations::invite,
        crate::handlers::invitations::list_pending,
        crate::handlers::invitations::accept,
        crate::handlers::invitations::reject,
        crate::handlers::invitations::revoke,

        // Queue endpoints
        crate::handlers::queue::list,
        crate::handlers::queue::add,
        crate::handlers::queue::delete,
        crate::handlers::queue::reorder,
    ),
    components(
        schemas(
            // Auth DTOs
            crate::dtos::auth::RegisterRequest,
            crate::dtos::auth::LoginRequest,
            crate::dtos::auth::LogoutRequest,
            crate::dtos::auth::AuthResponse,
            crate::dtos::auth::RefreshRequest,
            crate::dtos::auth::ForgotPasswordRequest,
            crate::dtos::auth::ResetPasswordRequest,
            crate::dtos::auth::GoogleLoginRequest,

            // Room DTOs
            crate::dtos::rooms::RoomResponse,
            crate::dtos::rooms::TransferOwnershipRequest,

            // User DTOs
            crate::dtos::user::UserResponse,
            crate::dtos::user::PublicUserResponse,
            crate::dtos::user::UpdateFavoriteGenresRequest,
            crate::dtos::user::UpdatePrivacyLevelRequest,
            crate::dtos::user::UpdateUsernameRequest,
            crate::dtos::user::UpdateEmailRequest,
            crate::dtos::user::UpdatePasswordRequest,
            crate::dtos::user::ConfirmEmailRequest,
            crate::dtos::user::ResetPasswordRequest,

            // Friend DTOs
            crate::dtos::friend::FriendRequestDto,
            crate::dtos::friend::FriendResponseDto,

            // Hifi DTOs
            crate::dtos::hifi::SearchResponse,
            crate::dtos::hifi::SearchData,
            crate::dtos::hifi::TrackItem,
            crate::dtos::hifi::AlbumData,
            crate::dtos::hifi::ArtistData,
            crate::dtos::hifi::TrackResponse,
            crate::dtos::hifi::StreamUrlResponse,

            // Invitation DTOs
            crate::dtos::invitation::InvitationResponse,

            // Queue DTOs
            crate::dtos::queue::AddToQueueRequest,
            crate::dtos::queue::RemoveFromQueueRequest,
            crate::dtos::queue::ReorderQueueRequest,

            // WebSocket DTOs
            crate::dtos::ws::UserInfo,
            crate::dtos::ws::QueuedTrack,

            // Models
            crate::models::user::PrivacyLevel,
            crate::models::queue::Queue,
        )
    ),
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Users", description = "User management endpoints"),
        (name = "Rooms", description = "Room management endpoints"),
        (name = "Friends", description = "Friends management endpoints"),
        (name = "Hifi", description = "Hifi search and track endpoints"),
        (name = "Invitations", description = "Room invitations endpoints"),
        (name = "Queue", description = "Queue management endpoints"),
    )
)]
pub struct ApiDoc;
