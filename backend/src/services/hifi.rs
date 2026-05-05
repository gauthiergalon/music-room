use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde_json::Value;

use crate::{
    dtos::hifi::{AlbumData, ArtistData, SearchResponse, TrackItem, TrackResponse},
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

    let url = format!("http://localhost:8000/track/?id={}&quality=HIGH", track_id);

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<TrackResponse>()
        .await?;

    Ok(response)
}

pub async fn get_track_info(pool: &sqlx::PgPool, track_id: i64) -> Result<TrackItem, AppError> {
    let track_record = sqlx::query!(
        "SELECT id, title, artist, album, duration, cover FROM tracks WHERE id = $1",
        track_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(t) = track_record {
        return Ok(TrackItem {
            id: t.id,
            title: t.title,
            duration: t.duration,
            audio_quality: None,
            album: Some(AlbumData {
                title: t.album,
                cover: t.cover,
            }),
            artists: Some(vec![ArtistData {
                name: Some(t.artist),
            }]),
        });
    }

    let client = Client::new();
    let url = format!("http://localhost:8000/info/?id={}", track_id);
    let info_payload = client.get(&url).send().await?.json::<Value>().await?;
    let track_data = info_payload.get("data").cloned().ok_or_else(|| {
        AppError::InternalError("Missing track data in info response".to_string())
    })?;
    let track_resp: TrackItem = serde_json::from_value(track_data)
        .map_err(|_| AppError::InternalError("Invalid track data in info response".to_string()))?;

    let title = track_resp.title.clone();
    let artist = track_resp
        .artists
        .as_ref()
        .and_then(|a| a.first())
        .and_then(|a| a.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let album_title = track_resp.album.as_ref().and_then(|a| a.title.clone());
    let cover_url = track_resp.album.as_ref().and_then(|a| a.cover.clone());
    let duration = track_resp.duration;

    sqlx::query!(
        "INSERT INTO tracks (id, title, artist, album, duration, cover) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
        track_id,
        title,
        artist,
        album_title,
        duration,
        cover_url
    )
    .execute(pool)
    .await?;

    Ok(track_resp)
}

pub async fn get_stream_url(track_id: i64) -> Result<String, AppError> {
    let track_response = get_track_details(track_id).await?;

    let manifest_b64 = track_response
        .data
        .get("manifest")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InternalError("Missing or invalid manifest field".to_string()))?;

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
