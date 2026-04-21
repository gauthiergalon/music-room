use axum::http::StatusCode;
use axum_test::TestServer;
use backend::dtos::friend::FriendResponseDto;
use backend::routes::app_router;
use backend::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct TestAuthResponse {
    access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TestUserResponse {
    id: Uuid,
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

async fn register_login_and_get_user(
    server: &TestServer,
    username: &str,
    email: &str,
) -> (String, Uuid) {
    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": username,
            "email": email,
            "password": "password123"
        }))
        .await;

    let auth_json: TestAuthResponse = res.json();
    let token = auth_json.access_token;

    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    let user_json: TestUserResponse = res.json();
    (token, user_json.id)
}

#[sqlx::test]
async fn test_send_friend_request(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, id_a) =
        register_login_and_get_user(&server, "alice_s", "alice_s@example.com").await;
    let (_token_b, _) = register_login_and_get_user(&server, "bob_s", "bob_s@example.com").await;

    let res = server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_s" }))
        .await;

    res.assert_status(StatusCode::CREATED);
    let friend_res: FriendResponseDto = res.json();
    assert_eq!(friend_res.sender_id, id_a);
    assert!(friend_res.is_pending);
}

#[sqlx::test]
async fn test_list_friend_requests(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, id_a) =
        register_login_and_get_user(&server, "alice_l", "alice_l@example.com").await;
    let (token_b, _) = register_login_and_get_user(&server, "bob_l", "bob_l@example.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_l" }))
        .await
        .assert_status(StatusCode::CREATED);

    let res_b = server
        .get("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_b),
        )
        .await;

    res_b.assert_status(StatusCode::OK);
    let friends: Vec<FriendResponseDto> = res_b.json();
    assert_eq!(friends.len(), 1);
    assert!(friends[0].is_pending);
    assert_eq!(friends[0].sender_id, id_a);
}

#[sqlx::test]
async fn test_accept_friend_request_success(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, id_a) =
        register_login_and_get_user(&server, "alice_acc", "alice_acc@test.com").await;
    let (token_b, _) = register_login_and_get_user(&server, "bob_acc", "bob_acc@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_acc" }))
        .await
        .assert_status(StatusCode::CREATED);

    let res = server
        .post(&format!("/friends/{id_a}/accept"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_b),
        )
        .await;
    res.assert_status(StatusCode::OK);

    let friend: FriendResponseDto = res.json();
    assert!(!friend.is_pending);
}

#[sqlx::test]
async fn test_cannot_accept_own_friend_request(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, _id_a) =
        register_login_and_get_user(&server, "alice_acc2", "alice_acc2@test.com").await;
    let (_token_b, id_b) =
        register_login_and_get_user(&server, "bob_acc2", "bob_acc2@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_acc2" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post(&format!("/friends/{id_b}/accept"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .await
        .assert_status(StatusCode::CONFLICT);
}

#[sqlx::test]
async fn test_reject_friend_request(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, id_a) =
        register_login_and_get_user(&server, "alice_rej", "alice_rej@test.com").await;
    let (token_b, _) = register_login_and_get_user(&server, "bob_rej", "bob_rej@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_rej" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .delete(&format!("/friends/{id_a}/reject"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_b),
        )
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let list_res = server
        .get("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .await;
    let friends: Vec<FriendResponseDto> = list_res.json();
    assert_eq!(friends.len(), 0);
}

#[sqlx::test]
async fn test_remove_friend(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, id_a) =
        register_login_and_get_user(&server, "alice_rm", "alice_rm@test.com").await;
    let (token_b, id_b) = register_login_and_get_user(&server, "bob_rm", "bob_rm@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_rm" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post(&format!("/friends/{id_a}/accept"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_b),
        )
        .await
        .assert_status(StatusCode::OK);

    server
        .delete(&format!("/friends/{id_b}"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .await
        .assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_cannot_send_request_to_self(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, _) =
        register_login_and_get_user(&server, "alice_e1", "alice_edge1@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "alice_e1" }))
        .await
        .assert_status(StatusCode::CONFLICT);
}

#[sqlx::test]
async fn test_cannot_send_duplicate_request(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, _id_a) =
        register_login_and_get_user(&server, "alice_e2", "alice_edge2@test.com").await;
    let (_token_b, _) = register_login_and_get_user(&server, "bob_e2", "bob_edge2@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_e2" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_e2" }))
        .await
        .assert_status(StatusCode::CONFLICT);
}

#[sqlx::test]
async fn test_mutual_request_auto_accepts(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);

    let (token_a, _) =
        register_login_and_get_user(&server, "alice_e3", "alice_edge3@test.com").await;
    let (token_b, _) = register_login_and_get_user(&server, "bob_e3", "bob_edge3@test.com").await;

    server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_a),
        )
        .json(&json!({ "username": "bob_e3" }))
        .await
        .assert_status(StatusCode::CREATED);

    let auto_accept_res = server
        .post("/friends")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_b),
        )
        .json(&json!({ "username": "alice_e3" }))
        .await;

    auto_accept_res.assert_status(StatusCode::CREATED);
    let friend: FriendResponseDto = auto_accept_res.json();
    assert!(!friend.is_pending);
}
