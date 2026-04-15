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
async fn test_get_me(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_get_me", "getme@example.com").await;

    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let user = res.json::<TestUserResponse>();
    assert_eq!(user.username, "test_get_me");
    assert_eq!(user.email, "getme@example.com");
}

#[sqlx::test]
async fn test_update_username(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_update_u", "updateu@example.com").await;

    let res = server
        .patch("/users/me/username")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "username": "new_username"
        }))
        .await;

    res.assert_status(StatusCode::OK);

    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    let user = res.json::<TestUserResponse>();
    assert_eq!(user.username, "new_username");
}

#[sqlx::test]
async fn test_update_email(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_update_e", "updatee@example.com").await;

    let res = server
        .patch("/users/me/email")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "new_email": "newemail@example.com"
        }))
        .await;

    res.assert_status(StatusCode::OK);

    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    let user = res.json::<TestUserResponse>();
    assert_eq!(user.email, "newemail@example.com");
}

#[sqlx::test]
async fn test_update_password(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_update_p", "updatepwd@example.com").await;

    let res = server
        .patch("/users/me/password")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "current_password": "password123",
            "new_password": "newpassword456"
        }))
        .await;

    res.assert_status(StatusCode::NO_CONTENT);

    // Try to login with new password
    let res = server
        .post("/auth/login")
        .json(&json!({
            "email": "updatepwd@example.com",
            "password": "newpassword456"
        }))
        .await;

    res.assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn test_get_user_by_id(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_get_u", "getuid@example.com").await;

    // First get me to get the user ID
    let me_res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    let me = me_res.json::<TestUserResponse>();

    // Then get public info by ID
    let res = server
        .get(&format!("/users/{}", me.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::OK);
    #[derive(Deserialize)]
    struct PublicUserResponse {
        username: String,
    }
    let user = res.json::<PublicUserResponse>();
    assert_eq!(user.username, "test_get_u");
}

#[sqlx::test]
async fn test_confirm_email_invalid_token(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_conf_invalid", "conf_invalid@example.com").await;

    let res = server
        .patch("/users/me/confirm-email?token=invalid_token123")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_confirm_email_success(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_conf_success", "conf_success@example.com").await;

    let me_res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let me = me_res.json::<TestUserResponse>();
    let user_id = uuid::Uuid::parse_str(&me.id).unwrap();

    let token_pair = backend::services::tokens::TokenPair::generate();

    let new_token = backend::models::email_token::NewEmailToken {
        token_hash: token_pair.hash.clone(),
        user_id,
        new_email: "new_confirmed@example.com".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
    };

    backend::repositories::email_tokens::create(&pool, new_token)
        .await
        .unwrap();

    let res = server
        .patch(&format!(
            "/users/me/confirm-email?token={}",
            token_pair.plain
        ))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);

    let me_res2 = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let me2 = me_res2.json::<TestUserResponse>();
    assert_eq!(me2.email, "new_confirmed@example.com");
}

#[derive(Serialize, Deserialize, Debug)]
struct PrivacyTestUserResponse {
    id: String,
    username: String,
    favorite_genres: Option<Vec<String>>,
    privacy_level: backend::models::user::PrivacyLevel,
}

#[sqlx::test]
async fn test_update_favorite_genres(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_upd_genres", "up_ge@test.com").await;

    let res = server
        .patch("/users/me/favorite-genres")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "favorite_genres": ["Rock", "Electro"]
        }))
        .await;

    res.assert_status(StatusCode::OK);

    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    let user = res.json::<PrivacyTestUserResponse>();
    assert_eq!(user.favorite_genres, Some(vec!["Rock".to_string(), "Electro".to_string()]));
}

#[sqlx::test]
async fn test_update_privacy_level(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_up_priv", "up_priv@test.com").await;

    let res = server
        .patch("/users/me/privacy")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "privacy_level": "Private"
        }))
        .await;

    res.assert_status(StatusCode::OK);

    let res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    let user = res.json::<PrivacyTestUserResponse>();
    assert_eq!(user.privacy_level, backend::models::user::PrivacyLevel::Private);
}

#[sqlx::test]
async fn test_get_user_privacy_control(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    
    // Alice
    let token_alice = register_and_login(&server, "alice_priv", "alice_p@test.com").await;
    let me_res_alice = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_alice),
        )
        .await;
    let alice = me_res_alice.json::<PrivacyTestUserResponse>();
    
    // Bob
    let token_bob = register_and_login(&server, "bob_priv", "bob_p@test.com").await;

    // Alice sets genres and privacy level to Private
    server.patch("/users/me/favorite-genres")
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token_alice))
        .json(&json!({ "favorite_genres": ["Jazz"] }))
        .await;
    
    let res = server.patch("/users/me/privacy")
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token_alice))
        .json(&json!({ "privacy_level": "Private" }))
        .await;
    res.assert_status(StatusCode::OK);

    // Bob tries to get Alice's profile -> should be None
    let res = server.get(&format!("/users/{}", alice.id))
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token_bob))
        .await;
    res.assert_status(StatusCode::OK);
    let public_alice_priv = res.json::<PrivacyTestUserResponse>();
    assert_eq!(public_alice_priv.favorite_genres, None, "Bob should not see Alice's genres");

    // Alice sets to Public
    let res = server.patch("/users/me/privacy")
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token_alice))
        .json(&json!({ "privacy_level": "Public" }))
        .await;
    res.assert_status(StatusCode::OK);

    // Bob tries to get Alice's profile -> should see Some(["Jazz"])
    let res = server.get(&format!("/users/{}", alice.id))
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token_bob))
        .await;
    let public_alice_pub = res.json::<PrivacyTestUserResponse>();
    assert_eq!(public_alice_pub.favorite_genres, Some(vec!["Jazz".to_string()]), "Bob should see Alice's genres now");
}
