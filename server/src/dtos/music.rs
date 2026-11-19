use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResponse {
    pub version: String,
    pub data: SearchData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct SearchData {
    pub limit: u32,
    pub offset: u32,
    pub total_number_of_items: u32,
    pub items: Vec<TrackItem>, // Restricted to tracks only!
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct TrackItem {
    pub id: i64,
    pub title: String,
    pub duration: i32,
    pub audio_quality: Option<String>,
    pub album: Option<AlbumData>,
    pub artists: Option<Vec<ArtistData>>,
}

impl TrackItem {
    /// Builds a track from the metadata cached in the `tracks` table. A track
    /// that is not cached still gets a displayable placeholder.
    pub fn from_cache(
        id: i64,
        title: Option<String>,
        artist: Option<String>,
        album: Option<String>,
        cover: Option<String>,
        duration: Option<i32>,
    ) -> Self {
        Self {
            id,
            title: title.unwrap_or_else(|| format!("Track {id}")),
            duration: duration.unwrap_or(0),
            audio_quality: None,
            album: (album.is_some() || cover.is_some()).then_some(AlbumData {
                title: album,
                cover,
            }),
            artists: artist.map(|name| vec![ArtistData { name: Some(name) }]),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct AlbumData {
    pub title: Option<String>,
    pub cover: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct ArtistData {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct TrackResponse {
    pub version: String,
    pub data: Value, // The API returns various manifest data
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StreamUrlResponse {
    pub stream_url: String,
}
