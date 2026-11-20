//! The links of the emails open pages served by the backend, which work in any
//! browser without a session.

mod common;

use axum::http::{StatusCode, header::AUTHORIZATION};
use axum_test::TestServer;
use common::{PUBLIC_URL, TestApp, TestUser, bearer};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn email_confirmed(server: &TestServer, user: &TestUser) -> bool {
    server
        .get("/users/me")
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .await
        .json::<Value>()["email_confirmed"]
        .as_bool()
        .unwrap()
}

/// The link of the most recent email sent to `receiver`.
fn last_link(app: &TestApp, receiver: &str) -> String {
    let (_, email) = app
        .mailer
        .sent()
        .into_iter()
        .rev()
        .find(|(to, _)| to == receiver)
        .expect("an email was sent");
    email
        .body
        .split_whitespace()
        .find(|word| word.starts_with("http"))
        .expect("the email carries a link")
        .to_string()
}

#[sqlx::test]
async fn test_confirmation_link_opens_a_page_that_confirms_on_submit(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "link").await;

    server
        .post("/users/me/send-confirmation-email")
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let token = app.last_email_token(&user.email).unwrap();
    assert_eq!(
        last_link(&app, &user.email),
        format!("{PUBLIC_URL}/confirm-email?token={token}")
    );

    // Opening the link only shows the form: a mail scanner cannot spend the token.
    let page = server.get(&format!("/confirm-email?token={token}")).await;
    page.assert_status_ok();
    assert!(page.text().contains(&format!(r#"value="{token}""#)));
    assert_eq!(page.header("cache-control").to_str().unwrap(), "no-store");
    assert!(!email_confirmed(&server, &user).await);

    // Submitting it confirms the address, without any session.
    let done = server
        .post(&format!("/confirm-email?token={token}"))
        .form(&json!({ "token": token }))
        .await;
    done.assert_status_ok();
    assert!(done.text().contains("Email confirmed"));
    assert!(email_confirmed(&server, &user).await);

    // The token is single use.
    server
        .post("/confirm-email")
        .form(&json!({ "token": token }))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_confirm_email_api_needs_no_session(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "nosession").await;

    server
        .post("/users/me/send-confirmation-email")
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let token = app.last_email_token(&user.email).unwrap();

    server
        .patch("/auth/confirm-email")
        .json(&json!({ "token": token }))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert!(email_confirmed(&server, &user).await);
}

#[sqlx::test]
async fn test_reset_link_opens_a_form_that_changes_the_password(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.server();
    let user = common::register(&server, "resetpage").await;

    server
        .post("/auth/forgot-password")
        .json(&json!({ "email": user.email }))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let token = app.wait_for_email_token(&user.email).await;
    assert_eq!(
        last_link(&app, &user.email),
        format!("{PUBLIC_URL}/reset-password?token={token}")
    );

    let form = server.get(&format!("/reset-password?token={token}")).await;
    form.assert_status_ok();
    assert!(form.text().contains(r#"name="new_password""#));

    // Mistakes show the form again and keep the token valid.
    let mismatch = server
        .post("/reset-password")
        .form(&json!({ "token": token, "new_password": "brand-new-password", "confirmation": "other-password" }))
        .await;
    mismatch.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    assert!(mismatch.text().contains("The two passwords are different."));
    server
        .post("/reset-password")
        .form(&json!({ "token": token, "new_password": "short", "confirmation": "short" }))
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    let done = server
        .post("/reset-password")
        .form(&json!({ "token": token, "new_password": "brand-new-password", "confirmation": "brand-new-password" }))
        .await;
    done.assert_status_ok();
    assert!(done.text().contains("Password changed"));

    server
        .post("/auth/login")
        .json(&json!({ "email": user.email, "password": "brand-new-password" }))
        .await
        .assert_status_ok();
    server
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": user.refresh_token }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_broken_links_show_an_error_page_without_echoing_input(pool: PgPool) {
    let server = TestApp::new(pool).server();

    for path in ["/confirm-email", "/reset-password"] {
        server
            .get(path)
            .await
            .assert_status(StatusCode::BAD_REQUEST);

        let page = server
            .get(&format!("{path}?token=%3Cscript%3Ealert(1)%3C%2Fscript%3E"))
            .await;
        page.assert_status(StatusCode::BAD_REQUEST);
        assert!(!page.text().contains("<script>"));

        // A well-formed token that was never issued.
        let unknown = uuid::Uuid::new_v4();
        let form = match path {
            "/confirm-email" => json!({ "token": unknown }),
            _ => {
                json!({ "token": unknown, "new_password": "brand-new-password", "confirmation": "brand-new-password" })
            }
        };
        let page = server.post(path).form(&form).await;
        page.assert_status(StatusCode::BAD_REQUEST);
        assert!(page.text().contains("This link does not work"));
    }
}

#[sqlx::test]
async fn test_pages_offer_the_app_on_android_only(pool: PgPool) {
    let server = TestApp::new(pool).server();
    let token = uuid::Uuid::new_v4();
    let android = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/131.0 Mobile Safari/537.36";
    let desktop = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36";

    for action in ["confirm-email", "reset-password"] {
        let app_link = format!(r#"href="musicroom://{action}?token={token}""#);

        let page = server
            .get(&format!("/{action}?token={token}"))
            .add_header(axum::http::header::USER_AGENT, android)
            .await;
        page.assert_status_ok();
        assert!(page.text().contains(&app_link), "{action} on Android");

        let page = server
            .get(&format!("/{action}?token={token}"))
            .add_header(axum::http::header::USER_AGENT, desktop)
            .await;
        page.assert_status_ok();
        assert!(!page.text().contains("musicroom://"), "{action} on desktop");
    }
}
