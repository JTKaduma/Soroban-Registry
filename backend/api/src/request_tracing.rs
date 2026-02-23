//! Structured request tracing middleware.
//!
//! Every incoming HTTP request gets a unique UUID (`X-Request-ID`), and a
//! JSON-structured log line is emitted after the response is sent.
//!
//! Health-check endpoints are intentionally skipped so they don't pollute
//! the log stream.
//!
//! Log fields:
//!   timestamp, request_id, method, path, status, duration_ms, user_ip

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::time::Instant;
use uuid::Uuid;

/// Paths that should never be logged (health checks, readiness probes, etc.)
const SKIP_LOG_PATHS: &[&str] = &["/health", "/healthz", "/ready", "/ping", "/metrics"];

/// The response header name carrying the request ID back to the caller.
pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Axum middleware: attach a request ID, log the completed request as JSON,
/// and add the `X-Request-ID` header to the response.
pub async fn tracing_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let user_ip = addr.ip().to_string();

    // Inject the request ID into extensions so handlers / DB layers can read it
    req.extensions_mut().insert(RequestId(request_id.clone()));

    let start = Instant::now();
    let mut response = next.run(req).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    // Attach X-Request-ID to the response so clients can correlate logs
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID.clone(), val);
    }

    // Skip noisy health-check paths
    if SKIP_LOG_PATHS.iter().any(|p| path.starts_with(p)) {
        return response;
    }

    let status = response.status().as_u16();

    // Emit a single structured JSON log line per request
    tracing::info!(
        request_id = %request_id,
        method     = %method,
        path       = %path,
        status     = status,
        duration_ms = duration_ms,
        user_ip    = %user_ip,
        "request"
    );

    response
}

// ── Request ID extractor ──────────────────────────────────────────────────────

/// A newtype wrapper stored in request extensions so downstream code can
/// cheaply retrieve the current request ID without re-parsing headers.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Retrieve the request ID from Axum request extensions, if present.
    pub fn from_request(req: &Request<Body>) -> Option<&str> {
        req.extensions()
            .get::<RequestId>()
            .map(|r| r.0.as_str())
    }
}

// ── JSON tracing subscriber initialiser ──────────────────────────────────────

/// Initialise `tracing-subscriber` with a JSON formatter suitable for
/// ELK / Splunk / Datadog ingestion.
///
/// Call this **once** at application startup, replacing the plain-text
/// subscriber currently set up in `main.rs`.
///
/// Log rotation (daily, 7-day retention) is handled by the deployment
/// environment (e.g. logrotate, Docker log driver, or a dedicated log
/// shipper). The subscriber itself writes to stdout so the runtime can
/// redirect / rotate as needed.
pub fn init_json_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json()) // structured JSON output
        .init();
}
