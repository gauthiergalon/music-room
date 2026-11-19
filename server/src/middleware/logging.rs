use axum::{extract::Request, middleware::Next, response::Response};
use tracing::{debug, info};

/// Query parameters that grant access by themselves and must never be logged:
/// the access token the web client sends on a WebSocket upgrade (a browser
/// cannot set headers there) and the signature of a relayed stream link.
const SECRET_PARAMETERS: [&str; 2] = ["token", "sig"];

/// Requests of the stream relay: one per audio segment and per web listener.
const RELAY_PREFIX: &str = "/hifi/stream/";

/// The path and query of a request with the value of secret parameters hidden.
fn redacted_target(uri: &axum::http::Uri) -> String {
    let Some(query) = uri.query() else {
        return uri.path().to_string();
    };

    let query = query
        .split('&')
        .map(|param| match param.split_once('=') {
            Some((name, _))
                if SECRET_PARAMETERS
                    .iter()
                    .any(|secret| name.eq_ignore_ascii_case(secret)) =>
            {
                format!("{name}=<redacted>")
            }
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
        let target = redacted_target(request.uri());

        if request.uri().path().starts_with(RELAY_PREFIX) {
            // Far too many to be useful at the default level.
            debug!("{} {}", request.method(), target);
        } else {
            info!(
                "{} {} - Platform: {}, Device: {}, App-Version: {}",
                request.method(),
                target,
                header("X-Platform"),
                header("X-Device"),
                header("X-App-Version"),
            );
        }
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

    #[test]
    fn the_signature_of_a_relayed_stream_link_is_never_logged() {
        let uri: axum::http::Uri =
            "/hifi/stream/segment?u=aHR0cHM6Ly9l&exp=1790000000&sig=deadbeef"
                .parse()
                .unwrap();
        let logged = redacted_target(&uri);

        assert!(!logged.contains("deadbeef"));
        assert_eq!(
            logged,
            "/hifi/stream/segment?u=aHR0cHM6Ly9l&exp=1790000000&sig=<redacted>"
        );
    }
}
