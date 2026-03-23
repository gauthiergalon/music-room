use crate::dtos::ws::WsEvent;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
	pub pool: PgPool,
	pub jwt_secret: String,
	pub room_channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<WsEvent>>>>,
}
