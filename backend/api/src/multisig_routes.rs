// multisig_routes.rs
// Route definitions for Multi-Signature Contract Deployment (issue #47)

use axum::{
    routing::{get, post},
    Router,
};

use crate::{multisig_handlers, state::AppState};

/// Multi-sig policy and proposal routes
pub fn multisig_routes() -> Router<AppState> {
    Router::new()
        // Policy management
        .route(
            "/api/multisig/policies",
            post(multisig_handlers::create_policy),
        )
        // Proposal listing (all proposals, filterable by status/policy)
        .route(
            "/api/multisig/proposals",
            get(multisig_handlers::list_proposals),
        )
        // Create an unsigned proposal (spec: POST /contracts/deploy-proposal)
        .route(
            "/api/contracts/deploy-proposal",
            post(multisig_handlers::create_proposal),
        )
        // Add a signer's approval (spec: POST /contracts/{id}/sign)
        .route(
            "/api/contracts/:id/sign",
            post(multisig_handlers::sign_proposal),
        )
        // Execute an approved proposal (spec: POST /contracts/{id}/execute)
        .route(
            "/api/contracts/:id/execute",
            post(multisig_handlers::execute_proposal),
        )
        // Retrieve full proposal info with signatures and policy
        .route(
            "/api/contracts/:id/proposal",
            get(multisig_handlers::get_proposal),
        )
}
