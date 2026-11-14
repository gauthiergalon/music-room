mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::{FAILING_TRACK_ID, TestApp, UNKNOWN_TRACK_ID, create_app};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

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

/// The first track added to an idle room starts playing right away and leaves
/// the queue, so tests about the queue itself put one track in front first.
async fn start_playback(server: &TestServer, token: &str, room_id: &str) {
    server
        .post(&format!("/rooms/{}/queue", room_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({ "track_id": 999 }))
        .await
        .assert_status(StatusCode::NO_CONTENT);
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
    start_playback(&server, &token, &room.id).await;

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
    start_playback(&server, &token, &room.id).await;

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
    start_playback(&server, &token, &room.id).await;

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

#[sqlx::test]
async fn test_queue_add_caches_track_metadata_once(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let owner = common::register(&server, "cache").await;
    let room_id = common::create_room(&server, &owner).await;

    for _ in 0..3 {
        common::add_track(&server, &owner, room_id, 77).await;
    }

    // The provider is asked once; later adds use the stored metadata.
    assert_eq!(app.music.track_info_calls(), 1);
}

#[sqlx::test]
async fn test_queue_add_reports_provider_errors(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let owner = common::register(&server, "errors").await;
    let room_id = common::create_room(&server, &owner).await;
    let add = |track_id: i64| {
        server
            .post(&format!("/rooms/{room_id}/queue"))
            .add_header(
                axum::http::header::AUTHORIZATION,
                common::bearer(&owner.access_token),
            )
            .json(&json!({ "track_id": track_id }))
    };

    add(UNKNOWN_TRACK_ID)
        .await
        .assert_status(StatusCode::NOT_FOUND);
    add(FAILING_TRACK_ID)
        .await
        .assert_status(StatusCode::BAD_GATEWAY);

    // Nothing was queued for the failed requests.
    let queue = server
        .get(&format!("/rooms/{room_id}/queue"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&owner.access_token),
        )
        .await
        .json::<Vec<TestQueueResponse>>();
    assert!(queue.is_empty());
}

#[sqlx::test]
async fn test_queue_add_checks_permissions_before_calling_the_provider(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.server();
    let owner = common::register(&server, "owner").await;
    let stranger = common::register(&server, "stranger").await;
    let room_id = common::create_room(&server, &owner).await;
    sqlx::query("UPDATE rooms SET is_public = false WHERE id = $1")
        .bind(room_id)
        .execute(&pool)
        .await
        .unwrap();

    server
        .post(&format!("/rooms/{room_id}/queue"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&stranger.access_token),
        )
        .json(&json!({ "track_id": 5 }))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    assert_eq!(app.music.track_info_calls(), 0);
}

#[sqlx::test]
async fn test_queue_remove_and_reorder_report_missing_items_and_conflicts(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let owner = common::register(&server, "edit").await;
    let room_id = common::create_room(&server, &owner).await;
    let auth = common::bearer(&owner.access_token);
    // The first track starts playing; the next two stay in the queue.
    for track_id in [1, 2, 3] {
        common::add_track(&server, &owner, room_id, track_id).await;
    }
    let queue = server
        .get(&format!("/rooms/{room_id}/queue"))
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .await
        .json::<Vec<TestQueueResponse>>();
    assert_eq!(queue.len(), 2);

    let missing = uuid::Uuid::new_v4();
    server
        .delete(&format!("/rooms/{room_id}/queue"))
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "id": missing }))
        .await
        .assert_status(StatusCode::NOT_FOUND);
    server
        .patch(&format!("/rooms/{room_id}/queue"))
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "id": missing, "new_position": 10.0 }))
        .await
        .assert_status(StatusCode::NOT_FOUND);

    // Moving an item onto the exact position of another one is a conflict, not a crash.
    server
        .patch(&format!("/rooms/{room_id}/queue"))
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .json(&json!({ "id": queue[0].id, "new_position": queue[1].position }))
        .await
        .assert_status(StatusCode::CONFLICT);
}
