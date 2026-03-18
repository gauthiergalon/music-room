use axum::{
	Router, middleware,
	routing::{get, patch},
};
use sqlx::PgPool;

use crate::{
	handlers::user::{get_me, get_user, update_email, update_password, update_username},
	middleware::auth::auth_middleware,
};

pub fn router() -> Router<PgPool> {
	let protected = Router::new().route("/{id}", get(get_user)).route("/me", get(get_me)).route("/me/username", patch(update_username)).route("/me/email", patch(update_email)).route("/me/password", patch(update_password)).layer(middleware::from_fn(auth_middleware));

	protected
}
