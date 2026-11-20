//! The pages the links of the emails open. They work in any browser, without
//! the app and without a session: the token of the link is the proof.
//!
//! Opening a link only shows a form. The single-use token is spent when the
//! form is submitted, so mail scanners that open links to check them cannot
//! use it up before the user clicks.
//!
//! On Android the pages also offer to continue in the app: a `musicroom://`
//! link, which Chrome hands to the app when it is installed. Mail apps rarely
//! make such links clickable, so the emails carry the http link and the page
//! makes the jump.

use axum::{
    Form,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    errors::AppError,
    services::{auth as auth_service, user as user_service},
    state::AppState,
    validation,
};

#[derive(Deserialize)]
pub struct TokenQuery {
    token: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfirmEmailForm {
    token: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordForm {
    token: String,
    new_password: String,
    confirmation: String,
}

pub async fn confirm_email_page(headers: HeaderMap, Query(query): Query<TokenQuery>) -> Response {
    let Some(token) = parse_token(query.token.as_deref()) else {
        return invalid_link();
    };
    let app = open_in_app(&headers, "confirm-email", &token);

    page(
        StatusCode::OK,
        "Confirm your email",
        &format!(
            r#"<p>Confirm that this email address is yours.</p>
{app}<form method="post">
  <input type="hidden" name="token" value="{token}">
  <button type="submit">Confirm my email</button>
</form>"#
        ),
    )
}

pub async fn confirm_email(
    State(state): State<AppState>,
    Form(form): Form<ConfirmEmailForm>,
) -> Response {
    let Some(token) = parse_token(Some(&form.token)) else {
        return invalid_link();
    };

    match user_service::confirm_email(&state.pool, &token).await {
        Ok(()) => page(
            StatusCode::OK,
            "Email confirmed",
            "<p>Your email address is confirmed. You can go back to Music Room.</p>",
        ),
        Err(error) => failure(error),
    }
}

pub async fn reset_password_page(headers: HeaderMap, Query(query): Query<TokenQuery>) -> Response {
    let Some(token) = parse_token(query.token.as_deref()) else {
        return invalid_link();
    };
    let app = open_in_app(&headers, "reset-password", &token);
    reset_form(StatusCode::OK, &token, &app)
}

pub async fn reset_password(
    State(state): State<AppState>,
    Form(form): Form<ResetPasswordForm>,
) -> Response {
    let Some(token) = parse_token(Some(&form.token)) else {
        return invalid_link();
    };

    if form.new_password != form.confirmation {
        return reset_form(
            StatusCode::UNPROCESSABLE_ENTITY,
            &token,
            &error_message("The two passwords are different."),
        );
    }
    if let Err(error) = validation::password(&form.new_password) {
        return reset_form(
            StatusCode::UNPROCESSABLE_ENTITY,
            &token,
            &error_message(error.as_str()),
        );
    }

    match auth_service::reset_password(&state.pool, &token, &form.new_password).await {
        Ok(()) => page(
            StatusCode::OK,
            "Password changed",
            "<p>Your password is changed and every device was logged out. \
             Log in to Music Room with your new password.</p>",
        ),
        Err(error) => failure(error),
    }
}

/// `message` is always one of our own static messages, never user input.
fn error_message(message: &str) -> String {
    format!(r#"<p class="error">{message}</p>"#)
}

/// `before` is HTML shown above the form: an error or the link to the app.
fn reset_form(status: StatusCode, token: &str, before: &str) -> Response {
    page(
        status,
        "Choose a new password",
        &format!(
            r#"{before}<form method="post">
  <input type="hidden" name="token" value="{token}">
  <label>New password
    <input type="password" name="new_password" minlength="8" maxlength="128" autocomplete="new-password" required>
  </label>
  <label>Confirm the new password
    <input type="password" name="confirmation" minlength="8" maxlength="128" autocomplete="new-password" required>
  </label>
  <button type="submit">Change my password</button>
</form>"#
        ),
    )
}

/// On Android, a link that continues in the app (`musicroom://<action>`),
/// which the app handles like the old emails did. Empty elsewhere.
fn open_in_app(headers: &HeaderMap, action: &str, token: &str) -> String {
    let android = headers
        .get(header::USER_AGENT)
        .and_then(|agent| agent.to_str().ok())
        .is_some_and(|agent| agent.contains("Android"));
    if !android {
        return String::new();
    }

    format!(
        r#"<a class="app" href="musicroom://{action}?token={token}">Open in the Music Room app</a>
<p class="or">or continue here</p>
"#
    )
}

/// Tokens are UUIDs. Only a well-formed one is ever written back into a page,
/// in its canonical form, so the page never echoes arbitrary input.
fn parse_token(token: Option<&str>) -> Option<String> {
    token
        .and_then(|token| Uuid::parse_str(token.trim()).ok())
        .map(|token| token.to_string())
}

fn failure(error: AppError) -> Response {
    match error {
        // Unknown, already used or expired token.
        AppError::Unauthorized(_) => invalid_link(),
        error => {
            tracing::error!("Email link page failed: {:?}", error);
            page(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong",
                "<p>Please try again in a moment.</p>",
            )
        }
    }
}

fn invalid_link() -> Response {
    page(
        StatusCode::BAD_REQUEST,
        "This link does not work",
        "<p>It is incomplete, was already used or has expired (links are valid \
         for 15 minutes). Ask for a new one from the Music Room app.</p>",
    )
}

fn page(status: StatusCode, title: &str, body: &str) -> Response {
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · Music Room</title>
<style>
  :root {{ color-scheme: light dark; --accent: #6750a4; }}
  body {{ margin: 0; min-height: 100vh; display: grid; place-items: center;
         font-family: system-ui, sans-serif; background: Canvas; color: CanvasText; }}
  main {{ width: min(420px, 100% - 32px); }}
  h1 {{ font-size: 1.5rem; }}
  form {{ display: grid; gap: 16px; }}
  label {{ display: grid; gap: 6px; }}
  input {{ font: inherit; padding: 10px 12px; border-radius: 8px; border: 1px solid GrayText; }}
  button {{ font: inherit; padding: 12px; border: 0; border-radius: 8px;
           background: var(--accent); color: white; cursor: pointer; }}
  .error {{ color: #b3261e; }}
  .app {{ display: block; padding: 12px; border-radius: 8px; text-align: center;
         text-decoration: none; border: 2px solid var(--accent); color: inherit; }}
  .or {{ text-align: center; color: GrayText; }}
</style>
</head>
<body>
<main>
<h1>{title}</h1>
{body}
</main>
</body>
</html>"#
    );

    let mut response = (status, Html(html)).into_response();
    let headers = response.headers_mut();
    // The URL carries a token: keep it out of caches and referrers, and do
    // not let another site frame the form.
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'",
        ),
    );
    response
}
