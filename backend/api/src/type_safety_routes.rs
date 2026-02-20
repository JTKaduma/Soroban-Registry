//! Routes for Contract Type Safety Validation
//!
//! Defines API routes for validating contract function calls
//! and generating type-safe bindings.

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;
use crate::type_safety_handlers;

/// Create router for type safety validation endpoints
pub fn type_safety_routes() -> Router<AppState> {
    Router::new()
        // Validate a contract function call
        .route(
            "/api/contracts/:id/validate-call",
            post(type_safety_handlers::validate_call),
        )
        // List all functions on a contract
        .route(
            "/api/contracts/:id/functions",
            get(type_safety_handlers::list_contract_functions),
        )
        // Get info about a specific function
        .route(
            "/api/contracts/:id/functions/:method",
            get(type_safety_handlers::get_function_info),
        )
        // Generate type-safe bindings
        .route(
            "/api/contracts/:id/bindings",
            get(type_safety_handlers::generate_contract_bindings),
        )
}
