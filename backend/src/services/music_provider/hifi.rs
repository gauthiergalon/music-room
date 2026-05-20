use std::sync::OnceLock;
use async_trait::async_trait;

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde_json::Value;
use urlencoding;

use crate::{
    dtos::music::{AlbumData, ArtistData, SearchResponse, TrackItem, TrackResponse},
    errors::AppError,
};
use super::MusicProvider;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
static HIFI_HOST: OnceLock<String> = OnceLock::new();

const TRACK_MANIFEST_MIME_TYPE_DASH: &str = "application/dash+xml";
const TRACK_MANIFEST_MIME_TYPE_JSON: &str = "application/vnd.tidal.bts";
const TRACK_MANIFEST_TYPE_HLS: &str = "HLS";
const TRACK_MANIFEST_FORMAT: &str = "AACLC";

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(5)
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(
                    "Failed to create optimized HTTP client, using default: {}",
                    e
                );
                Client::new()
            })
    })
}

fn get_hifi_host() -> &'static str {
    HIFI_HOST
        .get_or_init(|| std::env::var("HIFI_API_HOST").unwrap_or_else(|_| "localhost".to_string()))
}

pub struct HifiProvider;

impl HifiProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MusicProvider for HifiProvider {
    async fn search_tracks(&self, query: &str) -> Result<SearchResponse, AppError> {
        let client = get_http_client();
        let host = get_hifi_host();

        let url = format!(
            "http://{}:8000/search/?s={}",
            host,
            urlencoding::encode(query)
        );

        let response = client
            .get(&url)
            .send()
            .await?
            .json::<SearchResponse>()
            .await?;

        Ok(response)
    }

    async fn get_track_details(&self, track_id: i64) -> Result<TrackResponse, AppError> {
        let client = get_http_client();
        let host = get_hifi_host();

        let url = format!("http://{}:8000/track/?id={}&quality=HIGH", host, track_id);

        let response = client
            .get(&url)
            .send()
            .await?
            .json::<TrackResponse>()
            .await?;

        Ok(response)
    }

    async fn get_track_info(&self, pool: &sqlx::PgPool, track_id: i64) -> Result<TrackItem, AppError> {
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

        let client = get_http_client();
        let host = get_hifi_host();
        let url = format!("http://{}:8000/info/?id={}", host, track_id);
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

    async fn get_stream_url(
        &self,
        track_id: i64,
        platform: Option<&str>,
    ) -> Result<String, AppError> {
        let track_response = self.get_track_details(track_id).await?;

        let prefer_direct = matches!(platform, Some(value) if value.eq_ignore_ascii_case("web"));
        let direct_url = Self::extract_direct_stream_url(&track_response.data)?;

        if prefer_direct {
            if let Some(url) = direct_url.clone() {
                return Ok(url);
            }
        }

        if track_response
            .data
            .get("manifestMimeType")
            .and_then(Value::as_str)
            == Some(TRACK_MANIFEST_MIME_TYPE_DASH)
        {
            if let Some(url) = self.get_track_manifest_url(track_id).await? {
                return Ok(url);
            }
        }

        if let Some(url) = direct_url {
            return Ok(url);
        }

        if let Some(url) = self.get_track_manifest_url(track_id).await? {
            return Ok(url);
        }

        Err(AppError::InternalError(
            "No playable stream URL could be resolved".to_string(),
        ))
    }
}

impl HifiProvider {
    fn extract_direct_stream_url(track_data: &Value) -> Result<Option<String>, AppError> {
        let manifest_mime_type = track_data
            .get("manifestMimeType")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if manifest_mime_type != TRACK_MANIFEST_MIME_TYPE_JSON {
            return Ok(None);
        }

        let manifest_b64 = track_data
            .get("manifest")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::InternalError("Missing or invalid manifest field".to_string())
            })?;

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
            .ok_or_else(|| {
                AppError::InternalError("No playable URL found in manifest".to_string())
            })?
            .to_string();

        Ok(Some(stream_url))
    }

    async fn get_track_manifest_url(&self, track_id: i64) -> Result<Option<String>, AppError> {
        let client = get_http_client();
        let host = get_hifi_host();

        let url = format!(
            "http://{}:8000/trackManifests/?id={}&adaptive=true&manifestType={}&uriScheme=HTTPS&usage=PLAYBACK&formats={}",
            host,
            track_id,
            TRACK_MANIFEST_TYPE_HLS,
            TRACK_MANIFEST_FORMAT,
        );
        let response = client
            .get(&url)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let manifest_url = response
            .get("data")
            .and_then(|value| value.get("data"))
            .and_then(|value| value.get("attributes"))
            .and_then(|value| value.get("uri").or_else(|| value.get("url")))
            .and_then(Value::as_str)
            .map(str::to_string);

        Ok(manifest_url)
    }
}

#[cfg(test)]
mod tests {
    use super::HifiProvider;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde_json::json;

    #[test]
    fn extracts_direct_stream_url_from_bts_manifest() {
        let track_data = json!({
            "manifestMimeType": "application/vnd.tidal.bts",
            "manifest": STANDARD.encode(r#"{"mimeType":"audio/flac","codecs":"flac","encryptionType":"NONE","urls":["https://example.com/audio.flac"]}"#)
        });

        let stream_url = HifiProvider::extract_direct_stream_url(&track_data)
            .expect("manifest should parse")
            .expect("url should exist");

        assert_eq!(stream_url, "https://example.com/audio.flac");
    }

    #[test]
    fn ignores_dash_manifests_for_direct_url_extraction() {
        let track_data = json!({
            "manifestMimeType": "application/dash+xml",
            "manifest": "PG1wZD48L21wZD4="
        });

        let stream_url = HifiProvider::extract_direct_stream_url(&track_data)
            .expect("manifest detection should not fail");

        assert!(stream_url.is_none());
    }
}
