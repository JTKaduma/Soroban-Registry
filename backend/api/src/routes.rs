use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    activity_feed_handlers,
    compatibility_handlers,
    handlers,
    metrics_handler,
    resource_handlers,
    auth_handlers,
    state::AppState,
};

pub fn observability_routes() -> Router<AppState> {
    Router::new()
}

pub fn contract_routes() -> Router<AppState> {
    Router::new()
        .route("/api/contracts", get(handlers::list_contracts))
        .route("/api/contracts", post(handlers::publish_contract))
        .route("/api/contracts/trending", get(handlers::get_trending_contracts))
        .route("/api/contracts/graph", get(handlers::get_contract_graph))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route("/api/contracts/:id/abi", get(handlers::get_contract_abi))
        .route("/api/contracts/:id/versions", get(handlers::get_contract_versions))
        .route("/api/contracts/:id/state/:key", get(handlers::get_contract_state).post(handlers::update_contract_state))
        .route("/api/contracts/:id/analytics", get(handlers::get_contract_analytics))
        .route("/api/contracts/:id/trust-score", get(handlers::get_trust_score))
        .route("/api/contracts/:id/dependencies", get(handlers::get_contract_dependencies))
        .route("/api/contracts/:id/dependents", get(handlers::get_contract_dependents))
        .route("/api/contracts/verify", post(handlers::verify_contract))
        .route(
            "/api/contracts/:id/state/:key",
            get(handlers::get_contract_state).post(handlers::update_contract_state),
        )
        .route(
            "/api/contracts/:id/performance",
            get(handlers::get_contract_performance),
        )
        .route(
            "/api/contracts/:id/compatibility",
            get(compatibility_handlers::get_contract_compatibility)
                .post(compatibility_handlers::add_contract_compatibility),
        )
        .route(
            "/api/contracts/:id/compatibility/export",
            get(compatibility_handlers::export_contract_compatibility),
        )
        .route("/api/contracts/:id/deployments/status", get(handlers::get_deployment_status))
        .route("/api/deployments/green", post(handlers::deploy_green))
}

pub fn publisher_routes() -> Router<AppState> {
    Router::new()
}

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/stats", get(handlers::get_stats))
        .route(
            "/api/activity-feed",
            get(activity_feed_handlers::get_activity_feed),
        )
}

pub fn migration_routes() -> Router<AppState> {
    Router::new()
}
