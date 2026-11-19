//! Relays HLS streams to browsers.
//!
//! The provider's audio CDN refuses (403) any request that carries an `Origin`
//! header, and a browser always sends one when it plays HLS, whether through
//! its own player or through hls.js. Native apps do not, so they keep talking
//! to the CDN directly. For the web client the backend fetches the playlists
//! and segments itself and serves them from its own origin.
//!
//! The relay is not an open proxy: every URL it accepts was produced by the
//! backend, carries an expiry and an HMAC signature, and must point at an
//! allowed host.

use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Url;
use serde::Deserialize;
use sha2::Sha256;

use crate::errors::{AppError, ErrorMessage};

type HmacSha256 = Hmac<Sha256>;

/// Mount point of the relay routes; signed links are relative to it.
pub const STREAM_PATH: &str = "/hifi/stream";
const PLAYLIST_ENDPOINT: &str = "playlist.m3u8";
const SEGMENT_ENDPOINT: &str = "segment";

/// How long a signed link stays valid: comfortably longer than any track.
const LINK_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Playlists are small text files; anything bigger is not a playlist.
const MAX_PLAYLIST_BYTES: usize = 2 * 1024 * 1024;

pub const DEFAULT_ALLOWED_HOSTS: &[&str] = &["tidal.com"];

/// Query string of a signed relay link.
#[derive(Debug, Deserialize)]
pub struct SignedLink {
    /// Upstream URL, base64url encoded.
    pub u: String,
    /// Expiry, in seconds since the Unix epoch.
    pub exp: i64,
    /// Hex HMAC-SHA256 of the expiry and the upstream URL.
    pub sig: String,
}

/// An upstream response relayed as is.
pub struct RelayedSegment {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: axum::body::Bytes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Playlist,
    Segment,
}

pub struct StreamProxy {
    key: Vec<u8>,
    allowed_hosts: Vec<String>,
    http: reqwest::Client,
}

impl StreamProxy {
    /// `secret` signs the links. `allowed_hosts` are the domains (and their
    /// subdomains) the relay may fetch from.
    pub fn new(secret: &str, allowed_hosts: Vec<String>) -> Self {
        // Derive a dedicated key so a relay signature can never be confused
        // with anything else signed with the same secret.
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any size");
        mac.update(b"music-room/stream-proxy/v1");

        Self {
            key: mac.finalize().into_bytes().to_vec(),
            allowed_hosts: allowed_hosts
                .into_iter()
                .map(|host| host.trim().to_lowercase())
                .filter(|host| !host.is_empty())
                .collect(),
            http: reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                // A redirect could lead outside the allowed hosts.
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("the HTTP client configuration is valid"),
        }
    }

    /// Reads `STREAM_PROXY_ALLOWED_HOSTS` (comma separated), defaulting to the
    /// music provider's domain.
    pub fn from_env(secret: &str) -> Self {
        let allowed_hosts = std::env::var("STREAM_PROXY_ALLOWED_HOSTS")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.split(',').map(str::to_string).collect())
            .unwrap_or_else(|| {
                DEFAULT_ALLOWED_HOSTS
                    .iter()
                    .map(|h| h.to_string())
                    .collect()
            });

