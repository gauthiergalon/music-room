mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::{TestApp, TestConfig, create_app, google};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

#[derive(Serialize, Deserialize, Debug)]
struct TestAuthResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: String,
    details: Option<Vec<String>>,
}

#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    res.assert_status(StatusCode::CREATED);

    let json = res.json::<TestAuthResponse>();
    assert!(!json.access_token.is_empty());
    assert!(!json.refresh_token.is_empty());
}

#[sqlx::test]
async fn test_register_duplicate_email(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser1",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser2",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    res.assert_status(StatusCode::CONFLICT);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Conflict");
    assert_eq!(error_res.details.unwrap()[0], "Email already in use");
}

#[sqlx::test]
async fn test_register_duplicate_username(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test1@example.com",
            "password": "password123"
        }))
        .await;

    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test2@example.com",
            "password": "password123"
        }))
        .await;

    res.assert_status(StatusCode::CONFLICT);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Conflict");
    assert_eq!(error_res.details.unwrap()[0], "Username already taken");
}

#[sqlx::test]
async fn test_login_success(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    let res = server
        .post("/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    res.assert_status(StatusCode::OK);
    let json = res.json::<TestAuthResponse>();
    assert!(!json.access_token.is_empty());
    assert!(!json.refresh_token.is_empty());
}

#[sqlx::test]
async fn test_login_invalid_password(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    let res = server
        .post("/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "wrongpassword"
        }))
        .await;

    res.assert_status(StatusCode::UNAUTHORIZED);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Unauthorized");
    assert_eq!(error_res.details.unwrap()[0], "Invalid email or password");
}

#[sqlx::test]
async fn test_login_nonexistent_user(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/login")
        .json(&json!({
            "email": "ghost@example.com",
            "password": "password123"
        }))
        .await;

    res.assert_status(StatusCode::UNAUTHORIZED);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Unauthorized");
    assert_eq!(error_res.details.unwrap()[0], "Invalid email or password");
}

#[sqlx::test]
async fn test_validation_errors(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": "ab",
            "email": "not-an-email",
            "password": "short"
        }))
        .await;

    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Validation Error");
    assert_eq!(
        error_res.details.unwrap()[0],
        "Username has invalid length (must be between 3 and 24 characters)".to_string()
    );
}

#[sqlx::test]
async fn test_username_too_long(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": "this_username_is_way_too_long_and_should_fail",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Validation Error");
    assert_eq!(
        error_res.details.unwrap()[0],
        "Username has invalid length (must be between 3 and 24 characters)"
    );
}

#[sqlx::test]
async fn test_refresh_token_success(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let login_res = server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    let json = login_res.json::<TestAuthResponse>();
    let refresh_token = json.refresh_token;

    let res = server
        .post("/auth/refresh")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;

    res.assert_status(StatusCode::OK);
    let new_json = res.json::<TestAuthResponse>();
    assert!(!new_json.access_token.is_empty());
    assert!(!new_json.refresh_token.is_empty());
    assert_ne!(refresh_token, new_json.refresh_token);
}

#[sqlx::test]
async fn test_refresh_token_invalid(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/refresh")
        .json(&json!({
            "refresh_token": "invalid_or_nonexistent_token"
        }))
        .await;

    res.assert_status(StatusCode::UNAUTHORIZED);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Unauthorized");
    assert_eq!(error_res.details.unwrap()[0], "Invalid token");
}

