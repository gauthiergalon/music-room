use axum::extract::ws::Utf8Bytes;

use crate::dtos::ws::WsEventServer;

pub mod connection;
pub mod manager;
pub mod room;

pub use connection::{WsUser, handle_socket};
pub use room::send_room_state;

/// An event broadcast to a room, already serialised. Every connection of the
/// room forwards the same text: cloning it only bumps a reference count,
/// instead of copying and serialising the event once per member.
pub type RoomMessage = Utf8Bytes;

/// Serialises an event once. `None` (and logged) if it cannot be.
pub fn encode(event: &WsEventServer) -> Option<RoomMessage> {
    match serde_json::to_string(event) {
        Ok(text) => Some(text.into()),
        Err(e) => {
            tracing::error!("Failed to serialise a WS event: {}", e);
            None
        }
    }
}
