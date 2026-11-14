//! Verification of Google ID tokens against Google's published signing keys.

use std::time::{Duration, Instant};

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::errors::{AppError, ErrorMessage};

pub const DEFAULT_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const JWKS_TTL: Duration = Duration::from_secs(60 * 60);
const JWKS_TIMEOUT: Duration = Duration::from_secs(5);
const ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];

#[derive(Deserialize, Debug)]
pub struct GoogleTokenInfo {
    pub sub: String,
    pub email: String,
    #[serde(default)]
    pub email_verified: bool,
    pub given_name: Option<String>,
}

#[derive(Deserialize, Clone)]
struct JwkKey {
    kid: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

struct CachedKeys {
    fetched_at: Instant,
    keys: Vec<JwkKey>,
}

pub struct GoogleAuth {
    client_id: String,
    jwks_url: String,
    http: reqwest::Client,
    cache: RwLock<Option<CachedKeys>>,
}

impl GoogleAuth {
    pub fn new(client_id: impl Into<String>, jwks_url: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            jwks_url: jwks_url.into(),
            http: reqwest::Client::builder()
                .timeout(JWKS_TIMEOUT)
                .build()
                .expect("the HTTP client configuration is valid"),
            cache: RwLock::new(None),
        }
    }

    /// Checks the signature, audience, issuer and expiry of an ID token.
    pub async fn verify(&self, id_token: &str) -> Result<GoogleTokenInfo, AppError> {
        let invalid = || AppError::Unauthorized(ErrorMessage::InvalidCredentials);

        let kid = decode_header(id_token)
            .map_err(|_| invalid())?
            .kid
            .ok_or_else(invalid)?;
        let key = self.signing_key(&kid).await?.ok_or_else(invalid)?;

        let decoding_key =
            DecodingKey::from_rsa_components(&key.n, &key.e).map_err(|_| invalid())?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&ISSUERS);

        decode::<GoogleTokenInfo>(id_token, &decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| invalid())
    }

    /// Looks the key up in the cache; the keys are downloaded again when the
    /// cache is older than an hour or does not know the key (Google rotates them).
    async fn signing_key(&self, kid: &str) -> Result<Option<JwkKey>, AppError> {
        if let Some(cached) = self.cache.read().await.as_ref()
            && cached.fetched_at.elapsed() < JWKS_TTL
            && let Some(key) = cached.keys.iter().find(|key| key.kid == kid)
        {
            return Ok(Some(key.clone()));
        }

        let keys = self
            .http
            .get(&self.jwks_url)
            .send()
            .await?
            .error_for_status()?
            .json::<JwksResponse>()
            .await?
            .keys;

        let key = keys.iter().find(|key| key.kid == kid).cloned();
        *self.cache.write().await = Some(CachedKeys {
            fetched_at: Instant::now(),
            keys,
        });

        Ok(key)
    }
}
