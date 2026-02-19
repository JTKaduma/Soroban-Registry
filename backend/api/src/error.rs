use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    error: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    code: u16,
    timestamp: String,
    correlation_id: String,
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            error: error.into(),
            message: message.into(),
        }
    }

    pub fn bad_request(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error, message)
    }

    pub fn not_found(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, error, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "InternalServerError", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let correlation_id = Uuid::new_v4().to_string();
        let payload = ErrorResponse {
            error: self.error,
            message: self.message,
            code: self.status.as_u16(),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            correlation_id: correlation_id.clone(),
        };

        let mut response = (self.status, Json(payload)).into_response();
        if let Ok(value) = HeaderValue::from_str(&correlation_id) {
            response.headers_mut().insert(header::HeaderName::from_static("x-correlation-id"), value);
        }
        response
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;
