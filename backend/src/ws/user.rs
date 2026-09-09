use uuid::Uuid;

use crate::{
    dtos::ws::WsEventServer,
    state::AppState,
};

pub async fn send_user_state(state: &AppState, room_id: Uuid, owner_id: Uuid) {
    let rooms = state.active_rooms.read().await;
    if let Some(room) = rooms.get(&room_id) {
        let user_list = room.user_list();
        let event = WsEventServer::UserState {
            user_list,
            owner: owner_id,
            delegate_user_id: room.delegate.as_ref().map(|delegate| delegate.user_id),
            delegate_device: room.delegate.as_ref().map(|delegate| delegate.device_name.clone()),
        };

        let _ = room.tx.send(event);
    }
}
