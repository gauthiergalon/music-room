use crate::dtos::ws::WsEvent;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

pub struct ActiveRoom {
    pub tx: broadcast::Sender<WsEvent>,
    pub users: HashMap<Uuid, String>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub active_rooms: Arc<RwLock<HashMap<Uuid, ActiveRoom>>>,
}
