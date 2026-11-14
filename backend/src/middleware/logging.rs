use axum::{extract::Request, middleware::Next, response::Response};
use tracing::info;

/// The path and query of a request with the value of secret parameters hidden.
/// Browsers cannot set headers on a WebSocket upgrade, so the web client sends
/// its access token as `?token=`; it must never reach the logs.
fn redacted_target(uri: &axum::http::Uri) -> String {
    let Some(query) = uri.query() else {
        return uri.path().to_string();
    };

    let query = query
        .split('&')
        .map(|param| match param.split_once('=') {
            Some((name, _)) if name.eq_ignore_ascii_case("token") => format!("{name}=<redacted>"),
            _ => param.to_string(),
        })
        .collect::<Vec<_>>()
        .join("&");

    format!("{}?{}", uri.path(), query)
}

pub async fn request_logger(request: Request, next: Next) -> Response {
    {
        let header = |name: &str| {
            request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("Unknown")
        };

        info!(
            "{} {} - Platform: {}, Device: {}, App-Version: {}",
            request.method(),
            redacted_target(request.uri()),
            header("X-Platform"),
            header("X-Device"),
            header("X-App-Version"),
        );
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_query_parameter_is_never_logged() {
        let uri: axum::http::Uri = "/rooms/42/ws?token=secret.jwt.value&device=Pixel"
            .parse()
            .unwrap();
        let logged = redacted_target(&uri);

        assert!(!logged.contains("secret"));
        assert_eq!(logged, "/rooms/42/ws?token=<redacted>&device=Pixel");
    }

    #[test]
    fn requests_without_a_query_are_logged_as_is() {
        let uri: axum::http::Uri = "/rooms".parse().unwrap();
        assert_eq!(redacted_target(&uri), "/rooms");
    }
}
