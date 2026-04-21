use crate::{
    dtos::friend::FriendResponseDto, errors::AppError, repositories::friends as friends_repo,
};
use sqlx::PgPool;
use uuid::Uuid;

fn order_ids(id1: Uuid, id2: Uuid) -> (Uuid, Uuid) {
    if id1 < id2 { (id1, id2) } else { (id2, id1) }
}

pub async fn send_request(
    pool: &PgPool,
    sender_id: Uuid,
    username: &str,
) -> Result<FriendResponseDto, AppError> {
    let target = crate::repositories::users::find_by_username(pool, username)
        .await?
        .ok_or(AppError::NotFound(
            crate::errors::ErrorMessage::UserNotFound,
        ))?;
    let target_id = target.id;

    if sender_id == target_id {
        return Err(AppError::Conflict(
            crate::errors::ErrorMessage::SelfFriendRequest,
        ));
    }

    let (user_id_1, user_id_2) = order_ids(sender_id, target_id);
    let friend = friends_repo::create(pool, user_id_1, user_id_2, sender_id).await?;

    Ok(friend.into())
}

pub async fn accept_request(
    pool: &PgPool,
    acceptor_id: Uuid,
    requester_id: Uuid,
) -> Result<FriendResponseDto, AppError> {
    let (user_id_1, user_id_2) = order_ids(acceptor_id, requester_id);

    let existing = friends_repo::find_by_users(pool, user_id_1, user_id_2)
        .await?
        .ok_or(AppError::NotFound(
            crate::errors::ErrorMessage::FriendNotFound,
        ))?;

    if existing.sender_id == acceptor_id {
        return Err(AppError::Conflict(
            crate::errors::ErrorMessage::SenderCannotAcceptOwn,
        ));
    }

    if !existing.is_pending {
        return Err(AppError::Conflict(
            crate::errors::ErrorMessage::FriendAlreadyExists,
        ));
    }

    let friend = friends_repo::update_accept(pool, user_id_1, user_id_2).await?;

    Ok(friend.into())
}

pub async fn reject_request(pool: &PgPool, user_id: Uuid, target_id: Uuid) -> Result<(), AppError> {
    let (user_id_1, user_id_2) = order_ids(user_id, target_id);
    friends_repo::delete_pending(pool, user_id_1, user_id_2).await
}

pub async fn remove_friend(pool: &PgPool, user_id: Uuid, target_id: Uuid) -> Result<(), AppError> {
    let (user_id_1, user_id_2) = order_ids(user_id, target_id);
    friends_repo::delete(pool, user_id_1, user_id_2).await
}

pub async fn list_friends(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<FriendResponseDto>, AppError> {
    let friends = friends_repo::find_by_user(pool, user_id).await?;
    let dtos = friends.into_iter().map(Into::into).collect();
    Ok(dtos)
}

pub async fn are_friends(pool: &PgPool, id1: Uuid, id2: Uuid) -> Result<bool, AppError> {
    let (user_id_1, user_id_2) = order_ids(id1, id2);
    let friend = friends_repo::find_by_users(pool, user_id_1, user_id_2).await?;
    match friend {
        Some(f) => Ok(!f.is_pending),
        None => Ok(false),
    }
}
