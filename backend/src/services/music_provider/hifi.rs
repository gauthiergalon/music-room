use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration as StdDuration, Instant},
};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use super::MusicProvider;
use crate::{
    dtos::music::{SearchResponse, TrackItem, TrackResponse},
    errors::{AppError, ErrorMessage},
};

const TRACK_MANIFEST_MIME_TYPE_DASH: &str = "application/dash+xml";
const TRACK_MANIFEST_MIME_TYPE_JSON: &str = "application/vnd.tidal.bts";
const TRACK_MANIFEST_TYPE_HLS: &str = "HLS";
const TRACK_MANIFEST_FORMAT: &str = "AACLC";

const CONNECT_TIMEOUT: StdDuration = StdDuration::from_secs(5);
const REQUEST_TIMEOUT: StdDuration = StdDuration::from_secs(15);
/// Every listener of a room asks for the same stream URL when a track starts;
/// a short cache turns that burst into one upstream resolution.
const STREAM_URL_TTL: StdDuration = StdDuration::from_secs(60);
const STREAM_CACHE_MAX_ENTRIES: usize = 1000;

pub struct HifiProvider {
    client: Client,
    host: String,
    /// Resolved stream URLs keyed by (track id, direct-URL preference).
    stream_urls: Mutex<HashMap<(i64, bool), (Instant, String)>>,
}

impl HifiProvider {
    /// `host` is the `host[:port]` of the HiFi API, without scheme.
    pub fn new(host: impl Into<String>) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(5)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("the HTTP client configuration is valid");

        Self {
            client,
            host: host.into(),
            stream_urls: Mutex::new(HashMap::new()),
        }
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var("HIFI_API_HOST").unwrap_or_else(|_| "localhost:8000".to_string()))
    }

    fn cached_stream_url(&self, key: (i64, bool)) -> Option<String> {
        let cache = self.stream_urls.lock().ok()?;
        cache
            .get(&key)
            .filter(|(stored_at, _)| stored_at.elapsed() < STREAM_URL_TTL)
            .map(|(_, url)| url.clone())
    }

    fn cache_stream_url(&self, key: (i64, bool), url: &str) {
        if let Ok(mut cache) = self.stream_urls.lock() {
            if cache.len() >= STREAM_CACHE_MAX_ENTRIES {
                cache.retain(|_, (stored_at, _)| stored_at.elapsed() < STREAM_URL_TTL);
            }
            cache.insert(key, (Instant::now(), url.to_string()));
        }
    }

    /// GET a HiFi endpoint and turn HTTP failures into the right API error.
    async fn get(&self, url: &str) -> Result<reqwest::Response, AppError> {
        let response = self.client.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(ErrorMessage::TrackNotFound));
        }
        Ok(response.error_for_status()?)
    }
}

#[async_trait]
impl MusicProvider for HifiProvider {
    async fn search_tracks(&self, query: &str) -> Result<SearchResponse, AppError> {
        let url = format!(
            "http://{}/search/?s={}",
            self.host,
            urlencoding::encode(query)
        );

        Ok(self.get(&url).await?.json::<SearchResponse>().await?)
    }

    async fn get_track_details(&self, track_id: i64) -> Result<TrackResponse, AppError> {
        let url = format!("http://{}/track/?id={}&quality=LOW", self.host, track_id);

        let response = self.get(&url).await?;
        let response = resolve_pending_response(&self.client, response, &self.host).await?;
        serde_json::from_value(response).map_err(|error| invalid_upstream(error.to_string()))
    }

    async fn get_track_info(&self, track_id: i64) -> Result<TrackItem, AppError> {
        let url = format!("http://{}/info/?id={}", self.host, track_id);
        let payload = self.get(&url).await?.json::<Value>().await?;

        let track_data = payload
            .get("data")
            .cloned()
            .ok_or_else(|| invalid_upstream("Missing track data in info response"))?;

        serde_json::from_value(track_data)
            .map_err(|_| invalid_upstream("Invalid track data in info response"))
    }

