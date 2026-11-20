mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::{Duration, Utc};
use common::{TestApp, create_app};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

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

#[derive(Serialize, Deserialize, Debug)]
struct TestSubscriptionResponse {
    is_subscribed: bool,
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
async fn test_enable_subscription(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_enable_sub", "enable_sub@example.com").await;

    let res = server
        .post("/users/me/subscription")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::OK);

    let user = res.json::<TestSubscriptionResponse>();
    assert!(user.is_subscribed);

    let me_res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    me_res.assert_status(StatusCode::OK);
    let me = me_res.json::<TestSubscriptionResponse>();
    assert!(me.is_subscribed);
}

#[sqlx::test]
async fn test_subscription_deactivates_after_expiration(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_expire_sub", "expire_sub@example.com").await;

    let me_res = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    me_res.assert_status(StatusCode::OK);
    let me = me_res.json::<TestUserResponse>();
    let user_id = uuid::Uuid::parse_str(&me.id).unwrap();

    sqlx::query!(
        "UPDATE users SET is_subscribed = TRUE, end_subscription_date = $1 WHERE id = $2",
        Utc::now() - Duration::seconds(1),
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let refreshed = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    refreshed.assert_status(StatusCode::OK);
    let refreshed_user = refreshed.json::<TestSubscriptionResponse>();
    assert!(!refreshed_user.is_subscribed);
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
        .patch("/auth/confirm-email")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "token": "invalid_token123"
        }))
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
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(15),
    };

    backend::repositories::email_tokens::create(&pool, new_token)
        .await
        .unwrap();

    let res = server
        .patch("/auth/confirm-email")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&json!({
            "token": token_pair.plain
        }))
        .await;

    println!("Expected 204, got {:?}", res.status_code());
    println!("Body: {:?}", res.text());
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
    assert_eq!(
        user.favorite_genres,
        Some(vec!["Rock".to_string(), "Electro".to_string()])
    );
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
    assert_eq!(
        user.privacy_level,
        backend::models::user::PrivacyLevel::Private
    );
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
    server
        .patch("/users/me/favorite-genres")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_alice),
        )
        .json(&json!({ "favorite_genres": ["Jazz"] }))
        .await;

    let res = server
        .patch("/users/me/privacy")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_alice),
        )
        .json(&json!({ "privacy_level": "Private" }))
        .await;
    res.assert_status(StatusCode::OK);

    // Bob tries to get Alice's profile -> should be None
    let res = server
        .get(&format!("/users/{}", alice.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_bob),
        )
        .await;
    res.assert_status(StatusCode::OK);
    let public_alice_priv = res.json::<PrivacyTestUserResponse>();
    assert_eq!(
        public_alice_priv.favorite_genres, None,
        "Bob should not see Alice's genres"
    );

    // Alice sets to Public
    let res = server
        .patch("/users/me/privacy")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_alice),
        )
        .json(&json!({ "privacy_level": "Public" }))
        .await;
    res.assert_status(StatusCode::OK);

    // Bob tries to get Alice's profile -> should see Some(["Jazz"])
    let res = server
        .get(&format!("/users/{}", alice.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token_bob),
        )
        .await;
    let public_alice_pub = res.json::<PrivacyTestUserResponse>();
    assert_eq!(
        public_alice_pub.favorite_genres,
        Some(vec!["Jazz".to_string()]),
        "Bob should see Alice's genres now"
    );
}

#[sqlx::test]
async fn test_email_change_only_applies_after_confirmation(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "mailchange").await;
    let auth = common::bearer(&user.access_token);

    // Mixed case and spaces are normalised before anything else.
    let res = server
        .patch("/users/me/email")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "new_email": "  New.Address@Example.com " }))
        .await;
    res.assert_status(StatusCode::OK);

    // Nothing changed yet: the old address still identifies the account.
    assert_eq!(res.json::<serde_json::Value>()["email"], user.email);
    server
        .post("/auth/login")
        .json(&json!({ "email": user.email, "password": common::PASSWORD }))
        .await
        .assert_status(StatusCode::OK);

    // The link went to the new address, not the current one.
    let token = app
        .last_email_token("new.address@example.com")
        .expect("a confirmation email was sent to the new address");
    assert!(app.last_email_token(&user.email).is_none());

    server
        .patch("/auth/confirm-email")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "token": token }))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let me = server
        .get("/users/me")
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .await
        .json::<serde_json::Value>();
    assert_eq!(me["email"], "new.address@example.com");
    assert_eq!(me["email_confirmed"], true);

    server
        .post("/auth/login")
        .json(&json!({ "email": "new.address@example.com", "password": common::PASSWORD }))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn test_email_change_to_a_taken_address_is_refused(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "taker").await;
    let other = common::register(&server, "owner").await;

    server
        .patch("/users/me/email")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&user.access_token),
        )
        .json(&json!({ "new_email": other.email.to_uppercase() }))
        .await
        .assert_status(StatusCode::CONFLICT);

    assert!(app.mailer.sent().is_empty());
}

