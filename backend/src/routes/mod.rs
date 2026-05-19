use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

mod auth;
mod friends;
mod music;
mod invitations;
mod rooms;
mod user;

pub fn app_router(state: AppState) -> Router<AppState> {
    let swagger = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", crate::openapi::ApiDoc::openapi());

    Router::new()
        .merge(swagger)
        .nest("/auth", auth::router(state.clone()))
        .nest("/rooms", rooms::router(state.clone()))
        .nest("/users", user::router(state.clone()))
        .nest("/friends", friends::router(state.clone()))
        .nest("/hifi", music::router(state.clone()))
        .merge(invitations::router(state.clone()))
}
