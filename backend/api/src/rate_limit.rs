use std::{
    collections::HashMap,
    env,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, MatchedPath, State},
    http::{
        header::{AUTHORIZATION, RETRY_AFTER},
        HeaderName, HeaderValue, Method, Request, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

const DEFAULT_READ_LIMIT_PER_MINUTE: u32 = 100;
const DEFAULT_WRITE_LIMIT_PER_MINUTE: u32 = 20;
const DEFAULT_AUTH_LIMIT_PER_MINUTE: u32 = 1_000;
const DEFAULT_HEALTH_LIMIT_PER_MINUTE: u32 = 10_000;
const DEFAULT_WINDOW_SECONDS: u64 = 60;
const ENDPOINT_LIMIT_ENV_PREFIX: &str = "RATE_LIMIT_ENDPOINT_";

const HEADER_RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const HEADER_RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const HEADER_RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");

#[derive(Clone)]
pub struct RateLimitState {
    config: Arc<RateLimitConfig>,
    buckets: Arc<Mutex<HashMap<BucketKey, BucketState>>>,
}

impl RateLimitState {
    pub fn from_env() -> Self {
        Self::new(RateLimitConfig::from_env())
    }

    fn new(config: RateLimitConfig) -> Self {
        Self {
            config: Arc::new(config),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check_request<B>(&self, request: &Request<B>) -> RateLimitDecision {
        let (limit, endpoint_key) = self.select_limit(request);
        let ip = extract_client_ip(request);
        let key = BucketKey { ip, endpoint_key };
        let now = Instant::now();

        let mut buckets = self.buckets.lock().expect("rate limiter mutex poisoned");

        let bucket = buckets.entry(key).or_insert_with(|| BucketState {
            window_start: now,
            count: 0,
        });

        if now.duration_since(bucket.window_start) >= self.config.window {
            bucket.window_start = now;
            bucket.count = 0;
        }

        let remaining_window = self
            .config
            .window
            .saturating_sub(now.duration_since(bucket.window_start));
        let reset_seconds = ceil_duration_to_seconds(remaining_window).max(1);

        if bucket.count >= limit {
            return RateLimitDecision {
                allowed: false,
                limit,
                remaining: 0,
                reset_seconds,
            };
        }

        bucket.count += 1;
        let remaining = limit.saturating_sub(bucket.count);

        RateLimitDecision {
            allowed: true,
            limit,
            remaining,
            reset_seconds,
        }
    }

    fn select_limit<B>(&self, request: &Request<B>) -> (u32, String) {
        let method = request.method();
        let matched_path = request
            .extensions()
            .get::<MatchedPath>()
            .map(|p| p.as_str())
            .unwrap_or_else(|| request.uri().path());
        let endpoint_key = endpoint_key(method, matched_path);

        if let Some(limit) = self.config.endpoint_limits.get(&endpoint_key) {
            return (*limit, endpoint_key);
        }

        if matched_path == "/health" || method == Method::OPTIONS {
            return (self.config.health_limit, endpoint_key);
        }

        if request.headers().contains_key(AUTHORIZATION) {
            return (self.config.auth_limit, endpoint_key);
        }

        if is_write_method(method) {
            return (self.config.write_limit, endpoint_key);
        }

        (self.config.read_limit, endpoint_key)
    }
}

struct RateLimitConfig {
    read_limit: u32,
    write_limit: u32,
    auth_limit: u32,
    health_limit: u32,
    window: Duration,
    endpoint_limits: HashMap<String, u32>,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        let read_limit = env_u32("RATE_LIMIT_READ_PER_MINUTE", DEFAULT_READ_LIMIT_PER_MINUTE);
        let write_limit = env_u32(
            "RATE_LIMIT_WRITE_PER_MINUTE",
            DEFAULT_WRITE_LIMIT_PER_MINUTE,
        );
        let auth_limit = env_u32("RATE_LIMIT_AUTH_PER_MINUTE", DEFAULT_AUTH_LIMIT_PER_MINUTE);
        let health_limit = env_u32(
            "RATE_LIMIT_HEALTH_PER_MINUTE",
            DEFAULT_HEALTH_LIMIT_PER_MINUTE,
        );
        let window_seconds = env_u64("RATE_LIMIT_WINDOW_SECONDS", DEFAULT_WINDOW_SECONDS).max(1);

        let mut endpoint_limits = HashMap::new();
        for (key, value) in env::vars() {
            let Some(endpoint_key) = key.strip_prefix(ENDPOINT_LIMIT_ENV_PREFIX) else {
                continue;
            };

            let Ok(limit) = value.parse::<u32>() else {
                tracing::warn!("Ignoring invalid endpoint rate limit `{key}`: `{value}`");
                continue;
            };
            if limit == 0 {
                tracing::warn!("Ignoring zero endpoint rate limit `{key}`");
                continue;
            }

            endpoint_limits.insert(endpoint_key.to_string(), limit);
        }

        tracing::info!(
            read_limit,
            write_limit,
            auth_limit,
            health_limit,
            window_seconds,
            endpoint_overrides = endpoint_limits.len(),
            "Rate limiter configured"
        );

        Self {
            read_limit,
            write_limit,
            auth_limit,
            health_limit,
            window: Duration::from_secs(window_seconds),
            endpoint_limits,
        }
    }

    #[cfg(test)]
    fn for_tests(read_limit: u32, write_limit: u32, health_limit: u32, window: Duration) -> Self {
        Self {
            read_limit,
            write_limit,
            auth_limit: DEFAULT_AUTH_LIMIT_PER_MINUTE,
            health_limit,
            window,
            endpoint_limits: HashMap::new(),
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
struct BucketKey {
    ip: String,
    endpoint_key: String,
}

struct BucketState {
    window_start: Instant,
    count: u32,
}

struct RateLimitDecision {
    allowed: bool,
    limit: u32,
    remaining: u32,
    reset_seconds: u64,
}

pub async fn rate_limit_middleware(
    State(rate_limiter): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let decision = rate_limiter.check_request(&request);

    if !decision.allowed {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "rate limit exceeded" })),
        )
            .into_response();
        attach_rate_limit_headers(&mut response, &decision);
        response.headers_mut().insert(
            RETRY_AFTER,
            HeaderValue::from_str(&decision.reset_seconds.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1")),
        );
        return response;
    }

    let mut response = next.run(request).await;
    attach_rate_limit_headers(&mut response, &decision);
    response
}

fn attach_rate_limit_headers(response: &mut Response, decision: &RateLimitDecision) {
    response.headers_mut().insert(
        HEADER_RATE_LIMIT_LIMIT,
        HeaderValue::from_str(&decision.limit.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    response.headers_mut().insert(
        HEADER_RATE_LIMIT_REMAINING,
        HeaderValue::from_str(&decision.remaining.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    response.headers_mut().insert(
        HEADER_RATE_LIMIT_RESET,
        HeaderValue::from_str(&decision.reset_seconds.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("1")),
    );
}

fn extract_client_ip<B>(request: &Request<B>) -> String {
    if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_x_forwarded_for)
    {
        return ip.to_string();
    }

    if let Some(ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_addr)
    {
        return ip.to_string();
    }

    if let Some(connect_info) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    "unknown".to_string()
}

fn parse_x_forwarded_for(raw: &str) -> Option<IpAddr> {
    raw.split(',').map(str::trim).find_map(parse_ip_addr)
}

fn parse_ip_addr(raw: &str) -> Option<IpAddr> {
    raw.parse::<IpAddr>()
        .ok()
        .or_else(|| raw.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
}

fn is_write_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn endpoint_key(method: &Method, path: &str) -> String {
    let normalized_path = path
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    let compact_path = normalized_path
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    if compact_path.is_empty() {
        format!("{}_ROOT", method.as_str().to_ascii_uppercase())
    } else {
        format!("{}_{}", method.as_str().to_ascii_uppercase(), compact_path)
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn ceil_duration_to_seconds(duration: Duration) -> u64 {
    let secs = duration.as_secs();
    if duration.subsec_nanos() > 0 {
        secs + 1
    } else {
        secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::Request,
        middleware,
        routing::{get, post},
        Router,
    };
    use tower::Service;

    fn test_app(
        read_limit: u32,
        write_limit: u32,
        health_limit: u32,
        window: Duration,
    ) -> Router<()> {
        let limiter = RateLimitState::new(RateLimitConfig::for_tests(
            read_limit,
            write_limit,
            health_limit,
            window,
        ));

        Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/read", get(|| async { "read" }))
            .route("/write", post(|| async { "write" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ))
    }

    async fn call(app: &Router<()>, request: Request<Body>) -> Response {
        let mut svc = app.clone();
        svc.call(request).await.unwrap()
    }

    #[tokio::test]
    async fn returns_429_on_101st_request() {
        let app = test_app(100, 20, 10_000, Duration::from_secs(60));

        for _ in 0..100 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/read")
                    .method("GET")
                    .header("x-forwarded-for", "203.0.113.10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "203.0.113.10")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn includes_rate_limit_headers_on_success_and_429() {
        let app = test_app(1, 1, 10_000, Duration::from_secs(60));

        let ok_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "198.51.100.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(ok_response.status(), StatusCode::OK);
        assert!(ok_response.headers().contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(ok_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(ok_response.headers().contains_key(HEADER_RATE_LIMIT_RESET));

        let limited_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "198.51.100.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(limited_response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_RESET));
        assert!(limited_response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn allows_requests_again_after_window_reset() {
        let app = test_app(1, 1, 10_000, Duration::from_secs(1));

        let first = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);

        let second = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);

        tokio::time::sleep(Duration::from_secs(2)).await;

        let third = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(third.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn write_limits_are_stricter_than_reads() {
        let app = test_app(3, 1, 10_000, Duration::from_secs(60));
        let ip = "203.0.113.33";

        let write_ok = call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(write_ok.status(), StatusCode::OK);

        let write_limited = call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(write_limited.status(), StatusCode::TOO_MANY_REQUESTS);

        let read_ok = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(read_ok.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_checks_have_high_dedicated_limit() {
        let app = test_app(1, 1, 10, Duration::from_secs(60));
        let ip = "198.51.100.99";

        for _ in 0..10 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/health")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_eq!(response.status(), StatusCode::OK);
        }

        let limited = call(
            &app,
            Request::builder()
                .uri("/health")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
