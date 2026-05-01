use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde_json::{Value, json};

use crate::{
    dtos::hifi::{SearchResponse, TrackResponse},
    errors::AppError,
};

pub async fn search_tracks(query: &str) -> Result<SearchResponse, AppError> {
    let client = Client::new();

    let url = format!("http://localhost:8000/search/?s={}", query);

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<SearchResponse>()
        .await?;

    Ok(response)
}

pub async fn get_track_details(track_id: i64) -> Result<TrackResponse, AppError> {
    let client = Client::new();

    let url = format!("http://localhost:8000/track/?id={}", track_id);

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<TrackResponse>()
        .await?;

    Ok(response)
}

pub async fn get_stream_url(track_id: i64) -> Result<String, AppError> {
    let track_response = get_track_details(track_id).await?;

    let manifest_b64 = track_response
        .data
        .get("manifest")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InternalError("Missing or invalid manifest field".to_string()))?;

    // Decode base64 manifest
    let manifest_bytes = STANDARD
        .decode(manifest_b64)
        .map_err(|_| AppError::InternalError("Failed to decode manifest base64".to_string()))?;

    let manifest_json: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| AppError::InternalError("Failed to parse manifest JSON".to_string()))?;

    let urls = manifest_json
        .get("urls")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InternalError("No URLs found in manifest".to_string()))?;

    let stream_url = urls
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InternalError("No playable URL found in manifest".to_string()))?
        .to_string();

    Ok(stream_url)
}
