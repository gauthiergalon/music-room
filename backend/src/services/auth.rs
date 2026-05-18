use crate::{
    errors::{AppError, ErrorMessage},
    middleware::auth::Claims,
    models::refresh_token::NewRefreshToken,
    models::reset_token::NewResetToken,
    models::user::NewUser,
    repositories::{
        refresh_tokens as refresh_tokens_repo, reset_tokens as reset_tokens_repo,
        users as users_repo,
    },
    services::tokens::TokenPair,
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{TimeDelta, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::PgPool;

use uuid::Uuid;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AppError::Internal)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::Unauthorized(ErrorMessage::InvalidCredentials))
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

pub async fn register(
    pool: &PgPool,
    jwt_secret: &str,
    username: &str,
    email: &str,
    password: &str,
) -> Result<(String, String), AppError> {
    let username = username.trim();
    let email = email.trim().to_lowercase();
    let password_hash = hash_password(password)?;

    let user_id = users_repo::create(
        pool,
        NewUser {
            username,
            email: &email,
            password_hash: Some(password_hash),
            email_confirmed: false,
            google_id: None,
            favorite_genres: None,
            privacy_level: crate::models::user::PrivacyLevel::Friends,
        },
    )
    .await?;

    let access_token = generate_access_token(user_id, username.to_string(), jwt_secret)?;
    let refresh_token = store_refresh_token(pool, user_id).await?;

    Ok((access_token, refresh_token))
}

pub async fn login(
    pool: &PgPool,
    jwt_secret: &str,
    email: &str,
    password: &str,
) -> Result<(String, String), AppError> {
    let email = email.trim().to_lowercase();
    let user = users_repo::find_by_email(pool, &email)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    let password_hash = user
        .password_hash
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;
    verify_password(password, &password_hash)?;

    let access_token = generate_access_token(user.id, user.username.clone(), jwt_secret)?;
    let refresh_token = store_refresh_token(pool, user.id).await?;

    Ok((access_token, refresh_token))
}

pub async fn logout(pool: &PgPool, token: &str, user_id: &Uuid) -> Result<(), AppError> {
    let token_hash = TokenPair::hash(token);
    refresh_tokens_repo::delete(pool, token_hash, user_id).await?;
    Ok(())
}

pub async fn refresh(
    pool: &PgPool,
    jwt_secret: &str,
    token: &str,
) -> Result<(String, String), AppError> {
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
    let access_token = generate_access_token(stored.user_id, user.username, jwt_secret)?;
    let refresh_token = store_refresh_token(pool, stored.user_id).await?;

    Ok((access_token, refresh_token))
}

pub async fn forgot_password(pool: &PgPool, email: &str) -> Result<(), AppError> {
    let email_address = email.trim().to_lowercase();

    let user_model = users_repo::find_by_email(pool, &email_address)
        .await?
        .ok_or(AppError::NotFound(ErrorMessage::UserNotFound))?;

    if reset_tokens_repo::find_valid_by_user_id(pool, user_model.id)
        .await?
        .is_some()
    {
        return Err(AppError::TooManyRequests(ErrorMessage::TooManyEmails));
    }

    let token_pair = TokenPair::generate();
    let expires_at = Utc::now() + TimeDelta::minutes(15);
    let email = crate::services::email::Email::for_password_reset(&token_pair.plain);

    email.send(&email_address)?;
    reset_tokens_repo::create(
        pool,
        NewResetToken {
            token_hash: token_pair.hash,
            user_id: user_model.id,
            expires_at,
        },
    )
    .await?;

    Ok(())
}

pub async fn reset_password(
    pool: &PgPool,
    token: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let token_hash = TokenPair::hash(token);
    let password_hash = hash_password(new_password)?;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let stored = reset_tokens_repo::delete_and_return(&mut *tx, token_hash)
        .await?
        .ok_or(AppError::Unauthorized(ErrorMessage::TokenInvalid))?;

    if stored.expires_at < Utc::now() {
        return Err(AppError::Unauthorized(ErrorMessage::TokenExpired));
    }

    users_repo::update_password(&mut *tx, stored.user_id, password_hash).await?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok(())
}

#[derive(serde::Deserialize, Debug)]
pub struct GoogleTokenInfo {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub exp: usize,
}

#[derive(serde::Deserialize)]
struct JwkKey {
    kid: String,
    n: String,
    e: String,
}

#[derive(serde::Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

pub async fn google_login(
    pool: &PgPool,
    jwt_secret: &str,
    google_client_id: &str,
    _google_auth_url: &str, // Not strictly needed for verification if we fetch JWKS, but keeping signature
    id_token: &str,
) -> Result<(String, String), AppError> {
    let header = jsonwebtoken::decode_header(id_token)
        .map_err(|_| AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;
    let kid = header
        .kid
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    let jwks_url = "https://www.googleapis.com/oauth2/v3/certs";
    let resp = reqwest::get(jwks_url)
        .await
        .map_err(|_| AppError::Internal)?;
    let jwks: JwksResponse = resp.json().await.map_err(|_| AppError::Internal)?;

    let key = jwks
        .keys
        .into_iter()
        .find(|k| k.kid == kid)
        .ok_or(AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    let decoding_key = jsonwebtoken::DecodingKey::from_rsa_components(&key.n, &key.e)
        .map_err(|_| AppError::Internal)?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[google_client_id]);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);

    let token_data = jsonwebtoken::decode::<GoogleTokenInfo>(id_token, &decoding_key, &validation)
        .map_err(|_| AppError::Unauthorized(ErrorMessage::InvalidCredentials))?;

    let token_info = token_data.claims;

    let email = token_info.email.to_lowercase();
    let google_id = token_info.sub;

    let user_opt = users_repo::find_by_email(pool, &email).await?;

    let user_id = match user_opt {
        Some(user) => {
            if user.google_id != Some(google_id.clone()) {
                users_repo::link_google_id(pool, user.id, &google_id).await?;
            }
            user.id
        }
        None => {
            let base_username = token_info.given_name.unwrap_or_else(|| "User".to_string());
            let random_suffix = uuid::Uuid::new_v4()
                .to_string()
                .chars()
                .take(6)
                .collect::<String>();
            let username = format!("{}_{}", base_username, random_suffix);

            users_repo::create(
                pool,
                NewUser {
                    username: &username,
                    email: &email,
                    password_hash: None, // No password for Google users
                    email_confirmed: token_info.email_verified,
                    google_id: Some(google_id),
                    favorite_genres: None,
                    privacy_level: crate::models::user::PrivacyLevel::Public,
                },
            )
            .await?
        }
    };

    let user_model = users_repo::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::Internal)?;

    let access_token = generate_access_token(user_id, user_model.username, jwt_secret)?;
    let refresh_token = store_refresh_token(pool, user_id).await?;

    Ok((access_token, refresh_token))
}
