use axum::http::StatusCode;
use axum_test::TestServer;
use backend::routes::app_router;
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	assert_eq!(error_res.details.unwrap()[0], "Username has invalid length (must be between 3 and 32 characters)".to_string());
}

#[sqlx::test]
async fn test_username_too_long(pool: PgPool) {
	let app = app_router().with_state(pool);
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
	assert_eq!(error_res.details.unwrap()[0], "Username has invalid length (must be between 3 and 32 characters)");
}

#[sqlx::test]
async fn test_refresh_token_success(pool: PgPool) {
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
		.add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", json.access_token).parse::<axum::http::HeaderValue>().unwrap())
		.json(&json!({
			"refresh_token": json.refresh_token
		}))
		.await;

	res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_logout_no_auth_token(pool: PgPool) {
	let app = app_router().with_state(pool);
	let server = TestServer::new(app);

	let res = server
		.post("/auth/logout")
		.json(&json!({
			"refresh_token": "somerefresh"
		}))
		.await;

	res.assert_status(StatusCode::UNAUTHORIZED);
	let error_res = res.json::<ErrorResponse>();
	assert_eq!(error_res.error, "Unauthorized");
	assert_eq!(error_res.details.unwrap()[0], "Invalid token");
}

#[sqlx::test]
async fn test_forgot_password_success(pool: PgPool) {
	let app = app_router().with_state(pool);
	let server = TestServer::new(app);

	let res = server
		.post("/auth/forgot-password")
		.json(&json!({
			"email": "test@example.com"
		}))
		.await;

	res.assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn test_forgot_password_invalid_email(pool: PgPool) {
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	let app = app_router().with_state(pool);
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
	assert_eq!(error_res.details.unwrap()[0], "Password does not meet the required policy (must be at least 8 characters)");
}

#[sqlx::test]
async fn test_login_missing_fields(pool: PgPool) {
	let app = app_router().with_state(pool);
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
async fn test_logout_invalid_auth_token(pool: PgPool) {
	let app = app_router().with_state(pool);
	let server = TestServer::new(app);

	let res = server
		.post("/auth/logout")
		.add_header(axum::http::header::AUTHORIZATION, "Bearer invalid-token".parse::<axum::http::HeaderValue>().unwrap())
		.json(&json!({
			"refresh_token": "somerefresh"
		}))
		.await;

	res.assert_status(StatusCode::UNAUTHORIZED);
	let error_res = res.json::<ErrorResponse>();
	assert_eq!(error_res.error, "Unauthorized");
	assert_eq!(error_res.details.unwrap()[0], "Invalid token");
}
