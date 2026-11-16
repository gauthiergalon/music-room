use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{TimeDelta, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    models::{
        refresh_token::NewRefreshToken,
        reset_token::NewResetToken,
        user::{NewUser, PrivacyLevel},
    },
    repositories::{
        refresh_tokens as refresh_tokens_repo, reset_tokens as reset_tokens_repo,
        users as users_repo,
    },
    services::{
        email::{Email, Mailer},
        google::GoogleAuth,
        tokens::TokenPair,
    },
    validation::USERNAME_MAX_CHARS,
};

/// Access and refresh token of a session.
pub type Tokens = (String, String);

/// Argon2 is deliberately slow and takes about 19 MB of memory per hash.
/// Hashes run off the async workers, so one login cannot stall every other
/// request, and at most one per CPU at a time: a burst of logins queues up
/// instead of running hundreds of hashes at once and exhausting the memory.
static HASHING: LazyLock<Semaphore> = LazyLock::new(|| {
    Semaphore::new(std::thread::available_parallelism().map_or(2, |cpus| cpus.get()))
});

async fn run_argon2<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let _permit = HASHING.acquire().await.map_err(|_| AppError::Internal)?;
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| AppError::Internal)?
}

pub async fn hash_password(password: &str) -> Result<String, AppError> {
    let password = password.to_owned();
    run_argon2(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::Internal)
    })
    .await
}

pub async fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
    let (password, hash) = (password.to_owned(), hash.to_owned());
    run_argon2(move || {
        let parsed = PasswordHash::new(&hash).map_err(|_| AppError::Internal)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AppError::Unauthorized(ErrorMessage::InvalidCredentials))
    })
    .await
}

fn generate_access_token(
    user_id: Uuid,
    username: String,
    secret: &str,
) -> Result<String, AppError> {
    let exp = (Utc::now() + TimeDelta::minutes(15)).timestamp() as usize;
    let claims = Claims {
        user_id,
        username,
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::Internal)
}

async fn store_refresh_token(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let token_pair = TokenPair::generate();
    let expires_at = Utc::now() + TimeDelta::days(7);

    refresh_tokens_repo::create(
        pool,
        NewRefreshToken {
            token_hash: token_pair.hash,
            user_id,
            expires_at,
        },
    )
    .await?;

    Ok(token_pair.plain)
}

/// Opens a new session for the user.
pub async fn issue_tokens(
    pool: &PgPool,
    jwt_secret: &str,
    user_id: Uuid,
    username: String,
) -> Result<Tokens, AppError> {
    let access_token = generate_access_token(user_id, username, jwt_secret)?;
    let refresh_token = store_refresh_token(pool, user_id).await?;
    Ok((access_token, refresh_token))
}

/// `username` and `email` must already be validated and normalised.
pub async fn register(
    pool: &PgPool,
    jwt_secret: &str,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Tokens, AppError> {
    let password_hash = hash_password(password).await?;

    let user_id = users_repo::create(
        pool,
        NewUser {
            username,
            email,
            password_hash: Some(password_hash),
            email_confirmed: false,
            google_id: None,
            privacy_level: PrivacyLevel::Friends,
        },
    )
    .await?;

    issue_tokens(pool, jwt_secret, user_id, username.to_string()).await
}

pub async fn login(
    pool: &PgPool,
    jwt_secret: &str,
    email: &str,
    password: &str,
) -> Result<Tokens, AppError> {
    let user = users_repo::find_by_email(pool, email)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    let password_hash = user
        .password_hash
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;
    verify_password(password, &password_hash).await?;

    issue_tokens(pool, jwt_secret, user.id, user.username).await
}

/// Possession of the refresh token is what authorises ending its session, so
/// this works even when the access token has already expired.
pub async fn logout(pool: &PgPool, token: &str) -> Result<(), AppError> {
    refresh_tokens_repo::delete(pool, TokenPair::hash(token)).await
}

pub async fn refresh(pool: &PgPool, jwt_secret: &str, token: &str) -> Result<Tokens, AppError> {
    let token_hash = TokenPair::hash(token);
    let stored = refresh_tokens_repo::delete_and_return(pool, token_hash)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    if stored.expires_at < Utc::now() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
    }

    let user = users_repo::find_by_id(pool, stored.user_id)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    issue_tokens(pool, jwt_secret, user.id, user.username).await
}

