use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::dtos::ws::WsEventServer;
use crate::services::music_provider::MusicProvider;

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
    pub music_provider: Arc<dyn MusicProvider>,
}
