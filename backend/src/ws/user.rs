use crate::dtos::ws::{UserInfo, WsEventServer};
use crate::state::AppState;
use uuid::Uuid;

pub async fn send_user_state(state: &AppState, room_id: Uuid, owner_id: Uuid) {
    let rooms = state.active_rooms.read().await;
    if let Some(room) = rooms.get(&room_id) {
        let user_list: Vec<UserInfo> = room
            .users
            .iter()
            .map(|(&user_id, username)| UserInfo {
                user_id,
                username: username.clone(),
            })
            .collect();

        let event = WsEventServer::UserState {
            user_list,
            owner: owner_id,
        };

        let _ = room.tx.send(event);
    }
}