#[sqlx::test]
async fn test_send_confirmation_email_then_confirm(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "confirm").await;
    let auth = common::bearer(&user.access_token);

    server
        .post("/users/me/send-confirmation-email")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .await
        .assert_status(StatusCode::NO_CONTENT);

    // A second request while the first link is valid is refused.
    server
        .post("/users/me/send-confirmation-email")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .await
        .assert_status(StatusCode::TOO_MANY_REQUESTS);

    let token = app.last_email_token(&user.email).unwrap();
    server
        .patch("/auth/confirm-email")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "token": token }))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let me = server
        .get("/users/me")
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .await
        .json::<serde_json::Value>();
    assert_eq!(me["email"], user.email);
    assert_eq!(me["email_confirmed"], true);
}

#[sqlx::test]
async fn test_update_password_revokes_other_sessions_and_returns_a_new_one(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let user = common::register(&server, "pwd").await;

    let res = server
        .patch("/users/me/password")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&user.access_token),
        )
        .json(&json!({ "current_password": common::PASSWORD, "new_password": "newpassword456" }))
        .await;
    res.assert_status(StatusCode::OK);
    let new_session = res.json::<serde_json::Value>();

    // The session that existed before the change is revoked...
    server
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": user.refresh_token }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    // ...and the caller got a working replacement.
    server
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": new_session["refresh_token"] }))
        .await
        .assert_status(StatusCode::OK);

    server
        .post("/auth/login")
        .json(&json!({ "email": user.email, "password": "newpassword456" }))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn test_update_password_rejects_wrong_or_identical_password(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let user = common::register(&server, "pwdcheck").await;
    let auth = common::bearer(&user.access_token);

    server
        .patch("/users/me/password")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "current_password": "not-my-password", "new_password": "newpassword456" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    server
        .patch("/users/me/password")
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .json(&json!({ "current_password": common::PASSWORD, "new_password": common::PASSWORD }))
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_update_username_is_trimmed_and_validated(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let user = common::register(&server, "rename").await;
    let auth = common::bearer(&user.access_token);

    let res = server
        .patch("/users/me/username")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "username": "  renamed  " }))
        .await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.json::<serde_json::Value>()["username"], "renamed");

    server
        .patch("/users/me/username")
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .json(&json!({ "username": "    " }))
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_favorite_genres_are_cleaned_up_and_bounded(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let user = common::register(&server, "genres").await;
    let update = |genres: serde_json::Value| {
        server
            .patch("/users/me/favorite-genres")
            .add_header(
                axum::http::header::AUTHORIZATION,
                common::bearer(&user.access_token),
            )
            .json(&json!({ "favorite_genres": genres }))
    };

    // Spaces, empty entries and repetitions are removed.
    let res = update(json!(["  Rock ", "", "rock", "Jazz"])).await;
    res.assert_status(StatusCode::OK);
    assert_eq!(
        res.json::<serde_json::Value>()["favorite_genres"],
        json!(["Rock", "Jazz"])
    );

    // Too many entries, or an entry that is too long, is refused...
    let too_many: Vec<String> = (0..21).map(|i| format!("genre {i}")).collect();
    update(json!(too_many))
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    update(json!(["a".repeat(33)]))
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    // ...and leaves the profile as it was.
    let me = server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&user.access_token),
        )
        .await
        .json::<serde_json::Value>();
    assert_eq!(me["favorite_genres"], json!(["Rock", "Jazz"]));
}
