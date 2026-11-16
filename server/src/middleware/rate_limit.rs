//! Fixed-window rate limiting for the unauthenticated auth routes
//! (login, register, password reset), keyed by client IP.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    errors::{AppError, ErrorMessage},
    state::AppState,
};

/// Above this many tracked clients, expired windows are dropped on the next call.
const PRUNE_THRESHOLD: usize = 10_000;

pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    hits: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records one request for `key` and tells whether it is allowed.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter lock poisoned");

        if hits.len() > PRUNE_THRESHOLD {
            hits.retain(|_, (started, _)| now.duration_since(*started) < self.window);
        }

        let entry = hits.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.max_requests
    }
}

/// The address of the client. Behind a reverse proxy the socket peer is the
/// proxy, so the first `X-Forwarded-For` entry is used when present. That
/// header is only trustworthy when a proxy we control sets it.
fn client_key(request: &Request) -> String {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn auth_rate_limit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(limiter) = &state.auth_rate_limiter
        && !limiter.check(&client_key(&request))
    {
        return Err(AppError::TooManyRequests(ErrorMessage::RateLimited));
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_per_key() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));

        assert!(limiter.check("1.1.1.1"));
        assert!(limiter.check("1.1.1.1"));
        assert!(!limiter.check("1.1.1.1"));
        // Another client has its own budget.
        assert!(limiter.check("2.2.2.2"));
    }

    #[test]
    fn a_new_window_resets_the_count() {
        let limiter = RateLimiter::new(1, Duration::from_millis(20));

        assert!(limiter.check("1.1.1.1"));
        assert!(!limiter.check("1.1.1.1"));
        std::thread::sleep(Duration::from_millis(30));
        assert!(limiter.check("1.1.1.1"));
    }
}
