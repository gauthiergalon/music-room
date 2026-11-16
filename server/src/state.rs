use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    middleware::rate_limit::RateLimiter,
    services::{
        email::Mailer, google::GoogleAuth, music_provider::MusicProvider, stream_proxy::StreamProxy,
    },
    ws::manager::RoomManager,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub google: Arc<GoogleAuth>,
    /// Presence of connected clients. The database decides whether a room exists.
    pub rooms: Arc<RoomManager>,
    pub music_provider: Arc<dyn MusicProvider>,
    /// Relays HLS streams to browsers, which the audio CDN refuses to serve.
    pub stream_proxy: Arc<StreamProxy>,
    pub mailer: Arc<dyn Mailer>,
    /// `None` disables rate limiting on the public auth routes.
    pub auth_rate_limiter: Option<Arc<RateLimiter>>,
}
