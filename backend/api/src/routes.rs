use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{handlers, metrics_handler, resource_handlers, state::AppState};

pub fn observability_routes() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler::metrics_endpoint))
}

pub fn contract_routes() -> Router<AppState> {
    Router::new()
        .route("/api/contracts", get(handlers::list_contracts))
        .route("/api/contracts", post(handlers::publish_contract))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route("/api/contracts/:id/abi", get(handlers::get_contract_abi))
        .route(
            "/api/contracts/:id/versions",
            get(handlers::get_contract_versions),
        )
        .route("/api/contracts/verify", post(handlers::verify_contract))
        .route(
            "/api/contracts/:id/state/:key",
            get(handlers::get_contract_state).post(handlers::update_contract_state),
        )
}

/// Publisher-related routes
pub fn publisher_routes() -> Router<AppState> {
    Router::new()
        .route("/api/publishers", post(handlers::create_publisher))
        .route("/api/publishers/:id", get(handlers::get_publisher))
        .route(
            "/api/publishers/:id/contracts",
            get(handlers::get_publisher_contracts),
        )
}

/// Health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/stats", get(handlers::get_stats))
        .route("/api/cache/stats", get(handlers::get_cache_stats))
}

/// Migration-related routes
pub fn migration_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/migrations",
            post(handlers::migrations::create_migration).get(handlers::migrations::get_migrations),
        )
        .route(
            "/api/migrations/:id",
            put(handlers::migrations::update_migration).get(handlers::migrations::get_migration),
        )
}

pub fn canary_routes() -> Router<AppState> {
    Router::new()
}

pub fn ab_test_routes() -> Router<AppState> {
    Router::new()
}

pub fn performance_routes() -> Router<AppState> {
    Router::new()
}

pub fn resource_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/resources",
            get(resource_handlers::get_contract_resources),
        )
        .route(
            "/contracts/:id/resources",
            get(resource_handlers::get_contract_resources),
        )
}
