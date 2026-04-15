use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    models::{
        email_token::NewEmailToken,
        user::{PrivacyLevel, User},
    },
    repositories::{email_tokens as email_tokens_repo, users as users_repo},
    services::{
        auth::{hash_password, verify_password},
        email::Email,
        tokens::TokenPair,
    },
};

pub async fn get_me(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    users_repo::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))
}

pub async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    users_repo::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))
}

pub async fn update_username(
    pool: &PgPool,
    user_id: Uuid,
    new_username: &str,
) -> Result<User, AppError> {
    users_repo::update_username(pool, user_id, new_username).await
}

pub async fn update_email(pool: &PgPool, user_id: Uuid, new_email: &str) -> Result<User, AppError> {
    users_repo::update_email(pool, user_id, new_email).await
}

pub async fn update_password(
    pool: &PgPool,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let user = users_repo::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;
    let current_hash = user
        .password_hash
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    verify_password(current_password, &current_hash)?;
    let new_password_hash = hash_password(new_password)?;

    users_repo::update_password(pool, user_id, new_password_hash).await?;

    Ok(())
}

pub async fn confirm_email(pool: &PgPool, token: &str) -> Result<(), AppError> {
    let token_hash = TokenPair::hash(token);
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let stored_token = email_tokens_repo::delete_and_return(&mut *tx, token_hash)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    if stored_token.expires_at < Utc::now() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
    }

    users_repo::update_email(&mut *tx, stored_token.user_id, &stored_token.new_email).await?;
    tx.commit().await.map_err(AppError::Database)?;

    Ok(())
}

pub async fn send_email_confirmation_email(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let user = users_repo::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

    if email_tokens_repo::find_valid_by_user_id(pool, user_id)
        .await?
        .is_some()
    {
        return Err(AppError::TooManyRequests(ErrorMessage::TooManyEmails));
    }

    let token_pair = TokenPair::generate();
    let email = Email::for_email_confirmation(&token_pair.plain);
    let email_token = NewEmailToken {
        token_hash: token_pair.hash,
        user_id: user.id,
        new_email: user.email.clone(),
        expires_at: Utc::now() + Duration::hours(24),
    };

    email.send(&user.email)?;
    email_tokens_repo::create(pool, email_token).await?;

    Ok(())
}

pub async fn update_favorite_genres(
    pool: &PgPool,
    user_id: Uuid,
    favorite_genres: Option<Vec<String>>,
) -> Result<User, AppError> {
    users_repo::update_favorite_genres(pool, user_id, favorite_genres).await
}

pub async fn update_privacy_level(
    pool: &PgPool,
    user_id: Uuid,
    privacy_level: PrivacyLevel,
) -> Result<User, AppError> {
    users_repo::update_privacy_level(pool, user_id, privacy_level).await
}