#[sqlx::test]
async fn test_logout_success(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let login_res = server
        .post("/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;
    let json = login_res.json::<TestAuthResponse>();

    let res = server
        .post("/auth/logout")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", json.access_token)
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .json(&json!({
            "refresh_token": json.refresh_token
        }))
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_forgot_password_success(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/forgot-password")
        .json(&json!({
            "email": "test@example.com"
        }))
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_forgot_password_invalid_email(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/forgot-password")
        .json(&json!({
            "email": "invalid-email"
        }))
        .await;

    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Validation Error");
    assert_eq!(error_res.details.unwrap()[0], "Invalid email address");
}

#[sqlx::test]
async fn test_reset_password_invalid_token(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/reset-password")
        .json(&json!({
            "token": "invalid_token",
            "new_password": "newpassword123"
        }))
        .await;

    res.assert_status(StatusCode::UNAUTHORIZED);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Unauthorized");
    assert_eq!(error_res.details.unwrap()[0], "Invalid token");
}

#[sqlx::test]
async fn test_reset_password_weak_password(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/reset-password")
        .json(&json!({
            "token": "sometoken",
            "new_password": "short"
        }))
        .await;

    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let error_res = res.json::<ErrorResponse>();
    assert_eq!(error_res.error, "Validation Error");
    assert_eq!(
        error_res.details.unwrap()[0],
        "Password does not meet the required policy (must be at least 8 and at most 128 characters)"
    );
}

#[sqlx::test]
async fn test_login_missing_fields(pool: PgPool) {
    let app = create_app(pool.clone());
    let server = TestServer::new(app);

    let res = server
        .post("/auth/login")
        .json(&json!({
            "email": "test@example.com"
        }))
        .await;

    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_logout_works_without_an_access_token(pool: PgPool) {
    // A client whose access token expired must still be able to end its session.
    let server = TestServer::new(create_app(pool));
    let user = common::register(&server, "logout").await;

    server
        .post("/auth/logout")
        .json(&json!({ "refresh_token": user.refresh_token }))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    server
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": user.refresh_token }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_register_trims_and_rejects_blank_usernames(pool: PgPool) {
    let server = TestServer::new(create_app(pool));

    let res = server
        .post("/auth/register")
        .json(&json!({ "username": "      ", "email": "blank@example.com", "password": "password123" }))
        .await;
    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    let res = server
        .post("/auth/register")
        .json(&json!({ "username": "  padded  ", "email": "padded@example.com", "password": "password123" }))
        .await;
    res.assert_status(StatusCode::CREATED);
    let token = res.json::<TestAuthResponse>().access_token;

    let me = server
        .get("/users/me")
        .add_header(axum::http::header::AUTHORIZATION, common::bearer(&token))
        .await
        .json::<serde_json::Value>();
    assert_eq!(me["username"], "padded");
}

#[sqlx::test]
async fn test_login_is_case_insensitive_on_email(pool: PgPool) {
    let server = TestServer::new(create_app(pool));

    server
        .post("/auth/register")
        .json(&json!({ "username": "casey", "email": "Casey@Example.com", "password": "password123" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post("/auth/login")
        .json(&json!({ "email": "  CASEY@example.COM ", "password": "password123" }))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn test_password_reset_flow_revokes_existing_sessions(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "reset").await;

    server
        .post("/auth/forgot-password")
        .json(&json!({ "email": user.email }))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let reset_token = app.wait_for_email_token(&user.email).await;

    server
        .post("/auth/reset-password")
        .json(&json!({ "token": reset_token, "new_password": "brand-new-password" }))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    // The session opened before the reset is gone...
    server
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": user.refresh_token }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    // ...the old password no longer works and the new one does.
    server
        .post("/auth/login")
        .json(&json!({ "email": user.email, "password": common::PASSWORD }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    server
        .post("/auth/login")
        .json(&json!({ "email": user.email, "password": "brand-new-password" }))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn test_forgot_password_sends_one_email_at_a_time(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "forgot").await;

    // Two requests arriving together, as an impatient user would send them.
    let request = || {
        server
            .post("/auth/forgot-password")
            .json(&json!({ "email": user.email }))
    };
    let (first, second) = tokio::join!(request(), request());
    first.assert_status(StatusCode::NO_CONTENT);
    second.assert_status(StatusCode::NO_CONTENT);

    app.wait_for_email_token(&user.email).await;
    // Leave the second request the time to send an email if it was going to.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(app.mailer.sent().len(), 1);
}

#[sqlx::test]
async fn test_forgot_password_answers_before_doing_the_work(pool: PgPool) {
    // The response must not depend on whether the account exists: not its
    // status, and not its timing either. Sending an email takes time, so the
    // request answers first and the email goes out afterwards.
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "timing").await;
    app.mailer.set_delay(std::time::Duration::from_millis(400));

    server
        .post("/auth/forgot-password")
        .json(&json!({ "email": user.email }))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert!(
        app.mailer.sent().is_empty(),
        "the request waited for the email to be sent"
    );

    server
        .post("/auth/forgot-password")
        .json(&json!({ "email": "nobody@example.com" }))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    // The email of the existing account still arrives; the unknown one gets nothing.
    app.wait_for_email_token(&user.email).await;
    assert_eq!(app.mailer.sent().len(), 1);
}

#[sqlx::test]
async fn test_reset_password_rejects_too_long_password(pool: PgPool) {
    let server = TestServer::new(create_app(pool));

    server
        .post("/auth/reset-password")
        .json(&json!({ "token": "whatever", "new_password": "a".repeat(129) }))
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_auth_routes_are_rate_limited(pool: PgPool) {
    let app = TestApp::with_config(
        pool,
        TestConfig {
            auth_rate_limit: Some(3),
            ..TestConfig::default()
        },
    );
    let server = app.server();
    let attempt = || {
        server
            .post("/auth/login")
            .add_header("x-forwarded-for", "203.0.113.7")
            .json(&json!({ "email": "nobody@example.com", "password": "wrong-password" }))
    };

    for _ in 0..3 {
        attempt().await.assert_status(StatusCode::UNAUTHORIZED);
    }
    attempt().await.assert_status(StatusCode::TOO_MANY_REQUESTS);

    // Another client is not affected.
    server
        .post("/auth/login")
        .add_header("x-forwarded-for", "203.0.113.8")
        .json(&json!({ "email": "nobody@example.com", "password": "wrong-password" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

// ------------------------------------------------------------------ Google

async fn google_app(pool: PgPool) -> (TestApp, wiremock::MockServer) {
    let (jwks_server, google_jwks_url) = google::mock_jwks().await;
    let app = TestApp::with_config(
        pool,
        TestConfig {
            google_jwks_url,
            ..TestConfig::default()
        },
    );
    (app, jwks_server)
}

async fn me(server: &TestServer, access_token: &str) -> serde_json::Value {
    server
        .get("/users/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(access_token),
        )
        .await
        .json::<serde_json::Value>()
}

#[sqlx::test]
async fn test_google_login_creates_a_confirmed_public_account(pool: PgPool) {
    let (app, _jwks) = google_app(pool).await;
    let server = app.server();

    let id_token = google::sign(&google::claims("google-1", "Test.Google@example.com"));
    let res = server
        .post("/auth/google-login")
        .json(&json!({ "id_token": id_token }))
        .await;
    res.assert_status(StatusCode::OK);

    let user = me(&server, &res.json::<TestAuthResponse>().access_token).await;
    assert_eq!(user["email"], "test.google@example.com");
    assert_eq!(user["email_confirmed"], true);
    assert_eq!(user["privacy_level"], "Public");
    assert_eq!(user["google_linked"], true);
    assert_eq!(user["has_password"], false);
}

#[sqlx::test]
async fn test_google_login_rejects_forged_tokens(pool: PgPool) {
    let (app, _jwks) = google_app(pool).await;
    let server = app.server();

    // Not a JWT at all.
    server
        .post("/auth/google-login")
        .json(&json!({ "id_token": "fake_valid_google_token" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    // Properly signed, but issued for another application.
    let mut claims = google::claims("google-2", "other@example.com");
    claims["aud"] = json!("another_client_id");
    server
        .post("/auth/google-login")
        .json(&json!({ "id_token": google::sign(&claims) }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_google_login_requires_a_verified_google_email(pool: PgPool) {
    let (app, _jwks) = google_app(pool).await;
    let server = app.server();

    let mut claims = google::claims("google-3", "unverified@example.com");
    claims["email_verified"] = json!(false);

    server
        .post("/auth/google-login")
        .json(&json!({ "id_token": google::sign(&claims) }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_google_login_links_an_account_whose_email_is_confirmed(pool: PgPool) {
    let (app, _jwks) = google_app(pool.clone()).await;
    let server = app.server();
    let user = common::register(&server, "existing").await;
    sqlx::query("UPDATE users SET email_confirmed = TRUE WHERE id = $1")
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();

    let id_token = google::sign(&google::claims("google-4", &user.email));
    let res = server
        .post("/auth/google-login")
        .json(&json!({ "id_token": id_token }))
        .await;
    res.assert_status(StatusCode::OK);

    let linked = me(&server, &res.json::<TestAuthResponse>().access_token).await;
    assert_eq!(linked["id"], user.id.to_string());
    assert_eq!(linked["google_linked"], true);
}

#[sqlx::test]
async fn test_google_login_does_not_take_over_an_unconfirmed_account(pool: PgPool) {
    // Someone registered with this address without proving they own it. The
    // real owner logging in with Google must not land in that account.
    let (app, _jwks) = google_app(pool).await;
    let server = app.server();
    let squatter = common::register(&server, "squatter").await;

    let id_token = google::sign(&google::claims("google-5", &squatter.email));
    server
        .post("/auth/google-login")
        .json(&json!({ "id_token": id_token }))
        .await
        .assert_status(StatusCode::CONFLICT);

    let unchanged = me(&server, &squatter.access_token).await;
    assert_eq!(unchanged["google_linked"], false);
}

#[sqlx::test]
async fn test_google_login_finds_the_account_by_google_id_after_an_email_change(pool: PgPool) {
    let (app, _jwks) = google_app(pool.clone()).await;
    let server = app.server();

    let first = server
        .post("/auth/google-login")
        .json(
            &json!({ "id_token": google::sign(&google::claims("google-6", "before@example.com")) }),
        )
        .await;
    first.assert_status(StatusCode::OK);
    let account = me(&server, &first.json::<TestAuthResponse>().access_token).await;

    sqlx::query("UPDATE users SET email = 'after@example.com' WHERE google_id = 'google-6'")
        .execute(&pool)
        .await
        .unwrap();

    // Google still reports the old address; the account is the same one.
    let second = server
        .post("/auth/google-login")
        .json(
            &json!({ "id_token": google::sign(&google::claims("google-6", "before@example.com")) }),
        )
        .await;
    second.assert_status(StatusCode::OK);
    let same = me(&server, &second.json::<TestAuthResponse>().access_token).await;
    assert_eq!(same["id"], account["id"]);
}

#[sqlx::test]
async fn test_google_login_with_a_very_long_name(pool: PgPool) {
    let (app, _jwks) = google_app(pool).await;
    let server = app.server();

    let mut claims = google::claims("google-7", "longname@example.com");
    claims["given_name"] = json!("Maximilian-Alexander von Hohenzollern");

    let res = server
        .post("/auth/google-login")
        .json(&json!({ "id_token": google::sign(&claims) }))
        .await;
    res.assert_status(StatusCode::OK);

    let user = me(&server, &res.json::<TestAuthResponse>().access_token).await;
    assert!(user["username"].as_str().unwrap().chars().count() <= 24);
}

#[sqlx::test]
async fn test_link_google_to_the_current_account(pool: PgPool) {
    let (app, _jwks) = google_app(pool).await;
    let server = app.server();
    let user = common::register(&server, "linker").await;
    let other = common::register(&server, "other").await;

    // The Google address does not have to match the account's address.
    let id_token = google::sign(&google::claims("google-8", "someone.else@example.com"));
    let res = server
        .post("/users/me/google")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&user.access_token),
        )
        .json(&json!({ "id_token": id_token }))
        .await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.json::<serde_json::Value>()["google_linked"], true);

    // The same Google account cannot be linked to a second user.
    server
        .post("/users/me/google")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&other.access_token),
        )
        .json(&json!({ "id_token": id_token }))
        .await
        .assert_status(StatusCode::CONFLICT);

    // Logging in with Google now opens the linked account.
    let login = server
        .post("/auth/google-login")
        .json(&json!({ "id_token": id_token }))
        .await;
    login.assert_status(StatusCode::OK);
    let logged_in = me(&server, &login.json::<TestAuthResponse>().access_token).await;
    assert_eq!(logged_in["id"], user.id.to_string());
}
