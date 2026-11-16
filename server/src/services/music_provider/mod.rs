pub mod hifi;

use async_trait::async_trait;

use crate::{
    dtos::music::{SearchResponse, TrackItem, TrackResponse},
    errors::AppError,
};

#[async_trait]
pub trait MusicProvider: Send + Sync {
    async fn search_tracks(&self, query: &str) -> Result<SearchResponse, AppError>;
    async fn get_track_details(&self, track_id: i64) -> Result<TrackResponse, AppError>;
    /// Metadata of one track, fetched from the provider (no caching here).
    async fn get_track_info(&self, track_id: i64) -> Result<TrackItem, AppError>;
    async fn get_stream_url(
        &self,
        track_id: i64,
        platform: Option<&str>,
    ) -> Result<String, AppError>;
}