        Self::new(secret, allowed_hosts)
    }

    /// Whether a stream URL has to go through the relay to be playable in a browser.
    pub fn is_hls(url: &str) -> bool {
        Url::parse(url).is_ok_and(|url| url.path().to_ascii_lowercase().ends_with(".m3u8"))
    }

    /// The path, relative to the API root, at which the browser can load the
    /// given upstream playlist through the relay.
    pub fn playlist_path(&self, upstream_url: &str) -> Result<String, AppError> {
        let url =
            Url::parse(upstream_url).map_err(|_| invalid_upstream("unparsable stream URL"))?;
        self.check_host(&url)
            .map_err(|_| invalid_upstream("stream URL on an unexpected host"))?;

        let expires_at = Utc::now().timestamp() + LINK_TTL.as_secs() as i64;
        Ok(format!(
            "{STREAM_PATH}/{}",
            self.link(Kind::Playlist, &url, expires_at)
        ))
    }

    /// Fetches a playlist and rewrites every URL in it to go through the relay.
    pub async fn playlist(&self, link: &SignedLink) -> Result<String, AppError> {
        let url = self.verify(link)?;

        let response = self.http.get(url.clone()).send().await?;
        if response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND
        {
            return Err(AppError::NotFound(ErrorMessage::TrackNotStreamable));
        }
        let body = response.error_for_status()?.bytes().await?;
        if body.len() > MAX_PLAYLIST_BYTES {
            return Err(invalid_upstream("playlist too large"));
        }
        let text =
            std::str::from_utf8(&body).map_err(|_| invalid_upstream("playlist is not text"))?;

        self.rewrite_playlist(text, &url, link.exp)
    }

    /// Fetches a media segment (or any other binary resource of the stream),
    /// forwarding the byte range the player asked for.
    pub async fn segment(
        &self,
        link: &SignedLink,
        range: Option<&HeaderValue>,
    ) -> Result<RelayedSegment, AppError> {
        let url = self.verify(link)?;

        // No `Origin` header is sent: that is the whole point of the relay.
        let mut request = self.http.get(url);
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        let response = request.send().await?;

        let status = response.status();
        if status == StatusCode::FORBIDDEN || status == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(ErrorMessage::TrackNotStreamable));
        }
        if !status.is_success() {
            return Err(invalid_upstream(format!(
                "segment request failed with {status}"
            )));
        }

        let mut headers = HeaderMap::new();
        for name in [
            header::CONTENT_TYPE,
            header::CONTENT_RANGE,
            header::ACCEPT_RANGES,
        ] {
            if let Some(value) = response.headers().get(&name) {
                headers.insert(name, value.clone());
            }
        }
        // The link expires; the bytes behind it never change.
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=3600"),
        );

        Ok(RelayedSegment {
            status,
            headers,
            body: response.bytes().await?,
        })
    }

    fn rewrite_playlist(
        &self,
        playlist: &str,
        base: &Url,
        expires_at: i64,
    ) -> Result<String, AppError> {
        let mut rewritten = String::with_capacity(playlist.len() * 2);
        // A URI line that follows a variant declaration is another playlist.
        let mut next_uri_is_playlist = false;

        for line in playlist.lines() {
            let line = line.trim();

            if line.is_empty() {
                rewritten.push('\n');
                continue;
            }

            if line.starts_with('#') {
                next_uri_is_playlist = line.starts_with("#EXT-X-STREAM-INF");
                rewritten.push_str(&self.rewrite_tag(line, base, expires_at)?);
                rewritten.push('\n');
                continue;
            }

            let kind = if next_uri_is_playlist || line.to_ascii_lowercase().contains(".m3u8") {
                Kind::Playlist
            } else {
                Kind::Segment
            };
            next_uri_is_playlist = false;

            rewritten.push_str(&self.relayed(kind, base, line, expires_at)?);
            rewritten.push('\n');
        }

        Ok(rewritten)
    }

    /// Rewrites the `URI="..."` attribute of a tag, if it has one.
    fn rewrite_tag(&self, tag: &str, base: &Url, expires_at: i64) -> Result<String, AppError> {
        const ATTRIBUTE: &str = "URI=\"";

        let Some(start) = tag.find(ATTRIBUTE).map(|index| index + ATTRIBUTE.len()) else {
            return Ok(tag.to_string());
        };
        let Some(length) = tag[start..].find('"') else {
            return Ok(tag.to_string());
        };

        // Alternative renditions and I-frame lists are playlists; an init
        // segment (`EXT-X-MAP`) or a key is fetched as bytes.
        let kind =
            if tag.starts_with("#EXT-X-MEDIA") || tag.starts_with("#EXT-X-I-FRAME-STREAM-INF") {
                Kind::Playlist
            } else {
                Kind::Segment
            };
        let relayed = self.relayed(kind, base, &tag[start..start + length], expires_at)?;

        Ok(format!(
            "{}{}{}",
            &tag[..start],
            relayed,
            &tag[start + length..]
        ))
    }

    /// The relay link for a URI found in a playlist fetched from `base`.
    fn relayed(
        &self,
        kind: Kind,
        base: &Url,
        uri: &str,
        expires_at: i64,
    ) -> Result<String, AppError> {
        let url = base
            .join(uri)
            .map_err(|_| invalid_upstream("unparsable URL in playlist"))?;
        self.check_host(&url)
            .map_err(|_| invalid_upstream("playlist points at an unexpected host"))?;

        Ok(self.link(kind, &url, expires_at))
    }

    /// `endpoint?u=..&exp=..&sig=..`, relative to [`STREAM_PATH`].
    fn link(&self, kind: Kind, url: &Url, expires_at: i64) -> String {
        let endpoint = match kind {
            Kind::Playlist => PLAYLIST_ENDPOINT,
            Kind::Segment => SEGMENT_ENDPOINT,
        };

        format!(
            "{endpoint}?u={}&exp={expires_at}&sig={}",
            URL_SAFE_NO_PAD.encode(url.as_str()),
            hex::encode(self.mac(url.as_str(), expires_at).finalize().into_bytes())
        )
    }

    fn mac(&self, url: &str, expires_at: i64) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts keys of any size");
        mac.update(expires_at.to_string().as_bytes());
        mac.update(b"\n");
        mac.update(url.as_bytes());
        mac
    }

    /// Checks the signature, the expiry and the target host of a link.
    fn verify(&self, link: &SignedLink) -> Result<Url, AppError> {
        let invalid = || AppError::Forbidden(ErrorMessage::StreamLinkInvalid);

        let url = URL_SAFE_NO_PAD
            .decode(&link.u)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .ok_or_else(invalid)?;
        let signature = hex::decode(&link.sig).map_err(|_| invalid())?;

        // Constant-time comparison.
        self.mac(&url, link.exp)
            .verify_slice(&signature)
            .map_err(|_| invalid())?;

        if link.exp < Utc::now().timestamp() {
            return Err(AppError::Forbidden(ErrorMessage::StreamLinkExpired));
        }

        let url = Url::parse(&url).map_err(|_| invalid())?;
        self.check_host(&url).map_err(|_| invalid())?;
        Ok(url)
    }

    /// The relay only talks to the provider's hosts, over HTTP(S).
    fn check_host(&self, url: &Url) -> Result<(), ()> {
        if !matches!(url.scheme(), "https" | "http") {
            return Err(());
        }
        let host = url.host_str().ok_or(())?.to_lowercase();

        let allowed = self.allowed_hosts.iter().any(|allowed| {
            host == *allowed
                || host
                    .strip_suffix(allowed.as_str())
                    .is_some_and(|prefix| prefix.ends_with('.'))
        });
        if allowed { Ok(()) } else { Err(()) }
    }
}

