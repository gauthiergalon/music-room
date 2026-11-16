//! In-memory presence of the clients connected to each room.
//!
//! The database is the source of truth for the existence of a room; this
//! manager only knows who is connected right now, who may control playback and
//! how to reach them. Every mutation goes through it so that listeners are
//! always notified the same way.

use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, RwLock, broadcast};
use uuid::Uuid;

use crate::{
    dtos::ws::{UserInfo, WsEventServer},
    errors::{AppError, ErrorMessage},
    ws::{RoomMessage, encode},
};

const BROADCAST_CAPACITY: usize = 100;
pub const DEFAULT_MAX_ROOMS: usize = 1000;
pub const DEFAULT_OWNER_GRACE: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct MemberKey {
    user_id: Uuid,
    connection_id: Uuid,
}

#[derive(Clone, Debug)]
struct Member {
    username: String,
    device_name: String,
}

#[derive(Clone, Debug)]
struct Delegate {
    user_id: Uuid,
    connection_id: Uuid,
}

struct ActiveRoom {
    tx: broadcast::Sender<RoomMessage>,
    users: HashMap<MemberKey, Member>,
    owner_id: Uuid,
    delegate: Option<Delegate>,
    last_activity: DateTime<Utc>,
    /// Bumped every time the owner connects or fully disconnects, so a pending
    /// "close after the grace period" timer can tell whether it is stale.
    owner_epoch: u64,
    /// Serialises "read the room state from the database and broadcast it" and
    /// holds the version of the last state that was sent.
    state_version: Arc<Mutex<u64>>,
}

impl ActiveRoom {
    fn new(owner_id: Uuid) -> Self {
        Self {
            tx: broadcast::channel(BROADCAST_CAPACITY).0,
            users: HashMap::new(),
            owner_id,
            delegate: None,
            last_activity: Utc::now(),
            owner_epoch: 0,
            state_version: Arc::new(Mutex::new(0)),
        }
    }

    fn owner_connected(&self) -> bool {
        self.users.keys().any(|key| key.user_id == self.owner_id)
    }

    fn user_state(&self) -> WsEventServer {
        let mut user_list: Vec<UserInfo> = self
            .users
            .iter()
            .map(|(key, member)| UserInfo {
                user_id: key.user_id,
                username: member.username.clone(),
                connection_id: key.connection_id,
                device_name: member.device_name.clone(),
            })
            .collect();
        // HashMap order is random; keep the list stable for the clients.
        user_list.sort_by(|a, b| {
            (a.username.as_str(), a.connection_id).cmp(&(b.username.as_str(), b.connection_id))
        });

        WsEventServer::UserState {
            user_list,
            owner: self.owner_id,
            delegate_user_id: self.delegate.as_ref().map(|d| d.user_id),
            delegate_connection_id: self.delegate.as_ref().map(|d| d.connection_id),
        }
    }

    fn broadcast_user_state(&self) {
        if let Some(message) = encode(&self.user_state()) {
            let _ = self.tx.send(message);
        }
    }
}

pub struct JoinRequest {
    pub room_id: Uuid,
    pub owner_id: Uuid,
    pub user_id: Uuid,
    pub connection_id: Uuid,
    pub username: String,
    pub device_name: String,
}

/// What a disconnection means for the room.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LeaveOutcome {
    /// Set when the owner has no connection left. The room must be closed
    /// after the grace period unless the epoch has changed by then.
    pub owner_left_epoch: Option<u64>,
}

/// Handle used to publish the playback state of a room in a consistent order.
pub struct StateHandle {
    pub version: Arc<Mutex<u64>>,
    pub tx: broadcast::Sender<RoomMessage>,
}

pub struct RoomManager {
    rooms: RwLock<HashMap<Uuid, ActiveRoom>>,
    max_rooms: usize,
    owner_grace: Duration,
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ROOMS, DEFAULT_OWNER_GRACE)
    }
}

