use axum::{routing::get, routing::post, Router};

use crate::{governance_handlers, state::AppState};

pub fn governance_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/governance/proposals",
            post(governance_handlers::create_proposal).get(governance_handlers::list_proposals),
        )
        .route(
            "/api/governance/proposals/:id",
            get(governance_handlers::get_proposal),
        )
        .route(
            "/api/governance/proposals/:id/vote",
            post(governance_handlers::cast_vote),
        )
        .route(
            "/api/governance/proposals/:id/results",
            get(governance_handlers::get_proposal_results),
        )
        .route(
            "/api/governance/proposals/:id/execute",
            post(governance_handlers::execute_proposal),
        )
        .route(
            "/api/contracts/:id/governance/delegate",
            post(governance_handlers::delegate_vote),
        )
        .route(
            "/api/governance/delegations/:id/revoke",
            post(governance_handlers::revoke_delegation),
        )
}
