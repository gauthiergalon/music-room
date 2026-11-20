//! Shared helpers for the integration tests.
#![allow(dead_code)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use axum::http::{HeaderValue, header::AUTHORIZATION};
use axum_test::{TestServer, TestWebSocket};
use backend::{
    dtos::music::{AlbumData, ArtistData, SearchData, SearchResponse, TrackItem, TrackResponse},
    errors::{AppError, ErrorMessage},
    middleware::rate_limit::RateLimiter,
    routes::app_router,
    services::{
        email::MemoryMailer, google::GoogleAuth, music_provider::MusicProvider,
        stream_proxy::StreamProxy,
    },
    state::AppState,
    ws::manager::{DEFAULT_MAX_ROOMS, RoomManager},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub const JWT_SECRET: &str = "test_secret";
pub const GOOGLE_CLIENT_ID: &str = "test_client_id";
pub const PASSWORD: &str = "password123";
pub const PUBLIC_URL: &str = "https://music-room.test";

/// Tracks the provider does not know.
pub const UNKNOWN_TRACK_ID: i64 = 404_404;
/// Tracks for which the provider is down.
pub const FAILING_TRACK_ID: i64 = 502_502;
/// Tracks the provider knows but refuses to stream.
pub const UNSTREAMABLE_TRACK_ID: i64 = 403_403;

/// A music provider that answers from memory and counts metadata lookups.
#[derive(Default)]
pub struct MockMusicProvider {
    pub track_info_calls: AtomicUsize,
    /// When set, every track streams from this URL instead of a made-up one.
    pub stream_url: std::sync::Mutex<Option<String>>,
}

impl MockMusicProvider {
    pub fn track_info_calls(&self) -> usize {
        self.track_info_calls.load(Ordering::SeqCst)
    }

    fn track(track_id: i64) -> TrackItem {
        TrackItem {
            id: track_id,
            title: format!("Title {track_id}"),
            duration: 180,
            audio_quality: None,
            album: Some(AlbumData {
                title: Some(format!("Album {track_id}")),
                cover: Some("cover-id".to_string()),
            }),
            artists: Some(vec![ArtistData {
                name: Some(format!("Artist {track_id}")),
            }]),
        }
    }
}

#[async_trait]
impl MusicProvider for MockMusicProvider {
    async fn search_tracks(&self, _query: &str) -> Result<SearchResponse, AppError> {
        Ok(SearchResponse {
            version: "test".to_string(),
            data: SearchData {
                limit: 25,
                offset: 0,
                total_number_of_items: 1,
                items: vec![Self::track(1)],
            },
        })
    }

    async fn get_track_details(&self, _track_id: i64) -> Result<TrackResponse, AppError> {
        Ok(TrackResponse {
            version: "test".to_string(),
            data: json!({}),
        })
    }

    async fn get_track_info(&self, track_id: i64) -> Result<TrackItem, AppError> {
        self.track_info_calls.fetch_add(1, Ordering::SeqCst);
        match track_id {
            UNKNOWN_TRACK_ID => Err(AppError::NotFound(ErrorMessage::TrackNotFound)),
            FAILING_TRACK_ID => Err(AppError::BadGateway(ErrorMessage::UpstreamUnavailable)),
            _ => Ok(Self::track(track_id)),
        }
    }

    async fn get_stream_url(
        &self,
        track_id: i64,
        _platform: Option<&str>,
    ) -> Result<String, AppError> {
        if track_id == UNSTREAMABLE_TRACK_ID {
            return Err(AppError::NotFound(ErrorMessage::TrackNotStreamable));
        }
        if let Some(url) = self.stream_url.lock().unwrap().clone() {
            return Ok(url);
        }
        Ok(format!("https://stream.test/{track_id}"))
    }
}

/// Everything a test may want to tweak before the router is built.
pub struct TestConfig {
    pub google_jwks_url: String,
    pub owner_grace: Duration,
    pub max_rooms: usize,
    pub auth_rate_limit: Option<u32>,
    /// Hosts the stream relay may fetch from.
    pub stream_hosts: Vec<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            // Nothing listens here: a test that needs Google mounts a mock JWKS.
            google_jwks_url: "http://127.0.0.1:9/certs".to_string(),
            owner_grace: Duration::from_millis(300),
            max_rooms: DEFAULT_MAX_ROOMS,
            auth_rate_limit: None,
            stream_hosts: vec!["stream.test".to_string()],
        }
    }
}

