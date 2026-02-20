//! Input Validation Module
//!
//! This module provides comprehensive input validation and sanitization
//! for the Soroban Registry API.
//!
//! # Overview
//!
//! The validation system consists of three main components:
//!
//! 1. **Extractors** - Custom Axum extractors like `ValidatedJson<T>`
//! 2. **Validators** - Reusable validation functions for common patterns
//! 3. **Sanitizers** - Functions to clean and normalize input data
//!
//! # Usage
//!
//! ## Using ValidatedJson in Handlers
//!
//! ```ignore
//! use crate::validation::{ValidatedJson, Validatable, FieldError, ValidationBuilder};
//!
//! // Implement Validatable for your request type
//! impl Validatable for MyRequest {
//!     fn sanitize(&mut self) {
//!         self.name = sanitizers::sanitize_name(&self.name);
//!     }
//!
//!     fn validate(&self) -> Result<(), Vec<FieldError>> {
//!         ValidationBuilder::new()
//!             .check("name", || validators::validate_length(&self.name, 1, 255))
//!             .build()
//!     }
//! }
//!
//! // Use in handler - validation happens automatically
//! pub async fn create_item(
//!     ValidatedJson(req): ValidatedJson<MyRequest>,
//! ) -> impl IntoResponse {
//!     // req is sanitized and validated
//! }
//! ```
//!
//! ## Validation Error Response
//!
//! When validation fails, a 400 Bad Request is returned:
//!
//! ```json
//! {
//!   "error": "ValidationError",
//!   "message": "Validation failed for 2 fields",
//!   "errors": [
//!     {"field": "contract_id", "message": "must be a valid Stellar contract ID"},
//!     {"field": "name", "message": "must be at least 1 character"}
//!   ],
//!   "code": 400,
//!   "timestamp": "2026-02-20T10:30:00Z",
//!   "correlation_id": "uuid-here"
//! }
//! ```

pub mod extractors;
pub mod requests;
pub mod sanitizers;
pub mod validators;

// Re-export commonly used items
pub use extractors::{FieldError, Validatable, ValidatedJson, ValidationBuilder, ValidationError};
pub use sanitizers::{
    normalize_contract_id, normalize_stellar_address, sanitize_description,
    sanitize_description_optional, sanitize_name, sanitize_tags, sanitize_url_optional, strip_html,
    trim, trim_optional,
};
pub use validators::{
    validate_contract_id, validate_length, validate_no_html, validate_no_xss, validate_required,
    validate_semver, validate_source_code_size, validate_stellar_address,
    validate_stellar_address_optional, validate_tags, validate_url, validate_url_optional,
};
