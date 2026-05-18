use axum::{extract::Request, middleware::Next, response::Response};
use tracing::info;

pub async fn request_logger(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let platform = request
        .headers()
        .get("X-Platform")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown");

    let device = request
        .headers()
        .get("X-Device")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown");

    let version = request
        .headers()
        .get("X-App-Version")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown");

    info!(
        "{} {} - Platform: {}, Device: {}, App-Version: {}",
        method, uri, platform, device, version
    );

    next.run(request).await
}