pub struct TestApp {
    pub router: axum::Router,
    pub state: AppState,
    pub mailer: Arc<MemoryMailer>,
    pub music: Arc<MockMusicProvider>,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        Self::with_config(pool, TestConfig::default())
    }

    pub fn with_config(pool: PgPool, config: TestConfig) -> Self {
        let mailer = Arc::new(MemoryMailer::new());
        let music = Arc::new(MockMusicProvider::default());

        let state = AppState {
            pool,
            jwt_secret: JWT_SECRET.to_string(),
            google: Arc::new(GoogleAuth::new(GOOGLE_CLIENT_ID, config.google_jwks_url)),
            rooms: Arc::new(RoomManager::new(config.max_rooms, config.owner_grace)),
            music_provider: music.clone(),
            stream_proxy: Arc::new(StreamProxy::new(JWT_SECRET, config.stream_hosts)),
            mailer: mailer.clone(),
            public_url: PUBLIC_URL.to_string(),
            auth_rate_limiter: config
                .auth_rate_limit
                .map(|max| Arc::new(RateLimiter::new(max, Duration::from_secs(60)))),
        };

        Self {
            router: app_router(state.clone()).with_state(state.clone()),
            state,
            mailer,
            music,
        }
    }

    /// In-process server: enough for plain HTTP requests.
    pub fn server(&self) -> TestServer {
        TestServer::new(self.router.clone())
    }

    /// Server listening on a real socket, which WebSocket upgrades need.
    pub fn ws_server(&self) -> TestServer {
        TestServer::builder()
            .http_transport()
            .build(self.router.clone())
    }

    /// The token carried by the most recent email sent to `receiver`.
    pub fn last_email_token(&self, receiver: &str) -> Option<String> {
        self.mailer
            .sent()
            .into_iter()
            .rev()
            .find(|(to, _)| to == receiver)
            .and_then(|(_, email)| {
                let link = email.body.split("token=").nth(1)?;
                link.split_whitespace().next().map(str::to_string)
            })
    }

    /// Waits for an email to `receiver` and returns its token. Emails that
    /// answer a public request are sent in the background, after the response.
    pub async fn wait_for_email_token(&self, receiver: &str) -> String {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(token) = self.last_email_token(receiver) {
                return token;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "no email was sent to {receiver}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

/// The router alone, for tests that only speak HTTP.
pub fn create_app(pool: PgPool) -> axum::Router {
    TestApp::new(pool).router
}

pub fn bearer(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("Bearer {token}")).expect("token is a valid header value")
}

pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
}

/// Registers a user with a unique name and returns its session.
pub async fn register(server: &TestServer, name: &str) -> TestUser {
    let suffix = &Uuid::new_v4().simple().to_string()[..8];
    let username = format!("{name}_{suffix}");
    let email = format!("{username}@example.com");

    let response = server
        .post("/auth/register")
        .json(&json!({ "username": username, "email": email, "password": PASSWORD }))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    let tokens = response.json::<Value>();
    let access_token = tokens["access_token"].as_str().unwrap().to_string();

    let me = server
        .get("/users/me")
        .add_header(AUTHORIZATION, bearer(&access_token))
        .await
        .json::<Value>();

    TestUser {
        id: me["id"].as_str().unwrap().parse().unwrap(),
        username,
        email,
        access_token,
        refresh_token: tokens["refresh_token"].as_str().unwrap().to_string(),
    }
}

