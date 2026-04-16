use axum_test::TestServer;
use backend::{
    routes::app_router,
    state::AppState,
};
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
struct TestInvitationResponse {
    id: String,
    room_id: String,
    inviter_id: String,
    invitee_id: String,
    is_pending: bool,
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

fn generate_email() -> String {
    format!("test_{}@example.com", uuid::Uuid::new_v4())
}

async fn create_test_user(server: &TestServer) -> (String, String) {
    let email = generate_email();
    let body = json!({
        "username": format!("testuser_{}", uuid::Uuid::new_v4().simple().to_string()[..8].to_string()),
        "email": email,
        "password": "password123"
    });

    server.post("/auth/register").json(&body).await.assert_status(axum::http::StatusCode::CREATED);

    let login_body = json!({
        "email": email,
        "password": "password123"
    });

    let resp = server.post("/auth/login").json(&login_body).await;
    resp.assert_status(axum::http::StatusCode::OK);
    
    let auth_resp: TestAuthResponse = resp.json();
    
    let get_me = server.get("/users/me").add_header("Authorization", format!("Bearer {}", auth_resp.access_token)).await.json::<serde_json::Value>();
    let user_id = get_me["id"].as_str().unwrap().to_string();

    (auth_resp.access_token, user_id)
}

#[sqlx::test]
async fn test_invite_flow(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let (owner_token, _owner_id) = create_test_user(&server).await;
    let (friend_token, friend_id) = create_test_user(&server).await;

    // Create room
    let room_body = json!({
        "name": "Test Room"
    });
    let create_room_resp = server.post("/rooms")
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .json(&room_body)
        .await;
    create_room_resp.assert_status(axum::http::StatusCode::CREATED);
    let room: TestRoomResponse = create_room_resp.json();

    // Invite friend
    let invite_uri = format!("/rooms/{}/invite/{}", room.id, friend_id);
    let invite_resp = server.post(&invite_uri)
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .await;
    invite_resp.assert_status(axum::http::StatusCode::CREATED);
    let invitation: TestInvitationResponse = invite_resp.json();
    assert_eq!(invitation.is_pending, true);

    // List pending invitations as friend
    let list_resp = server.get("/me/invitations")
        .add_header("Authorization", format!("Bearer {}", friend_token))
        .await;
    list_resp.assert_status(axum::http::StatusCode::OK);
    let invitations: Vec<TestInvitationResponse> = list_resp.json();
    assert_eq!(invitations.len(), 1);
    assert_eq!(invitations[0].id, invitation.id);

    // Accept invitation
    let accept_uri = format!("/me/invitations/{}/accept", invitation.id);
    let accept_resp = server.post(&accept_uri)
        .add_header("Authorization", format!("Bearer {}", friend_token))
        .await;
    accept_resp.assert_status(axum::http::StatusCode::OK);
    let accepted: TestInvitationResponse = accept_resp.json();
    assert_eq!(accepted.is_pending, false);
}

#[sqlx::test]
async fn test_reject_invitation(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let (owner_token, _owner_id) = create_test_user(&server).await;
    let (friend_token, friend_id) = create_test_user(&server).await;

    // Create room
    let room_body = json!({
        "name": "Test Room Reject"
    });
    let create_room_resp = server.post("/rooms")
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .json(&room_body)
        .await;
    create_room_resp.assert_status(axum::http::StatusCode::CREATED);
    let room: TestRoomResponse = create_room_resp.json();

    // Invite friend
    let invite_uri = format!("/rooms/{}/invite/{}", room.id, friend_id);
    let invite_resp = server.post(&invite_uri)
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .await;
    invite_resp.assert_status(axum::http::StatusCode::CREATED);
    let invitation: TestInvitationResponse = invite_resp.json();

    // Reject invitation
    let reject_uri = format!("/me/invitations/{}/reject", invitation.id);
    let reject_resp = server.post(&reject_uri)
        .add_header("Authorization", format!("Bearer {}", friend_token))
        .await;
    reject_resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify it's deleted
    let list_resp = server.get("/me/invitations")
        .add_header("Authorization", format!("Bearer {}", friend_token))
        .await;
    list_resp.assert_status(axum::http::StatusCode::OK);
    let invitations: Vec<TestInvitationResponse> = list_resp.json();
    assert_eq!(invitations.len(), 0);
}

#[sqlx::test]
async fn test_revoke_invitation(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let (owner_token, _owner_id) = create_test_user(&server).await;
    let (_friend_token, friend_id) = create_test_user(&server).await;

    // Create room
    let room_body = json!({
        "name": "Test Room Revoke"
    });
    let create_room_resp = server.post("/rooms")
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .json(&room_body)
        .await;
    create_room_resp.assert_status(axum::http::StatusCode::CREATED);
    let room: TestRoomResponse = create_room_resp.json();

    // Invite friend
    let invite_uri = format!("/rooms/{}/invite/{}", room.id, friend_id);
    let invite_resp = server.post(&invite_uri)
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .await;
    invite_resp.assert_status(axum::http::StatusCode::CREATED);
    let invitation: TestInvitationResponse = invite_resp.json();

    // Revoke invitation
    let revoke_uri = format!("/invitations/{}/revoke", invitation.id);
    let revoke_resp = server.post(&revoke_uri)
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .await;
    revoke_resp.assert_status(axum::http::StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_unauthorized_actions(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let (owner_token, _owner_id) = create_test_user(&server).await;
    let (friend_token, friend_id) = create_test_user(&server).await;
    let (other_token, _other_id) = create_test_user(&server).await;

    // Create room
    let room_body = json!({
        "name": "Test Room Unauth"
    });
    let create_room_resp = server.post("/rooms")
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .json(&room_body)
        .await;
    create_room_resp.assert_status(axum::http::StatusCode::CREATED);
    let room: TestRoomResponse = create_room_resp.json();

    // Invite friend
    let invite_uri = format!("/rooms/{}/invite/{}", room.id, friend_id);
    let invite_resp = server.post(&invite_uri)
        .add_header("Authorization", format!("Bearer {}", owner_token))
        .await;
    invite_resp.assert_status(axum::http::StatusCode::CREATED);
    let invitation: TestInvitationResponse = invite_resp.json();

    // Non-owner cannot invite
    let invite_uri2 = format!("/rooms/{}/invite/{}", room.id, friend_id);
    server.post(&invite_uri2)
        .add_header("Authorization", format!("Bearer {}", friend_token))
        .await
        .assert_status(axum::http::StatusCode::FORBIDDEN);

    // Other user cannot accept
    let accept_uri = format!("/me/invitations/{}/accept", invitation.id);
    server.post(&accept_uri)
        .add_header("Authorization", format!("Bearer {}", other_token))
        .await
        .assert_status(axum::http::StatusCode::FORBIDDEN);

    // Other user cannot reject
    let reject_uri = format!("/me/invitations/{}/reject", invitation.id);
    server.post(&reject_uri)
        .add_header("Authorization", format!("Bearer {}", other_token))
        .await
        .assert_status(axum::http::StatusCode::FORBIDDEN);

    // Other user cannot revoke
    let revoke_uri = format!("/invitations/{}/revoke", invitation.id);
    server.post(&revoke_uri)
        .add_header("Authorization", format!("Bearer {}", other_token))
        .await
        .assert_status(axum::http::StatusCode::FORBIDDEN);
}
