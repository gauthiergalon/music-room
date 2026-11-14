use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::friend::{Friend, FriendWithUser};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct FriendRequestDto {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FriendResponseDto {
    pub user_id_1: Uuid,
    pub user_id_2: Uuid,
    pub sender_id: Uuid,
    pub is_pending: bool,
    /// The other user of the friendship, from the caller's point of view.
    pub friend_id: Uuid,
    pub username: String,
}

impl FriendResponseDto {
    pub fn new(friend: Friend, friend_id: Uuid, username: String) -> Self {
        Self {
            user_id_1: friend.user_id_1,
            user_id_2: friend.user_id_2,
            sender_id: friend.sender_id,
            is_pending: friend.is_pending,
            friend_id,
            username,
        }
    }
}

impl From<FriendWithUser> for FriendResponseDto {
    fn from(friend: FriendWithUser) -> Self {
        Self {
            user_id_1: friend.user_id_1,
            user_id_2: friend.user_id_2,
            sender_id: friend.sender_id,
            is_pending: friend.is_pending,
            friend_id: friend.friend_id,
            username: friend.username,
        }
    }
}
