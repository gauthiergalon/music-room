use std::{collections::HashMap, sync::Arc};

use axum::http::StatusCode;
use axum_test::TestServer;
use backend::{routes::app_router, state::AppState};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Debug)]
struct TestAuthResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TestRoomResponse {
    id: String,
    owner_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TestUserResponse {
    id: String,
    username: String,
    email: String,
}

fn create_app(pool: PgPool) -> axum::Router {
    let state = AppState {
        pool: pool.clone(),
        jwt_secret: "test_secret".to_string(),
        google_client_id: "test_client_id".to_string(),
        google_client_secret: "test_client_secret".to_string(),
        google_auth_url: "http://localhost:8080".to_string(),
        active_rooms: Arc::new(RwLock::new(HashMap::new())),
        music_provider: Arc::new(backend::services::music_provider::hifi::HifiProvider::new()),
    };
    app_router(state.clone()).with_state(state)
}

async fn register_and_login(server: &TestServer, username: &str, email: &str) -> String {
    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": username,
            "email": email,
            "password": "password123"
        }))
        .await;

    let json = res.json::<TestAuthResponse>();
    json.access_token
}

async fn get_me(server: &TestServer, token: &str) -> TestUserResponse {
    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    res.json::<TestUserResponse>()
}

#[sqlx::test]
async fn test_create(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_create", "create@example.com").await;

    let res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::CREATED);
    let room = res.json::<TestRoomResponse>();
    assert!(!room.id.is_empty());
    assert!(!room.owner_id.is_empty());
}

#[sqlx::test]
async fn test_get_room(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_get", "get@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .get(&format!("/rooms/{}", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let fetched_room = res.json::<TestRoomResponse>();
    assert_eq!(fetched_room.id, room.id);
    assert_eq!(fetched_room.owner_id, room.owner_id);
}

#[sqlx::test]
async fn test_delete(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_del", "del@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .delete(&format!("/rooms/{}", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);

    // Verify it's deleted
    let get_res = server
        .get(&format!("/rooms/{}", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    get_res.assert_status(StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_enable_license_room(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_pub", "pub@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .post(&format!("/rooms/{}/enable-license", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_disable_license_room(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_priv", "priv@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .post(&format!("/rooms/{}/disable-license", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_delegate_room_device_and_revoke(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_delegate", "delegate@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let delegate_res = server
        .post(&format!("/rooms/{}/delegate", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "device_name": "Pixel 8" }))
        .await;

    delegate_res.assert_status(StatusCode::NO_CONTENT);

    let revoke_res = server
        .delete(&format!("/rooms/{}/delegate", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    revoke_res.assert_status(StatusCode::NO_CONTENT);
}
