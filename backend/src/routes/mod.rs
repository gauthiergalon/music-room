use axum::Router;
use sqlx::PgPool;

use crate::state::AppState;

mod auth;
mod friends;
mod hifi;
mod invitations;
mod rooms;
mod user;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn app_router(state: AppState) -> Router<AppState> {
    let swagger = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", crate::openapi::ApiDoc::openapi());

    Router::new()
        .merge(swagger)
        .nest("/auth", auth::router(state.clone()))
        .nest("/rooms", rooms::router(state.clone()))
        .nest("/users", user::router(state.clone()))
        .nest("/friends", friends::router(state.clone()))
        .nest("/hifi", hifi::router(state.clone()))
        .merge(invitations::router(state.clone()))
}