/// The provider answered, but not with something the relay can use.
fn invalid_upstream(reason: impl Into<String>) -> AppError {
    tracing::error!("Stream relay error: {}", reason.into());
    AppError::BadGateway(ErrorMessage::UpstreamUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proxy() -> StreamProxy {
        StreamProxy::new("secret", vec!["tidal.com".to_string()])
    }

    fn parse_link(link: &str) -> (String, SignedLink) {
        let (endpoint, query) = link.split_once('?').expect("a link has a query");
        let value = |name: &str| {
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix(&format!("{name}=")))
                .unwrap()
                .to_string()
        };
        (
            endpoint.to_string(),
            SignedLink {
                u: value("u"),
                exp: value("exp").parse().unwrap(),
                sig: value("sig"),
            },
        )
    }

    fn upstream_of(link: &SignedLink) -> String {
        String::from_utf8(URL_SAFE_NO_PAD.decode(&link.u).unwrap()).unwrap()
    }

    #[test]
    fn recognises_hls_urls() {
        assert!(StreamProxy::is_hls(
            "https://m.tidal.com/1/manifests/abc.m3u8"
        ));
        assert!(StreamProxy::is_hls("https://m.tidal.com/a.M3U8?token=1"));
        assert!(!StreamProxy::is_hls(
            "https://a.tidal.com/track.flac?x=.m3u8"
        ));
        assert!(!StreamProxy::is_hls("not a url"));
    }

    #[test]
    fn a_link_made_by_the_relay_verifies() {
        let proxy = proxy();
        let path = proxy
            .playlist_path("https://im-fa.manifest.tidal.com/1/manifests/abc.m3u8?x=1")
            .unwrap();
        let (endpoint, link) = parse_link(&path);

        assert_eq!(endpoint, "/hifi/stream/playlist.m3u8");
        assert_eq!(
            proxy.verify(&link).unwrap().as_str(),
            "https://im-fa.manifest.tidal.com/1/manifests/abc.m3u8?x=1"
        );
    }

    #[test]
    fn tampered_or_foreign_links_are_refused() {
        let proxy = proxy();
        let path = proxy
            .playlist_path("https://im-fa.manifest.tidal.com/a.m3u8")
            .unwrap();
        let (_, link) = parse_link(&path);

        // Another target with the original signature.
        let forged = SignedLink {
            u: URL_SAFE_NO_PAD.encode("https://internal.example.com/secret"),
            exp: link.exp,
            sig: link.sig.clone(),
        };
        assert!(proxy.verify(&forged).is_err());

        // A longer validity with the original signature.
        let extended = SignedLink {
            u: link.u.clone(),
            exp: link.exp + 1000,
            sig: link.sig.clone(),
        };
        assert!(proxy.verify(&extended).is_err());

        // A link signed with another secret.
        let other = StreamProxy::new("another secret", vec!["tidal.com".to_string()]);
        assert!(other.verify(&link).is_err());

        assert!(proxy.verify(&link).is_ok());
    }

    #[test]
    fn expired_links_are_refused() {
        let proxy = proxy();
        let url = Url::parse("https://im-fa.manifest.tidal.com/a.m3u8").unwrap();
        let (_, link) = parse_link(&proxy.link(Kind::Playlist, &url, Utc::now().timestamp() - 1));

        assert!(matches!(
            proxy.verify(&link),
            Err(AppError::Forbidden(ErrorMessage::StreamLinkExpired))
        ));
    }

    #[test]
    fn only_allowed_hosts_can_be_reached() {
        let proxy = proxy();
        let allowed = |url: &str| proxy.check_host(&Url::parse(url).unwrap()).is_ok();

        assert!(allowed("https://tidal.com/a"));
        assert!(allowed("https://sp-ad-fa.audio.tidal.com/a"));
        assert!(!allowed("https://eviltidal.com/a"));
        assert!(!allowed("https://tidal.com.evil.example/a"));
        assert!(!allowed("https://169.254.169.254/latest/meta-data"));
        assert!(!allowed("file:///etc/passwd"));

        // Even a correctly signed link cannot target another host.
        assert!(proxy.playlist_path("https://example.com/a.m3u8").is_err());
    }

    #[test]
    fn master_playlist_variants_become_relayed_playlists() {
        let proxy = proxy();
        let base = Url::parse("https://im-fa.manifest.tidal.com/1/manifests/master.m3u8").unwrap();
        let playlist = "#EXTM3U\r\n#EXT-X-VERSION:3\r\n#EXT-X-STREAM-INF:BANDWIDTH=341808,CODECS=\"mp4a.40.2\"\r\nhttps://im-fa.manifest.tidal.com/1/manifests/media-token\r\n";

        let rewritten = proxy
            .rewrite_playlist(playlist, &base, 4_000_000_000)
            .unwrap();
        let lines: Vec<&str> = rewritten.lines().collect();

        assert_eq!(lines[0], "#EXTM3U");
        assert_eq!(
            lines[2],
            "#EXT-X-STREAM-INF:BANDWIDTH=341808,CODECS=\"mp4a.40.2\""
        );
        // No `.m3u8` in the URL: it is a playlist because of the tag before it.
        let (endpoint, link) = parse_link(lines[3]);
        assert_eq!(endpoint, "playlist.m3u8");
        assert_eq!(
            upstream_of(&link),
            "https://im-fa.manifest.tidal.com/1/manifests/media-token"
        );
        assert!(proxy.verify(&link).is_ok());
    }

    #[test]
    fn media_playlist_segments_and_init_segment_are_relayed() {
        let proxy = proxy();
        let base = Url::parse("https://im-fa.manifest.tidal.com/1/manifests/media.m3u8").unwrap();
        let playlist = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MAP:URI=\"https://sp-ad-fa.audio.tidal.com/mediatracks/abc/0.mp4?token=t\"\n#EXTINF:3.994,\nhttps://sp-ad-fa.audio.tidal.com/mediatracks/abc/1.mp4?token=t\n#EXTINF:3.994,\nsegments/2.mp4\n#EXT-X-ENDLIST\n";

        let rewritten = proxy
            .rewrite_playlist(playlist, &base, 4_000_000_000)
            .unwrap();
        let lines: Vec<&str> = rewritten.lines().collect();

        // The init segment inside the tag attribute.
        let map = lines[2];
        assert!(map.starts_with("#EXT-X-MAP:URI=\"segment?u="));
        assert!(map.ends_with('"'));
        let (_, init) = parse_link(
            map.trim_start_matches("#EXT-X-MAP:URI=\"")
                .trim_end_matches('"'),
        );
        assert_eq!(
            upstream_of(&init),
            "https://sp-ad-fa.audio.tidal.com/mediatracks/abc/0.mp4?token=t"
        );

        // Absolute and relative segment URLs.
        let (endpoint, first) = parse_link(lines[4]);
        assert_eq!(endpoint, "segment");
        assert_eq!(
            upstream_of(&first),
            "https://sp-ad-fa.audio.tidal.com/mediatracks/abc/1.mp4?token=t"
        );
        let (_, second) = parse_link(lines[6]);
        assert_eq!(
            upstream_of(&second),
            "https://im-fa.manifest.tidal.com/1/manifests/segments/2.mp4"
        );

        // Tags without a URI are untouched.
        assert_eq!(lines[1], "#EXT-X-TARGETDURATION:4");
        assert_eq!(lines[7], "#EXT-X-ENDLIST");
    }

    #[test]
    fn a_playlist_pointing_outside_the_allowed_hosts_is_rejected() {
        let proxy = proxy();
        let base = Url::parse("https://im-fa.manifest.tidal.com/media.m3u8").unwrap();
        let playlist = "#EXTM3U\n#EXTINF:4,\nhttp://127.0.0.1:5432/\n";

        assert!(
            proxy
                .rewrite_playlist(playlist, &base, 4_000_000_000)
                .is_err()
        );
    }
}