/// Creates the user's room and returns its id.
pub async fn create_room(server: &TestServer, user: &TestUser) -> Uuid {
    let response = server
        .post("/rooms")
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    response.json::<Value>()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

pub async fn add_track(server: &TestServer, user: &TestUser, room_id: Uuid, track_id: i64) {
    server
        .post(&format!("/rooms/{room_id}/queue"))
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .json(&json!({ "track_id": track_id }))
        .await
        .assert_status(axum::http::StatusCode::NO_CONTENT);
}

// ---------------------------------------------------------------- WebSocket

const WS_TIMEOUT: Duration = Duration::from_secs(5);

/// Opens the room's WebSocket and consumes the `Welcome` message.
/// Returns the socket and the connection id the server assigned.
pub async fn connect(server: &TestServer, user: &TestUser, room_id: Uuid) -> (TestWebSocket, Uuid) {
    let mut socket = server
        .get_websocket(&format!("/rooms/{room_id}/ws"))
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .add_header("x-device", "Test device")
        .await
        .into_websocket()
        .await;

    let welcome = next_event(&mut socket).await;
    assert_eq!(welcome["type"], "Welcome", "first message must be Welcome");
    let connection_id = welcome["payload"]["connection_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    (socket, connection_id)
}

/// The next JSON event, failing the test if nothing arrives in time.
pub async fn next_event(socket: &mut TestWebSocket) -> Value {
    tokio::time::timeout(WS_TIMEOUT, socket.receive_json::<Value>())
        .await
        .expect("timed out waiting for a WebSocket event")
}

/// The payload of the next event of the given type, skipping the others.
pub async fn next_of_type(socket: &mut TestWebSocket, event_type: &str) -> Value {
    loop {
        let event = next_event(socket).await;
        if event["type"] == event_type {
            return event["payload"].clone();
        }
    }
}

/// Asserts that no event of the given type arrives within `wait`.
pub async fn expect_no_event_of_type(socket: &mut TestWebSocket, event_type: &str, wait: Duration) {
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        match tokio::time::timeout_at(deadline, socket.receive_json::<Value>()).await {
            Err(_) => return,
            Ok(event) => assert_ne!(
                event["type"], event_type,
                "unexpected {event_type} event: {event}"
            ),
        }
    }
}

pub async fn send_event(socket: &mut TestWebSocket, event_type: &str, payload: Value) {
    socket
        .send_json(&json!({ "type": event_type, "payload": payload }))
        .await;
}

// ------------------------------------------------------------------- Google

pub mod google {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use serde_json::{Value, json};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    const KEY_ID: &str = "test-key";
    const PRIVATE_KEY_PEM: &[u8] = include_bytes!("../fixtures/google_test_key.pem");
    /// Public modulus of the test key above (base64url), as Google publishes its keys.
    const MODULUS: &str = "ymfUT55vpBnzyAKGkVVoI1LXXTUNP7l_PI_mZq-t0UoZqJNhHVd0I-Rj3ZGqNguxR1Lmjto4wXquR_H0u9KQ5HTc72jXBRn0MY5CHmuzwDv5zIH81qBZsfe-vOH8rNrNCfce-E_Hh1vo_GXczLpRNumxlgEHojx8ps88_iTPrHLSM2kjagtQ4Upj263bkxvLFpxx_s5Qi_xzFOOqTKyI_6fZ8-2XSjmBaxBLER0Xpziwq43doSuyrpkIkiAnLyATG6huQiyFjaeE53MM8HWt80922kCUVw9Yt2CMyXlKTtZlWmZ0lxgaveIDcbOceN3EL6yDn3ANkpMYqVktYYExhQ";

    /// Starts a server that publishes the test signing key like Google does.
    /// Returns it with the JWKS URL to configure the app with.
    pub async fn mock_jwks() -> (MockServer, String) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/certs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "keys": [{ "kty": "RSA", "alg": "RS256", "use": "sig", "kid": KEY_ID, "n": MODULUS, "e": "AQAB" }]
            })))
            .mount(&server)
            .await;

        let url = format!("{}/certs", server.uri());
        (server, url)
    }

    /// Claims of a valid ID token for the test client; override fields as needed.
    pub fn claims(sub: &str, email: &str) -> Value {
        json!({
            "iss": "https://accounts.google.com",
            "aud": super::GOOGLE_CLIENT_ID,
            "sub": sub,
            "email": email,
            "email_verified": true,
            "exp": (chrono::Utc::now() + chrono::Duration::minutes(15)).timestamp(),
            "name": "Test Google",
            "given_name": "Test",
            "family_name": "Google",
        })
    }

    /// Signs the claims with the test key, as Google would.
    pub fn sign(claims: &Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(KEY_ID.to_string());
        encode(
            &header,
            claims,
            &EncodingKey::from_rsa_pem(PRIVATE_KEY_PEM).expect("test key is a valid RSA key"),
        )
        .expect("signing a test token works")
    }
}
