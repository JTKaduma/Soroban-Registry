//! Payload size validation middleware
//!
//! This middleware enforces maximum request body size limits to prevent
//! denial-of-service attacks and consume excessive resources.
//!
//! Configuration via environment variables:
//! - MAX_PAYLOAD_SIZE_MB: Maximum payload size in MB (default: 5)

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, MatchedPath},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use std::net::SocketAddr;
use uuid::Uuid;
use chrono::{Utc, SecondsFormat};

const DEFAULT_MAX_PAYLOAD_MB: u64 = 5;
const HEADER_CONTENT_LENGTH: &str = "content-length";

#[derive(Debug, Serialize)]
struct PayloadTooLargeResponse {
    error: String,
    message: String,
    code: u16,
    max_size_mb: u64,
    max_size_bytes: u64,
    timestamp: String,
    correlation_id: String,
}

/// Get configured max payload size in bytes
pub fn get_max_payload_bytes() -> u64 {
    let env_mb = std::env::var("MAX_PAYLOAD_SIZE_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_PAYLOAD_MB);

    env_mb * 1024 * 1024
}

/// Middleware that validates request payload size
///
/// Returns 413 Payload Too Large if the request body exceeds the configured limit.
/// The limit is checked via the Content-Length header when available.
pub async fn payload_size_validation_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    matched_path: Option<MatchedPath>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let max_bytes = get_max_payload_bytes();

    // Check Content-Length header
    if let Some(content_length_str) = req.headers().get(HEADER_CONTENT_LENGTH) {
        if let Ok(content_length_str) = content_length_str.to_str() {
            if let Ok(size) = content_length_str.parse::<u64>() {
                if size > max_bytes {
                    let correlation_id = Uuid::new_v4().to_string();
                    let max_mb = max_bytes / (1024 * 1024);

                    // Log the violation
                    let client_ip = addr.ip();
                    let path = matched_path
                        .as_ref()
                        .map(|p| p.as_str())
                        .unwrap_or("unknown");
                    
                    crate::security_log::log_payload_too_large(
                        client_ip,
                        size as usize,
                        max_bytes as usize,
                        path,
                        &correlation_id,
                    );

                    let response = PayloadTooLargeResponse {
                        error: "PayloadTooLarge".to_string(),
                        message: format!(
                            "Request payload exceeds maximum size of {} MB ({} bytes)",
                            max_mb, max_bytes
                        ),
                        code: 413,
                        max_size_mb: max_mb,
                        max_size_bytes: max_bytes,
                        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
                        correlation_id,
                    };

                    return Err((StatusCode::PAYLOAD_TOO_LARGE, Json(response)).into_response());
                }
            }
        }
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_max_payload() {
        // Should return 5MB in bytes by default
        let max = get_max_payload_bytes();
        assert_eq!(max, 5 * 1024 * 1024);
    }
}
