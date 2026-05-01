use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub version: String,
    pub data: SearchData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchData {
    pub limit: u32,
    pub offset: u32,
    pub total_number_of_items: u32,
    pub items: Vec<TrackItem>, // Restricted to tracks only!
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackItem {
    pub id: i64,
    pub title: String,
    pub duration: i32,
    pub audio_quality: Option<String>,
    pub album: Option<AlbumData>,
    pub artists: Option<Vec<ArtistData>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumData {
    pub title: Option<String>,
    pub cover: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistData {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackResponse {
    pub version: String,
    pub data: Value, // The API returns various manifest data
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamUrlResponse {
    pub stream_url: String,
}