pub async fn forgot_password(
    pool: &PgPool,
    mailer: &dyn Mailer,
    email_address: &str,
) -> Result<(), AppError> {
    let user = users_repo::find_by_email(pool, email_address)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

    // One valid link per user at a time. Requests for the same account are
    // serialised on the user row, so two of them arriving together cannot
    // both pass the check and send two emails.
    let token_pair = TokenPair::generate();
    let mut tx = pool.begin().await?;
    users_repo::lock(&mut *tx, user.id).await?;

    if reset_tokens_repo::find_valid_by_user_id(&mut *tx, user.id)
        .await?
        .is_some()
    {
        return Err(AppError::TooManyRequests(ErrorMessage::TooManyEmails));
    }

    reset_tokens_repo::create(
        &mut *tx,
        NewResetToken {
            token_hash: token_pair.hash,
            user_id: user.id,
            expires_at: Utc::now() + TimeDelta::minutes(15),
        },
    )
    .await?;
    tx.commit().await?;

    let email = Email::for_password_reset(&token_pair.plain);
    if let Err(e) = mailer.send(email_address, &email).await {
        // Without the email the token is useless and would block a retry for 15 minutes.
        let _ = reset_tokens_repo::delete_by_user_id(pool, user.id).await;
        return Err(e);
    }

    Ok(())
}

pub async fn reset_password(
    pool: &PgPool,
    token: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let token_hash = TokenPair::hash(token);
    let password_hash = hash_password(new_password).await?;

    let mut tx = pool.begin().await?;

    let stored = reset_tokens_repo::delete_and_return(&mut *tx, token_hash)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    if stored.expires_at < Utc::now() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
    }

    users_repo::update_password(&mut *tx, stored.user_id, password_hash).await?;
    // Whoever knew the old password must not keep a session.
    refresh_tokens_repo::delete_all_for_user(&mut *tx, stored.user_id).await?;

    tx.commit().await?;

    Ok(())
}

/// Builds a valid, almost certainly unique username from a Google given name.
fn username_from_google(given_name: Option<&str>) -> String {
    const SUFFIX_CHARS: usize = 6;

    let base: String = given_name
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(USERNAME_MAX_CHARS - SUFFIX_CHARS - 1)
        .collect();
    let base = if base.is_empty() {
        "User".to_string()
    } else {
        base
    };

    let suffix: String = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(SUFFIX_CHARS)
        .collect();

    format!("{base}_{suffix}")
}

pub async fn google_login(
    pool: &PgPool,
    jwt_secret: &str,
    google: &GoogleAuth,
    id_token: &str,
) -> Result<Tokens, AppError> {
    let token_info = google.verify(id_token).await?;

    // The Google account id is the stable identity: the email of either side may change.
    if let Some(user) = users_repo::find_by_google_id(pool, &token_info.sub).await? {
        return issue_tokens(pool, jwt_secret, user.id, user.username).await;
    }

    // Past this point the email decides which account is used, so it must be proven.
    if !token_info.email_verified {
        return Err(AppError::Unauthorized(ErrorMessage::GoogleEmailNotVerified));
    }
    let email = token_info.email.trim().to_lowercase();

    if let Some(user) = users_repo::find_by_email(pool, &email).await? {
        // An unverified account may have been created by someone who does not
        // own this address; linking to it would let them into the victim's account.
        if !user.email_confirmed {
            return Err(AppError::Conflict(ErrorMessage::AccountEmailNotVerified));
        }
        users_repo::link_google_id(pool, user.id, &token_info.sub).await?;
        return issue_tokens(pool, jwt_secret, user.id, user.username).await;
    }

    let username = username_from_google(token_info.given_name.as_deref());
    let user_id = users_repo::create(
        pool,
        NewUser {
            username: &username,
            email: &email,
            password_hash: None,
            email_confirmed: true,
            google_id: Some(token_info.sub),
            privacy_level: PrivacyLevel::Public,
        },
    )
    .await?;

    issue_tokens(pool, jwt_secret, user_id, username).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn password_hashes_never_run_more_than_one_per_cpu() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<_> = (0..50)
            .map(|_| {
                let (running, peak) = (running.clone(), peak.clone());
                tokio::spawn(run_argon2(move || {
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    running.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                }))
            })
            .collect();
        for task in tasks {
            task.await.unwrap().unwrap();
        }

        let cpus = std::thread::available_parallelism().map_or(2, |cpus| cpus.get());
        assert!(peak.load(Ordering::SeqCst) <= cpus);
    }

    #[test]
    fn google_usernames_always_fit_the_column() {
        let long = username_from_google(Some("Maximilian-Alexander von Hohenzollern"));
        assert_eq!(long.chars().count(), USERNAME_MAX_CHARS);
        assert!(long.starts_with("MaximilianAlexand_"));

        assert!(username_from_google(None).starts_with("User_"));
        assert!(username_from_google(Some("  ")).starts_with("User_"));
        assert!(username_from_google(Some("Zoé")).starts_with("Zoé_"));
    }
}
