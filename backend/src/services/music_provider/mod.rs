pub mod hifi;

use crate::{
    dtos::music::{SearchResponse, TrackItem, TrackResponse},
    errors::AppError,
};
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait MusicProvider: Send + Sync {
    async fn search_tracks(&self, query: &str) -> Result<SearchResponse, AppError>;
    async fn get_track_details(&self, track_id: i64) -> Result<TrackResponse, AppError>;
    async fn get_track_info(&self, pool: &PgPool, track_id: i64) -> Result<TrackItem, AppError>;
    async fn get_stream_url(
        &self,
        track_id: i64,
        platform: Option<&str>,
    ) -> Result<String, AppError>;
}
