use axum::{
	Router, middleware,
	routing::{get, post},
};

use crate::{
	handlers::rooms::{create_room, delete_room, get_room, privatize, publish, transfer_ownership, ws_handler},
	middleware::auth::auth_middleware,
	state::AppState,
};

pub fn router(state: AppState) -> Router<AppState> {
	let protected = Router::new().route("/", post(create_room)).route("/{id}", get(get_room).delete(delete_room)).route("/{id}/ws", get(ws_handler)).route("/{id}/transfer-ownership", post(transfer_ownership)).route("/{id}/publish", post(publish)).route("/{id}/privatize", post(privatize)).layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

	protected
}
