//! Custom Axum extractors for validated input
//!
//! This module provides `ValidatedJson<T>` - a drop-in replacement for `Json<T>`
//! that automatically sanitizes and validates incoming JSON payloads.

use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    Json,
};
use chrono::{SecondsFormat, Utc};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

/// A field-level validation error
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Validation error response body
#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub message: String,
    pub errors: Vec<FieldError>,
    pub code: u16,
    pub timestamp: String,
    pub correlation_id: String,
}

impl ValidationErrorResponse {
    pub fn new(errors: Vec<FieldError>) -> Self {
        let error_summary = if errors.len() == 1 {
            format!("Validation failed for field '{}'", errors[0].field)
        } else {
            format!("Validation failed for {} fields", errors.len())
        };

        Self {
            error: "ValidationError".to_string(),
            message: error_summary,
            errors,
            code: 400,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            correlation_id: Uuid::new_v4().to_string(),
        }
    }
}

/// Validation error that converts to an HTTP response
#[derive(Debug)]
pub struct ValidationError {
    pub errors: Vec<FieldError>,
}

impl ValidationError {
    pub fn new(errors: Vec<FieldError>) -> Self {
        Self { errors }
    }

    pub fn single(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![FieldError::new(field, message)],
        }
    }
}

impl axum::response::IntoResponse for ValidationError {
    fn into_response(self) -> axum::response::Response {
        let response = ValidationErrorResponse::new(self.errors);
        (StatusCode::BAD_REQUEST, Json(response)).into_response()
    }
}

/// Trait for types that can be validated and sanitized
///
/// Implement this trait for your request types to enable automatic
/// validation when using `ValidatedJson<T>`.
pub trait Validatable: Sized {
    /// Sanitize the data in-place (trim whitespace, strip HTML, etc.)
    fn sanitize(&mut self);

    /// Validate the data and return any field errors
    fn validate(&self) -> Result<(), Vec<FieldError>>;
}

/// Custom JSON extractor that validates and sanitizes input
///
/// Use this instead of `Json<T>` to automatically:
/// 1. Parse JSON from the request body
/// 2. Sanitize all string fields (trim, strip HTML, normalize)
/// 3. Validate fields against defined rules
/// 4. Return detailed 400 errors for validation failures
///
/// # Example
///
/// ```ignore
/// use crate::validation::{ValidatedJson, Validatable, FieldError};
///
/// pub async fn create_item(
///     ValidatedJson(req): ValidatedJson<CreateRequest>,
/// ) -> impl IntoResponse {
///     // req is already sanitized and validated
///     // ...
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validatable + Send,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Step 1: Parse JSON
        let Json(mut data) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| {
                // Convert JSON parsing errors to validation errors
                let message = match err {
                    axum::extract::rejection::JsonRejection::JsonDataError(e) => {
                        format!("Invalid JSON data: {}", e.body_text())
                    }
                    axum::extract::rejection::JsonRejection::JsonSyntaxError(e) => {
                        format!("JSON syntax error: {}", e.body_text())
                    }
                    axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => {
                        "Content-Type must be application/json".to_string()
                    }
                    axum::extract::rejection::JsonRejection::BytesRejection(_) => {
                        "Failed to read request body".to_string()
                    }
                    _ => "Invalid JSON payload".to_string(),
                };
                ValidationError::single("body", message)
            })?;

        // Step 2: Sanitize the data
        data.sanitize();

        // Step 3: Validate the data
        data.validate().map_err(ValidationError::new)?;

        Ok(ValidatedJson(data))
    }
}

// Implement Deref for ergonomic access
impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Builder for accumulating validation errors
#[derive(Debug, Default)]
pub struct ValidationBuilder {
    errors: Vec<FieldError>,
}

impl ValidationBuilder {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    /// Add an error if the result is Err
    pub fn check<F>(&mut self, field: &str, validator: F) -> &mut Self
    where
        F: FnOnce() -> Result<(), String>,
    {
        if let Err(message) = validator() {
            self.errors.push(FieldError::new(field, message));
        }
        self
    }

    /// Add an error directly
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.errors.push(FieldError::new(field, message));
        self
    }

    /// Add error if condition is true
    pub fn check_condition(
        &mut self,
        condition: bool,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> &mut Self {
        if condition {
            self.errors.push(FieldError::new(field, message));
        }
        self
    }

    /// Finish building and return Result
    pub fn build(self) -> Result<(), Vec<FieldError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get current error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_error() {
        let error = FieldError::new("name", "is required");
        assert_eq!(error.field, "name");
        assert_eq!(error.message, "is required");
    }

    #[test]
    fn test_validation_builder() {
        let mut builder = ValidationBuilder::new();
        
        builder
            .check("name", || Err("is required".to_string()))
            .check("email", || Ok(()))
            .check_condition(true, "age", "must be positive");

        assert!(builder.has_errors());
        assert_eq!(builder.error_count(), 2);

        let result = builder.build();
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].field, "name");
        assert_eq!(errors[1].field, "age");
    }

    #[test]
    fn test_validation_error_response() {
        let errors = vec![
            FieldError::new("contract_id", "is required"),
            FieldError::new("name", "must be at least 1 character"),
        ];
        
        let response = ValidationErrorResponse::new(errors);
        
        assert_eq!(response.error, "ValidationError");
        assert_eq!(response.code, 400);
        assert_eq!(response.errors.len(), 2);
        assert!(response.message.contains("2 fields"));
    }

    #[test]
    fn test_single_error_response() {
        let errors = vec![FieldError::new("name", "is required")];
        let response = ValidationErrorResponse::new(errors);
        
        assert!(response.message.contains("field 'name'"));
    }
}
