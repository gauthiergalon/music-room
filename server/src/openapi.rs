use utoipa::{
    Modify, OpenApi,
    openapi::{
        self, ResponseBuilder,
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    },
};

/// Name of the security scheme of the access token (repeated as a literal in
/// `#[openapi(security(...))]`, which takes no constant).
const BEARER_AUTH: &str = "bearer_auth";

/// Every route requires the access token unless its annotation says
/// `security(())`: the routes of `/auth`, which hand out the tokens, and the
/// stream relay, whose links carry their own signature. This mirrors where
/// `auth_middleware` is applied in the routers.
#[derive(OpenApi)]
#[openapi(
    modifiers(&BearerAuth),
    security(("bearer_auth" = [])),
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
        crate::handlers::rooms::get,
        crate::handlers::rooms::delete,
        crate::handlers::rooms::publish,
        crate::handlers::rooms::privatize,
        crate::handlers::rooms::enable_license,
        crate::handlers::rooms::disable_license,
        crate::handlers::rooms::delegate_room,
        crate::handlers::rooms::revoke_delegate,

        // Users endpoints
        crate::handlers::user::get_me,
        crate::handlers::user::get_user,
        crate::handlers::user::update_username,
        crate::handlers::user::update_email,
        crate::handlers::user::update_password,
        crate::handlers::user::link_google,
        crate::handlers::user::confirm_email,
        crate::handlers::user::send_email_confirmation_email,
        crate::handlers::user::update_favorite_genres,
        crate::handlers::user::update_privacy_level,
        crate::handlers::user::enable_subscription,

        // Friends endpoints
        crate::handlers::friends::list,
        crate::handlers::friends::send_request,
        crate::handlers::friends::accept_request,
        crate::handlers::friends::reject_request,
        crate::handlers::friends::remove,

        // Hifi endpoints
        crate::handlers::music::search,
        crate::handlers::music::get_stream_url,
        crate::handlers::music::stream_playlist,
        crate::handlers::music::stream_segment,

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
            crate::dtos::rooms::DelegateRoomRequest,

            // User DTOs
            crate::dtos::user::UserResponse,
            crate::dtos::user::PublicUserResponse,
            crate::dtos::user::UpdateFavoriteGenresRequest,
            crate::dtos::user::UpdatePrivacyLevelRequest,
            crate::dtos::user::UpdateUsernameRequest,
            crate::dtos::user::UpdateEmailRequest,
            crate::dtos::user::UpdatePasswordRequest,
            crate::dtos::user::ConfirmEmailRequest,
            crate::dtos::user::LinkGoogleRequest,

            // Friend DTOs
            crate::dtos::friend::FriendRequestDto,
            crate::dtos::friend::FriendResponseDto,

            // Hifi DTOs
            crate::dtos::music::SearchResponse,
            crate::dtos::music::SearchData,
            crate::dtos::music::TrackItem,
            crate::dtos::music::AlbumData,
            crate::dtos::music::ArtistData,
            crate::dtos::music::StreamUrlResponse,

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
        (name = "Subscriptions", description = "Subscription management endpoints"),
    )
)]
pub struct ApiDoc;

/// Declares the access token as a bearer JWT (the "Authorize" button of the
/// Swagger UI) and documents the 401 of every route that requires it.
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        openapi
            .components
            .get_or_insert_with(Default::default)
            .add_security_scheme(
                BEARER_AUTH,
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Access token returned by /auth/login, /auth/register, \
                             /auth/google-login or /auth/refresh",
                        ))
                        .build(),
                ),
            );

        let unauthorized = ResponseBuilder::new()
            .description("Missing, invalid or expired access token")
            .build();
        for item in openapi.paths.paths.values_mut() {
            let operations = [
                &mut item.get,
                &mut item.put,
                &mut item.post,
                &mut item.delete,
                &mut item.patch,
            ];
            for operation in operations.into_iter().flatten() {
                // No security of its own: the global requirement applies.
                if operation.security.is_none() {
                    operation
                        .responses
                        .responses
                        .entry("401".to_string())
                        .or_insert_with(|| unauthorized.clone().into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn spec() -> Value {
        serde_json::to_value(ApiDoc::openapi()).unwrap()
    }

    #[test]
    fn the_access_token_is_a_bearer_jwt_required_by_default() {
        let spec = spec();
        assert_eq!(
            spec["components"]["securitySchemes"][BEARER_AUTH]["scheme"],
            "bearer"
        );
        assert_eq!(spec["security"], json!([{ BEARER_AUTH: [] }]));
    }

    #[test]
    fn protected_routes_document_the_unauthorized_response() {
        let spec = spec();
        let me = &spec["paths"]["/users/me"]["get"];
        assert!(me.get("security").is_none());
        assert!(me["responses"].get("401").is_some());
    }

    #[test]
    fn public_routes_opt_out_of_the_token() {
        let spec = spec();
        for (path, method) in [
            ("/auth/register", "post"),
            ("/auth/logout", "post"),
            ("/hifi/stream/segment", "get"),
        ] {
            let operation = &spec["paths"][path][method];
            assert_eq!(operation["security"], json!([{}]), "{path}");
            assert!(operation["responses"].get("401").is_none(), "{path}");
        }
    }
}
