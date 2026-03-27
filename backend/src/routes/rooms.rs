use axum::{
	Router, middleware,
	routing::{get, patch, post},
};

use crate::{
	handlers::{queue, rooms},
	middleware::auth::auth_middleware,
	state::AppState,
};

pub fn router(state: AppState) -> Router<AppState> {
	let protected = Router::new().route("/", get(rooms::list).post(rooms::create)).route("/{id}", get(rooms::get).delete(rooms::delete)).route("/{id}/ws", get(rooms::ws)).route("/{id}/transfer-ownership", post(rooms::transfer_ownership)).route("/{id}/publish", post(rooms::publish)).route("/{id}/privatize", post(rooms::privatize)).route("/{id}/queue", get(queue::list).post(queue::add).delete(queue::delete).patch(queue::reorder)).layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

	protected
}
