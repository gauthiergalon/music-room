use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    models::{
        email_token::NewEmailToken,
        user::{PrivacyLevel, User},
    },
    repositories::{
        email_tokens as email_tokens_repo, refresh_tokens as refresh_tokens_repo,
        users as users_repo,
    },
    services::{
        auth::{self, Tokens, hash_password, verify_password},
        email::{Email, Mailer},
        google::GoogleAuth,
        tokens::TokenPair,
    },
};

pub async fn get_me(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    refresh_subscription_status(pool, user_id).await
}

pub async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    users_repo::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))
}

/// `new_username` must already be validated and trimmed.
pub async fn update_username(
    pool: &PgPool,
    user_id: Uuid,
    new_username: &str,
) -> Result<User, AppError> {
    users_repo::update_username(pool, user_id, new_username).await
}

/// Sends a confirmation link to `address` and remembers which address the
/// link is for. One link per user at a time, so this cannot be used to flood
/// a mailbox.
async fn send_confirmation_link(
    pool: &PgPool,
    mailer: &dyn Mailer,
    public_url: &str,
    user_id: Uuid,
    address: &str,
) -> Result<(), AppError> {
    if email_tokens_repo::find_valid_by_user_id(pool, user_id)
        .await?
        .is_some()
    {
        return Err(AppError::TooManyRequests(ErrorMessage::TooManyEmails));
    }

    let token_pair = TokenPair::generate();
    email_tokens_repo::create(
        pool,
        NewEmailToken {
            token_hash: token_pair.hash,
            user_id,
            new_email: address.to_string(),
            expires_at: Utc::now() + Duration::minutes(15),
        },
    )
    .await?;

    let email = Email::for_email_confirmation(public_url, &token_pair.plain);
    if let Err(e) = mailer.send(address, &email).await {
        // Without the email the token is useless and would block a retry for 15 minutes.
        let _ = email_tokens_repo::delete_by_user_id(pool, user_id).await;
        return Err(e);
    }

    Ok(())
}

/// Starts an email change. The address on the account stays the same until
/// the link sent to the new address is confirmed, so a typo or someone else's
/// address can never lock the user out or hijack the account.
///
/// `new_email` must already be validated and normalised.
pub async fn request_email_change(
    pool: &PgPool,
    mailer: &dyn Mailer,
    public_url: &str,
    user_id: Uuid,
    new_email: &str,
) -> Result<User, AppError> {
    let user = get_user(pool, user_id).await?;

    if user.email == new_email {
        return Ok(user);
    }
    if users_repo::find_by_email(pool, new_email).await?.is_some() {
        return Err(AppError::Conflict(ErrorMessage::EmailTaken));
    }

    send_confirmation_link(pool, mailer, public_url, user_id, new_email).await?;
    Ok(user)
}

/// Changes the password, ends every other session and opens a new one for the caller.
pub async fn update_password(
    pool: &PgPool,
    jwt_secret: &str,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<Tokens, AppError> {
    let user = get_user(pool, user_id).await?;
    let current_hash = user
        .password_hash
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    verify_password(current_password, &current_hash).await?;
    if current_password == new_password {
        return Err(AppError::Validation(vec![
            ErrorMessage::PasswordSameAsCurrent,
        ]));
    }
    let new_password_hash = hash_password(new_password).await?;

    let mut tx = pool.begin().await?;
    users_repo::update_password(&mut *tx, user_id, new_password_hash).await?;
    refresh_tokens_repo::delete_all_for_user(&mut *tx, user_id).await?;
    tx.commit().await?;

    auth::issue_tokens(pool, jwt_secret, user_id, user.username).await
}

pub async fn confirm_email(pool: &PgPool, token: &str) -> Result<(), AppError> {
    let token_hash = TokenPair::hash(token);
    let mut tx = pool.begin().await?;

    let stored_token = email_tokens_repo::delete_and_return(&mut *tx, token_hash)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    if stored_token.expires_at < Utc::now() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
    }

    let user = users_repo::find_by_id(&mut *tx, stored_token.user_id)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

    if stored_token.new_email != user.email {
        users_repo::confirm_and_update_email(
            &mut *tx,
            stored_token.user_id,
            &stored_token.new_email,
        )
        .await?;
    } else {
        users_repo::confirm_email(&mut *tx, stored_token.user_id).await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn send_email_confirmation_email(
    pool: &PgPool,
    mailer: &dyn Mailer,
    public_url: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    let user = get_user(pool, user_id).await?;

    if user.email_confirmed {
        return Err(AppError::Conflict(ErrorMessage::EmailAlreadyVerified));
    }

    send_confirmation_link(pool, mailer, public_url, user_id, &user.email).await
}

/// Links a Google account to the logged-in user.
pub async fn link_google(
    pool: &PgPool,
    google: &GoogleAuth,
    user_id: Uuid,
    id_token: &str,
) -> Result<User, AppError> {
    let token_info = google.verify(id_token).await?;

    if let Some(owner) = users_repo::find_by_google_id(pool, &token_info.sub).await?
        && owner.id != user_id
    {
        return Err(AppError::Conflict(ErrorMessage::GoogleAccountAlreadyLinked));
    }

    users_repo::link_google_id(pool, user_id, &token_info.sub).await?;
    get_user(pool, user_id).await
}

pub async fn update_favorite_genres(
    pool: &PgPool,
    user_id: Uuid,
    favorite_genres: Vec<String>,
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

pub async fn enable_subscription(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    users_repo::enable_subscription(pool, user_id).await
}

pub async fn refresh_subscription_status(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    let user = get_user(pool, user_id).await?;

    let subscription_expired = user.is_subscribed
        && user
            .end_subscription_date
            .is_none_or(|end_date| end_date <= Utc::now());

    if subscription_expired {
        return users_repo::deactivate_subscription(pool, user_id).await;
    }

    Ok(user)
}