impl RoomManager {
    pub fn new(max_rooms: usize, owner_grace: Duration) -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            max_rooms,
            owner_grace,
        }
    }

    /// How long a room survives after its owner's last connection dropped.
    pub fn owner_grace(&self) -> Duration {
        self.owner_grace
    }

    /// Whether a new connection to `room_id` can be accepted.
    pub async fn has_capacity_for(&self, room_id: Uuid) -> bool {
        let rooms = self.rooms.read().await;
        rooms.contains_key(&room_id) || rooms.len() < self.max_rooms
    }

    /// Registers a connection and tells everyone in the room about it.
    pub async fn join(
        &self,
        request: JoinRequest,
    ) -> Result<broadcast::Receiver<RoomMessage>, AppError> {
        let mut rooms = self.rooms.write().await;

        if !rooms.contains_key(&request.room_id) && rooms.len() >= self.max_rooms {
            tracing::warn!(
                "Max active rooms limit reached, refusing room {}",
                request.room_id
            );
            return Err(AppError::ServiceUnavailable(ErrorMessage::TooManyRooms));
        }

        let room = rooms
            .entry(request.room_id)
            .or_insert_with(|| ActiveRoom::new(request.owner_id));

        room.last_activity = Utc::now();
        if request.user_id == room.owner_id {
            room.owner_epoch += 1;
        }

        // Subscribe before broadcasting so the newcomer receives the user list too.
        let rx = room.tx.subscribe();
        room.users.insert(
            MemberKey {
                user_id: request.user_id,
                connection_id: request.connection_id,
            },
            Member {
                username: request.username,
                device_name: request.device_name,
            },
        );
        room.broadcast_user_state();

        Ok(rx)
    }

    /// Removes a connection. Returns `None` if the room is not active any more.
    pub async fn leave(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        connection_id: Uuid,
    ) -> Option<LeaveOutcome> {
        let mut rooms = self.rooms.write().await;
        let room = rooms.get_mut(&room_id)?;

        room.users.remove(&MemberKey {
            user_id,
            connection_id,
        });

        if room
            .delegate
            .as_ref()
            .is_some_and(|d| d.user_id == user_id && d.connection_id == connection_id)
        {
            room.delegate = None;
        }

        let mut outcome = LeaveOutcome::default();
        if user_id == room.owner_id && !room.owner_connected() {
            room.owner_epoch += 1;
            outcome.owner_left_epoch = Some(room.owner_epoch);
        }

        room.broadcast_user_state();
        Some(outcome)
    }

    /// True if the owner is still away since the disconnection identified by `epoch`.
    pub async fn owner_still_absent(&self, room_id: Uuid, epoch: u64) -> bool {
        self.rooms
            .read()
            .await
            .get(&room_id)
            .is_some_and(|room| room.owner_epoch == epoch && !room.owner_connected())
    }

    pub async fn owner_connected(&self, room_id: Uuid) -> bool {
        self.rooms
            .read()
            .await
            .get(&room_id)
            .is_some_and(ActiveRoom::owner_connected)
    }

    /// Drops the room and tells every connected client it is closed.
    /// Returns whether the room was active.
    pub async fn close(&self, room_id: Uuid) -> bool {
        let room = self.rooms.write().await.remove(&room_id);
        match room {
            Some(room) => {
                if let Some(message) = encode(&WsEventServer::RoomClosed) {
                    let _ = room.tx.send(message);
                }
                tracing::debug!("[WS SEND] Room: {}, Type: RoomClosed", room_id);
                true
            }
            None => false,
        }
    }

    pub async fn touch(&self, room_id: Uuid) {
        if let Some(room) = self.rooms.write().await.get_mut(&room_id) {
            room.last_activity = Utc::now();
        }
    }

    pub async fn can_control(&self, room_id: Uuid, user_id: Uuid, connection_id: Uuid) -> bool {
        self.rooms
            .read()
            .await
            .get(&room_id)
            .is_some_and(|room| match &room.delegate {
                Some(delegate) => {
                    delegate.user_id == user_id && delegate.connection_id == connection_id
                }
                None => room.owner_id == user_id,
            })
    }

    /// Hands playback control to one connected device.
    pub async fn set_delegate(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        connection_id: Uuid,
    ) -> Result<(), AppError> {
        let mut rooms = self.rooms.write().await;
        let room = rooms
            .get_mut(&room_id)
            .ok_or(AppError::NotFound(ErrorMessage::ActiveRoomNotFound))?;

        let key = MemberKey {
            user_id,
            connection_id,
        };
        if !room.users.contains_key(&key) {
            return Err(AppError::NotFound(ErrorMessage::DeviceNotInRoom));
        }

        room.delegate = Some(Delegate {
            user_id,
            connection_id,
        });
        room.broadcast_user_state();
        Ok(())
    }

    /// Gives playback control back to the owner.
    pub async fn clear_delegate(&self, room_id: Uuid) -> Result<(), AppError> {
        let mut rooms = self.rooms.write().await;
        let room = rooms
            .get_mut(&room_id)
            .ok_or(AppError::NotFound(ErrorMessage::ActiveRoomNotFound))?;

        if room.delegate.take().is_some() {
            room.broadcast_user_state();
        }
        Ok(())
    }

    pub async fn user_state(&self, room_id: Uuid) -> Option<WsEventServer> {
        self.rooms
            .read()
            .await
            .get(&room_id)
            .map(ActiveRoom::user_state)
    }

    pub async fn state_handle(&self, room_id: Uuid) -> Option<StateHandle> {
        self.rooms
            .read()
            .await
            .get(&room_id)
            .map(|room| StateHandle {
                version: room.state_version.clone(),
                tx: room.tx.clone(),
            })
    }

    /// Rooms without any playback or presence activity for longer than `max_idle`.
    pub async fn idle_rooms(&self, max_idle: chrono::Duration) -> Vec<Uuid> {
        let now = Utc::now();
        self.rooms
            .read()
            .await
            .iter()
            .filter(|(_, room)| now - room.last_activity > max_idle)
            .map(|(id, _)| *id)
            .collect()
    }

    pub async fn active_count(&self) -> usize {
        self.rooms.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join_request(room_id: Uuid, owner_id: Uuid, user_id: Uuid) -> JoinRequest {
        JoinRequest {
            room_id,
            owner_id,
            user_id,
            connection_id: Uuid::new_v4(),
            username: "user".to_string(),
            device_name: "device".to_string(),
        }
    }

    #[tokio::test]
    async fn owner_leaving_reports_an_epoch_that_a_reconnect_invalidates() {
        let manager = RoomManager::default();
        let (room_id, owner) = (Uuid::new_v4(), Uuid::new_v4());

        let request = join_request(room_id, owner, owner);
        let connection = request.connection_id;
        let _rx = manager.join(request).await.unwrap();

        let outcome = manager.leave(room_id, owner, connection).await.unwrap();
        let epoch = outcome
            .owner_left_epoch
            .expect("owner has no connection left");
        assert!(manager.owner_still_absent(room_id, epoch).await);

        let _rx = manager
            .join(join_request(room_id, owner, owner))
            .await
            .unwrap();
        assert!(!manager.owner_still_absent(room_id, epoch).await);
    }

    #[tokio::test]
    async fn owner_with_a_second_device_is_not_reported_as_gone() {
        let manager = RoomManager::default();
        let (room_id, owner) = (Uuid::new_v4(), Uuid::new_v4());

        let first = join_request(room_id, owner, owner);
        let first_connection = first.connection_id;
        let _rx1 = manager.join(first).await.unwrap();
        let _rx2 = manager
            .join(join_request(room_id, owner, owner))
            .await
            .unwrap();

        let outcome = manager
            .leave(room_id, owner, first_connection)
            .await
            .unwrap();
        assert_eq!(outcome.owner_left_epoch, None);
    }

    #[tokio::test]
    async fn listener_leaving_does_not_close_the_room() {
        let manager = RoomManager::default();
        let (room_id, owner, listener) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

        let _owner_rx = manager
            .join(join_request(room_id, owner, owner))
            .await
            .unwrap();
        let request = join_request(room_id, owner, listener);
        let connection = request.connection_id;
        let _rx = manager.join(request).await.unwrap();

        let outcome = manager.leave(room_id, listener, connection).await.unwrap();
        assert_eq!(outcome.owner_left_epoch, None);
        assert!(manager.owner_connected(room_id).await);
    }

    #[tokio::test]
    async fn control_follows_the_delegated_connection_only() {
        let manager = RoomManager::default();
        let (room_id, owner, listener) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

        let owner_request = join_request(room_id, owner, owner);
        let owner_connection = owner_request.connection_id;
        let _owner_rx = manager.join(owner_request).await.unwrap();

        let first = join_request(room_id, owner, listener);
        let first_connection = first.connection_id;
        let _rx1 = manager.join(first).await.unwrap();
        let second = join_request(room_id, owner, listener);
        let second_connection = second.connection_id;
        let _rx2 = manager.join(second).await.unwrap();

        assert!(manager.can_control(room_id, owner, owner_connection).await);
        assert!(
            !manager
                .can_control(room_id, listener, first_connection)
                .await
        );

        manager
            .set_delegate(room_id, listener, first_connection)
            .await
            .unwrap();
        assert!(
            manager
                .can_control(room_id, listener, first_connection)
                .await
        );
        assert!(
            !manager
                .can_control(room_id, listener, second_connection)
                .await
        );
        assert!(!manager.can_control(room_id, owner, owner_connection).await);

        // The delegate disconnecting hands control back to the owner.
        manager.leave(room_id, listener, first_connection).await;
        assert!(manager.can_control(room_id, owner, owner_connection).await);
    }

    #[tokio::test]
    async fn room_limit_is_enforced_for_new_rooms_only() {
        let manager = RoomManager::new(1, DEFAULT_OWNER_GRACE);
        let (room_id, owner) = (Uuid::new_v4(), Uuid::new_v4());

        let _rx = manager
            .join(join_request(room_id, owner, owner))
            .await
            .unwrap();
        assert!(
            manager
                .join(join_request(room_id, owner, Uuid::new_v4()))
                .await
                .is_ok()
        );
        assert!(
            manager
                .join(join_request(Uuid::new_v4(), owner, owner))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn closing_notifies_connected_clients() {
        let manager = RoomManager::default();
        let (room_id, owner) = (Uuid::new_v4(), Uuid::new_v4());
        let mut rx = manager
            .join(join_request(room_id, owner, owner))
            .await
            .unwrap();
        // Drain the UserState sent on join.
        let _ = rx.recv().await;

        assert!(manager.close(room_id).await);
        let closed: serde_json::Value =
            serde_json::from_str(rx.recv().await.unwrap().as_str()).unwrap();
        assert_eq!(closed["type"], "RoomClosed");
        assert!(!manager.close(room_id).await);
    }
}
