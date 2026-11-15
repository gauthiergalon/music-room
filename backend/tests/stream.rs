//! The HLS relay the web client plays through.

mod common;

use axum::http::{StatusCode, header};
use common::{TestApp, TestConfig, bearer, register};
use serde_json::Value;
use sqlx::PgPool;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header_exists, method, path},
};

const MASTER: &str = "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-STREAM-INF:BANDWIDTH=341808,CODECS=\"mp4a.40.2\"\nmedia.m3u8\n";
const SEGMENT_BYTES: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// A CDN that behaves like the provider's: it serves playlists and segments,
/// but answers 403 to anything that carries an `Origin` header, which is what
/// every browser request does.
async fn fake_cdn() -> MockServer {
    let cdn = MockServer::start().await;
    let media = format!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MAP:URI=\"{0}/audio/0.mp4?token=t\"\n#EXTINF:3.994,\n{0}/audio/1.mp4?token=t\n#EXT-X-ENDLIST\n",
        cdn.uri()
    );

    Mock::given(header_exists("origin"))
        .respond_with(ResponseTemplate::new(403))
        .with_priority(1)
        .mount(&cdn)
        .await;
    Mock::given(method("GET"))
        .and(path("/manifests/master.m3u8"))
        .respond_with(ResponseTemplate::new(200).set_body_string(MASTER))
        .mount(&cdn)
        .await;
    Mock::given(method("GET"))
        .and(path("/manifests/media.m3u8"))
        .respond_with(ResponseTemplate::new(200).set_body_string(media))
        .mount(&cdn)
        .await;
    Mock::given(method("GET"))
        .and(path("/audio/0.mp4"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mp4")
                .set_body_bytes(b"init".to_vec()),
        )
        .mount(&cdn)
        .await;
    Mock::given(method("GET"))
        .and(path("/audio/1.mp4"))
        .and(header_exists("range"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header("content-type", "audio/mp4")
                .insert_header("content-range", "bytes 0-9/36")
                .set_body_bytes(SEGMENT_BYTES[..10].to_vec()),
        )
        .with_priority(2)
        .mount(&cdn)
        .await;
    Mock::given(method("GET"))
        .and(path("/audio/1.mp4"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mp4")
                .insert_header("accept-ranges", "bytes")
                .set_body_bytes(SEGMENT_BYTES.to_vec()),
        )
        .mount(&cdn)
        .await;

    cdn
}

/// An app whose every track streams from the fake CDN.
async fn app_with_cdn(pool: PgPool) -> (TestApp, MockServer) {
    let cdn = fake_cdn().await;
    let app = TestApp::with_config(
        pool,
        TestConfig {
            stream_hosts: vec!["127.0.0.1".to_string()],
            ..TestConfig::default()
        },
    );
    *app.music.stream_url.lock().unwrap() = Some(format!("{}/manifests/master.m3u8", cdn.uri()));
    (app, cdn)
}

async fn stream_url(server: &axum_test::TestServer, token: &str, platform: &str) -> String {
    let response = server
        .get("/hifi/track/1/stream-url")
        .add_header(header::AUTHORIZATION, bearer(token))
        .add_header("x-platform", platform)
        .await;
    response.assert_status(StatusCode::OK);
    response.json::<Value>()["stream_url"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The URI lines of a playlist (everything that is not a tag).
fn uris(playlist: &str) -> Vec<&str> {
    playlist
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[sqlx::test]
async fn native_apps_get_the_provider_url_and_browsers_get_the_relay(pool: PgPool) {
    let (app, cdn) = app_with_cdn(pool).await;
    let server = app.server();
    let user = register(&server, "listener").await;

    let android = stream_url(&server, &user.access_token, "android").await;
    assert_eq!(android, format!("{}/manifests/master.m3u8", cdn.uri()));

    let web = stream_url(&server, &user.access_token, "web").await;
    assert!(
        web.starts_with("/hifi/stream/playlist.m3u8?u="),
        "unexpected web stream URL: {web}"
    );
}

#[sqlx::test]
async fn a_browser_can_play_the_whole_stream_through_the_relay(pool: PgPool) {
    let (app, _cdn) = app_with_cdn(pool).await;
    let server = app.server();
    let user = register(&server, "listener").await;
    // What a browser sends with every request of a cross-origin stream.
    let origin = "http://localhost:36495";

    // 1. The master playlist. No access token: a media element cannot send one.
    let master_path = stream_url(&server, &user.access_token, "web").await;
    let master = server.get(&master_path).add_header("origin", origin).await;
    master.assert_status(StatusCode::OK);
    assert_eq!(
        master.header(header::CONTENT_TYPE),
        "application/vnd.apple.mpegurl"
    );
    let master = master.text();
    let media_link = uris(&master)[0];
    assert!(media_link.starts_with("playlist.m3u8?u="));

    // 2. The media playlist it points to, resolved like a player would.
    let media = server
        .get(&format!("/hifi/stream/{media_link}"))
        .add_header("origin", origin)
        .await;
    media.assert_status(StatusCode::OK);
    let media = media.text();
    assert!(media.contains("#EXT-X-TARGETDURATION:4"));
    assert!(media.contains("#EXT-X-ENDLIST"));

    // 3. The init segment referenced by EXT-X-MAP.
    let map_line = media
        .lines()
        .find(|line| line.starts_with("#EXT-X-MAP"))
        .unwrap();
    let init_link = map_line
        .split("URI=\"")
        .nth(1)
        .unwrap()
        .trim_end_matches('"');
    let init = server
        .get(&format!("/hifi/stream/{init_link}"))
        .add_header("origin", origin)
        .await;
    init.assert_status(StatusCode::OK);
    assert_eq!(init.as_bytes().as_ref(), b"init");

    // 4. The audio segment, whole and by range.
    let segment_link = uris(&media)[0];
    assert!(segment_link.starts_with("segment?u="));
    let segment = server
        .get(&format!("/hifi/stream/{segment_link}"))
        .add_header("origin", origin)
        .await;
    segment.assert_status(StatusCode::OK);
    assert_eq!(segment.header(header::CONTENT_TYPE), "audio/mp4");
    assert_eq!(segment.as_bytes().as_ref(), SEGMENT_BYTES);

    let ranged = server
        .get(&format!("/hifi/stream/{segment_link}"))
        .add_header("origin", origin)
        .add_header("range", "bytes=0-9")
        .await;
    ranged.assert_status(StatusCode::PARTIAL_CONTENT);
    assert_eq!(ranged.header(header::CONTENT_RANGE), "bytes 0-9/36");
    assert_eq!(ranged.as_bytes().as_ref(), &SEGMENT_BYTES[..10]);
}

#[sqlx::test]
async fn the_relay_refuses_links_it_did_not_sign(pool: PgPool) {
    let (app, cdn) = app_with_cdn(pool).await;
    let server = app.server();
    let user = register(&server, "listener").await;
    let genuine = stream_url(&server, &user.access_token, "web").await;

    // No signature at all.
    server
        .get("/hifi/stream/segment")
        .await
        .assert_status(StatusCode::BAD_REQUEST);

    // A different target with a genuine signature: the relay is not an open proxy.
    let query = genuine.split_once('?').unwrap().1;
    let value = |name: &str| {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(&format!("{name}=")))
            .unwrap()
    };
    let other_target = {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        URL_SAFE_NO_PAD.encode(format!("{}/audio/1.mp4", cdn.uri()))
    };
    server
        .get(&format!(
            "/hifi/stream/segment?u={other_target}&exp={}&sig={}",
            value("exp"),
            value("sig")
        ))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    // A tampered signature.
    server
        .get(&format!(
            "/hifi/stream/playlist.m3u8?u={}&exp={}&sig={}",
            value("u"),
            value("exp"),
            "00".repeat(32)
        ))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    // The genuine link still works.
    server.get(&genuine).await.assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn a_stream_the_provider_refuses_is_reported_as_unavailable(pool: PgPool) {
    let (app, cdn) = app_with_cdn(pool).await;
    let server = app.server();
    let user = register(&server, "listener").await;

    // The playlist exists for the provider's API but the CDN refuses it.
    *app.music.stream_url.lock().unwrap() = Some(format!("{}/manifests/gone.m3u8", cdn.uri()));
    let link = stream_url(&server, &user.access_token, "web").await;

    let response = server.get(&link).await;
    response.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(
        response.json::<Value>()["details"][0],
        "This track is not available for streaming"
    );
}

#[sqlx::test]
async fn the_raw_provider_track_endpoint_is_not_exposed(pool: PgPool) {
    let (app, _cdn) = app_with_cdn(pool).await;
    let server = app.server();
    let user = register(&server, "listener").await;

    // Only the stream URL is served; the provider's manifest stays internal.
    server
        .get("/hifi/track/1")
        .add_header(header::AUTHORIZATION, bearer(&user.access_token))
        .await
        .assert_status(StatusCode::NOT_FOUND);
    server
        .get("/hifi/track/1/stream-url")
        .add_header(header::AUTHORIZATION, bearer(&user.access_token))
        .await
        .assert_status(StatusCode::OK);
}
