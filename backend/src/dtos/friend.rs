use crate::models::friend::Friend;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FriendRequestDto {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendResponseDto {
    pub user_id_1: Uuid,
    pub user_id_2: Uuid,
    pub sender_id: Uuid,
    pub is_pending: bool,
}

impl From<Friend> for FriendResponseDto {
    fn from(friend: Friend) -> Self {
        Self {
            user_id_1: friend.user_id_1,
            user_id_2: friend.user_id_2,
            sender_id: friend.sender_id,
            is_pending: friend.is_pending,
        }
    }
}
