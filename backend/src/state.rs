use crate::dtos::ws::WsEventServer;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

pub struct ActiveRoom {
    pub tx: broadcast::Sender<WsEventServer>,
    pub users: HashMap<Uuid, String>,
    pub owner_id: Option<Uuid>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_auth_url: String,
    pub active_rooms: Arc<RwLock<HashMap<Uuid, ActiveRoom>>>,
}
