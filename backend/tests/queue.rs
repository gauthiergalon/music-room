use axum::http::StatusCode;
use axum_test::TestServer;
use backend::routes::app_router;
use backend::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
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
struct TestQueueResponse {
    id: String,
    room_id: String,
    track_id: i64,
    position: f64,
}

fn create_app(pool: PgPool) -> axum::Router {
    let state = AppState {
        pool: pool.clone(),
        jwt_secret: "test_secret".to_string(),
        google_client_id: "test_client_id".to_string(),
        google_client_secret: "test_client_secret".to_string(),
        google_auth_url: "http://localhost:8080".to_string(),
        active_rooms: Arc::new(RwLock::new(HashMap::new())),
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

#[sqlx::test]
async fn test_queue_add(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_q_add", "q_add@example.com").await;

    // Create room
    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    // Add track to queue
    let res = server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 12345 }))
        .await;
    res.assert_status(StatusCode::NO_CONTENT);

    // Direct DB check or verify it adds successfully
}

#[sqlx::test]
async fn test_queue_get(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_q_get", "q_get@example.com").await;

    // Create room
    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    // Add tracks to queue
    server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 111 }))
        .await;
    server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 222 }))
        .await;

    // Get queue
    let get_res = server
        .get(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    get_res.assert_status(StatusCode::OK);
    let q = get_res.json::<Vec<TestQueueResponse>>();
    assert_eq!(q.len(), 2);
    assert_eq!(q[0].track_id, 111);
    assert_eq!(q[1].track_id, 222);
}

#[sqlx::test]
async fn test_queue_remove(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_q_del", "q_del@example.com").await;

    // Create room
    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    // Add track to queue
    server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 999 }))
        .await;

    // Retrieve to get the ID
    let get_res = server
        .get(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let q = get_res.json::<Vec<TestQueueResponse>>();
    let track_unique_id = &q[0].id;

    // Remove track
    let del_res = server
        .delete(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "id": track_unique_id }))
        .await;

    del_res.assert_status(StatusCode::NO_CONTENT);

    // Verify empty queue
    let get_res2 = server
        .get(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let q2 = get_res2.json::<Vec<TestQueueResponse>>();
    assert_eq!(q2.len(), 0);
}

#[sqlx::test]
async fn test_queue_reorder(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_queue_reo", "q_reo@example.com").await;

    // Create room
    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    // Add 3 tracks to queue
    server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 111 }))
        .await;
    server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 222 }))
        .await;
    server
        .post(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 333 }))
        .await;

    // Get queue
    let get_res = server
        .get(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let q = get_res.json::<Vec<TestQueueResponse>>();
    assert_eq!(q.len(), 3);

    // Default positions should be 0, 1, 2
    assert_eq!(q[0].track_id, 111);
    assert_eq!(q[1].track_id, 222);
    assert_eq!(q[2].track_id, 333);

    // Move the third track (333) between 111 (pos 0) and 222 (pos 1), so new pos = 0.5
    let move_res = server
        .patch(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "id": q[2].id,
            "new_position": 0.5
        }))
        .await;
    move_res.assert_status(StatusCode::NO_CONTENT);

    // Get queue again and check reorder
    let get_res2 = server
        .get(&format!("/rooms/{}/queue", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let reordered_q = get_res2.json::<Vec<TestQueueResponse>>();
    assert_eq!(reordered_q.len(), 3);

    // ASC sorting should yield: 111 (0), 333 (0.5), 222 (1)
    assert_eq!(reordered_q[0].track_id, 111, "First track wrong");
    assert_eq!(reordered_q[1].track_id, 333, "Second track wrong");
    assert_eq!(reordered_q[2].track_id, 222, "Third track wrong");
    assert_eq!(
        reordered_q[1].position, 0.5,
        "Position not updated correctly"
    );
}
