use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::dtos::ws::{UserInfo, WsEventServer};
use crate::services::music_provider::MusicProvider;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RoomMemberKey {
    pub user_id: Uuid,
    pub device_name: String,
}

#[derive(Clone, Debug)]
pub struct RoomMember {
    pub username: String,
    pub device_name: String,
}

#[derive(Clone, Debug)]
pub struct RoomDelegate {
    pub user_id: Uuid,
    pub device_name: String,
}

pub struct ActiveRoom {
    pub tx: broadcast::Sender<WsEventServer>,
    pub users: HashMap<RoomMemberKey, RoomMember>,
    pub owner_id: Option<Uuid>,
    pub delegate: Option<RoomDelegate>,
    pub last_activity: DateTime<Utc>,
}

impl ActiveRoom {
    pub fn user_list(&self) -> Vec<UserInfo> {
        self.users
            .iter()
            .map(|(member_key, member)| UserInfo {
                user_id: member_key.user_id,
                username: member.username.clone(),
                device_name: member.device_name.clone(),
            })
            .collect()
    }
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