    async fn get_stream_url(
        &self,
        track_id: i64,
        platform: Option<&str>,
    ) -> Result<String, AppError> {
        let prefer_direct = matches!(platform, Some(value) if value.eq_ignore_ascii_case("web"));
        let cache_key = (track_id, prefer_direct);
        if let Some(url) = self.cached_stream_url(cache_key) {
            return Ok(url);
        }

        let url = self.resolve_stream_url(track_id, prefer_direct).await?;
        self.cache_stream_url(cache_key, &url);
        Ok(url)
    }
}

impl HifiProvider {
    async fn resolve_stream_url(
        &self,
        track_id: i64,
        prefer_direct: bool,
    ) -> Result<String, AppError> {
        let track_response = self.get_track_details(track_id).await?;
        let direct_url = Self::extract_direct_stream_url(&track_response.data)?;

        if prefer_direct && let Some(url) = direct_url.clone() {
            return Ok(url);
        }

        if track_response
            .data
            .get("manifestMimeType")
            .and_then(Value::as_str)
            == Some(TRACK_MANIFEST_MIME_TYPE_DASH)
            && let Some(url) = self.get_track_manifest_url(track_id).await?
        {
            return Ok(url);
        }

        if let Some(url) = direct_url {
            return Ok(url);
        }

        if let Some(url) = self.get_track_manifest_url(track_id).await? {
            return Ok(url);
        }

        Err(invalid_upstream("No playable stream URL could be resolved"))
    }

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
            .ok_or_else(|| invalid_upstream("Missing or invalid manifest field"))?;

        let manifest_bytes = STANDARD
            .decode(manifest_b64)
            .map_err(|_| invalid_upstream("Failed to decode manifest base64"))?;

        let manifest_json: Value = serde_json::from_slice(&manifest_bytes)
            .map_err(|_| invalid_upstream("Failed to parse manifest JSON"))?;

        let urls = manifest_json
            .get("urls")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_upstream("No URLs found in manifest"))?;

        let stream_url = urls
            .first()
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_upstream("No playable URL found in manifest"))?
            .to_string();

        Ok(Some(stream_url))
    }

    async fn get_track_manifest_url(&self, track_id: i64) -> Result<Option<String>, AppError> {
        let url = format!(
            "http://{}/trackManifests/?id={}&adaptive=true&manifestType={}&uriScheme=HTTPS&usage=PLAYBACK&formats={}",
            self.host, track_id, TRACK_MANIFEST_TYPE_HLS, TRACK_MANIFEST_FORMAT,
        );
        let response = self.get(&url).await?;
        let response = resolve_pending_response(&self.client, response, &self.host).await?;

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

/// The provider answered, but not with something usable.
fn invalid_upstream(reason: impl Into<String>) -> AppError {
    tracing::error!("HiFi API error: {}", reason.into());
    AppError::BadGateway(ErrorMessage::UpstreamUnavailable)
}

async fn resolve_pending_response(
    client: &Client,
    response: reqwest::Response,
    host: &str,
) -> Result<Value, AppError> {
    if response.status().as_u16() != 202 {
        return Ok(response.json::<Value>().await?);
    }

    let pending = response.json::<Value>().await?;
    let status_path = pending
        .get("statusUrl")
        .or_else(|| pending.get("status_url"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_upstream("HiFi playback request is pending without a status URL"))?;
    let status_url = if status_path.starts_with("http://") || status_path.starts_with("https://") {
        status_path.to_string()
    } else {
        format!("http://{}{}", host, status_path)
    };

    for _ in 0..30 {
        sleep(Duration::from_millis(500)).await;
        let poll = client.get(&status_url).send().await?;
        if poll.status().as_u16() == 202 {
            continue;
        }
        if !poll.status().is_success() {
            return Err(invalid_upstream(format!(
                "HiFi playback request failed with status {}",
                poll.status()
            )));
        }
        return Ok(poll.json::<Value>().await?);
    }

    Err(invalid_upstream("HiFi playback request timed out"))
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde_json::json;

    use super::HifiProvider;

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
