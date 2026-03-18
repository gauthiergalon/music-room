use axum::{
	Router, middleware,
	routing::{get, post},
};
use sqlx::PgPool;

use crate::{
	handlers::rooms::{create_room, delete_room, get_room, privatize, publish, transfer_ownership},
	middleware::auth::auth_middleware,
};

pub fn router() -> Router<PgPool> {
	let protected = Router::new().route("/", post(create_room)).route("/{id}", get(get_room).delete(delete_room)).route("/{id}/transfer-ownership", post(transfer_ownership)).route("/{id}/publish", post(publish)).route("/{id}/privatize", post(privatize)).layer(middleware::from_fn(auth_middleware));

	protected
}
