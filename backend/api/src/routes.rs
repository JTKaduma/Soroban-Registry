use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    breaking_changes, compatibility_testing_handlers, custom_metrics_handlers,
    deprecation_handlers, handlers, metrics_handler, migration_handlers, state::AppState,
};

pub fn observability_routes() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler::metrics_endpoint))
}

pub fn contract_routes() -> Router<AppState> {
    Router::new()
        .route("/api/contracts", get(handlers::list_contracts))
        .route("/api/contracts", post(handlers::publish_contract))
        .route(
            "/api/contracts/trending",
            get(handlers::get_trending_contracts),
        )
        .route("/api/contracts/graph", get(handlers::get_contract_graph))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route(
            "/api/contracts/:id/metadata",
            patch(handlers::update_contract_metadata),
        )
        .route(
            "/api/contracts/:id/publisher",
            patch(handlers::change_contract_publisher),
        )
        .route(
            "/api/contracts/:id/status",
            patch(handlers::update_contract_status),
        )
        .route(
            "/api/contracts/:id/audit-log",
            get(handlers::get_contract_audit_log),
        )
        .route("/api/contracts/:id/abi", get(handlers::get_contract_abi))
        .route(
            "/api/contracts/:id/openapi.yaml",
            get(handlers::get_contract_openapi_yaml),
        )
        .route(
            "/api/contracts/:id/openapi.json",
            get(handlers::get_contract_openapi_json),
        )
        .route(
            "/api/contracts/:id/versions",
            get(handlers::get_contract_versions).post(handlers::create_contract_version),
        )
        .route(
            "/api/contracts/:id/changelog",
            get(handlers::get_contract_changelog),
        )
        // Compatibility alias (spec asks for /contracts/{id}/changelog)
        .route(
            "/contracts/:id/changelog",
            get(handlers::get_contract_changelog),
        )
        .route(
            "/api/contracts/breaking-changes",
            get(breaking_changes::get_breaking_changes),
        )
        .route(
            "/api/contracts/:id/versions",
            get(handlers::get_contract_versions),
        )
        .route(
            "/api/contracts/:id/interactions",
            get(handlers::get_contract_interactions).post(handlers::post_contract_interaction),
        )
        .route(
            "/api/contracts/:id/interactions/batch",
            post(handlers::post_contract_interactions_batch),
        )
        .route(
            "/api/contracts/:id/deprecation-info",
            get(deprecation_handlers::get_deprecation_info),
        )
        .route(
            "/api/contracts/:id/deprecate",
            post(deprecation_handlers::deprecate_contract),
        )
        .route(
            "/api/contracts/:id/state/:key",
            get(handlers::get_contract_state).post(handlers::update_contract_state),
        )
        .route(
            "/api/contracts/:id/analytics",
            get(handlers::get_contract_analytics),
        )
        .route(
            "/api/contracts/:id/trust-score",
            get(handlers::get_trust_score),
        )
        .route(
            "/api/contracts/:id/dependencies",
            get(handlers::get_contract_dependencies),
        )
        .route(
            "/api/contracts/:id/dependents",
            get(handlers::get_contract_dependents),
        )
        .route(
            "/api/contracts/:id/impact",
            get(handlers::get_impact_analysis),
        )
        .route("/api/contracts/:id/deprecation-info", get(deprecation_handlers::get_deprecation_info))
        .route("/api/contracts/:id/deprecate", post(deprecation_handlers::deprecate_contract))
        .route("/api/contracts/:id/state/:key", get(handlers::get_contract_state).post(handlers::update_contract_state))
        .route("/api/contracts/:id/analytics", get(handlers::get_contract_analytics))
        .route("/api/contracts/:id/trust-score", get(handlers::get_trust_score))
        .route("/api/contracts/:id/dependencies", get(handlers::get_contract_dependencies))
        .route("/api/contracts/:id/dependents", get(handlers::get_contract_dependents))
        .route("/api/contracts/:id/impact", get(handlers::get_impact_analysis))
        .route("/api/contracts/verify", post(handlers::verify_contract))
        .route("/api/admin/audit-logs", get(handlers::get_all_audit_logs))
        .route(
            "/api/contracts/:id/performance",
            get(handlers::get_contract_performance),
        )
        .route(
            "/api/contracts/:id/metrics",
            get(custom_metrics_handlers::get_contract_metrics)
                .post(custom_metrics_handlers::record_contract_metric),
        )
        .route(
            "/api/contracts/:id/metrics/batch",
            post(custom_metrics_handlers::record_metrics_batch),
        )
        .route(
            "/api/contracts/:id/metrics/catalog",
            get(custom_metrics_handlers::get_metric_catalog),
        )
        // SDK / Wasm / Network Compatibility Testing Matrix (Issue #261)
        .route(
            "/api/contracts/:id/compatibility-matrix",
            get(compatibility_testing_handlers::get_compatibility_matrix),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/test",
            post(compatibility_testing_handlers::run_compatibility_test),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/history",
            get(compatibility_testing_handlers::get_compatibility_history),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/notifications",
            get(compatibility_testing_handlers::get_compatibility_notifications),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/notifications/read",
            post(compatibility_testing_handlers::mark_notifications_read),
        )
        .route(
            "/api/contracts/:id/deployments/status",
            get(handlers::get_deployment_status),
        )
        .route("/api/deployments/green", post(handlers::deploy_green))
    // TODO: backup_routes, notification_routes, and post_incident_routes
    // are available in the api library crate but need architectural refactoring
    // to be integrated with the main AppState
}

pub fn publisher_routes() -> Router<AppState> {
    Router::new()
        .route("/api/publishers", post(handlers::create_publisher))
        .route("/api/publishers/:id", get(handlers::get_publisher))
        .route(
            "/api/publishers/:id/contracts",
            get(handlers::get_publisher_contracts),
        )
}

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/stats", get(handlers::get_stats))
}

pub fn migration_routes() -> Router<AppState> {
    Router::new()
        // Database Migration Versioning and Rollback (Issue #252)
        .route(
            "/api/admin/migrations/status",
            get(migration_handlers::get_migration_status),
        )
        .route(
            "/api/admin/migrations/register",
            post(migration_handlers::register_migration),
        )
        .route(
            "/api/admin/migrations/validate",
            get(migration_handlers::validate_migrations),
        )
        .route(
            "/api/admin/migrations/lock",
            get(migration_handlers::get_lock_status),
        )
        .route(
            "/api/admin/migrations/:version",
            get(migration_handlers::get_migration_version),
        )
        .route(
            "/api/admin/migrations/:version/rollback",
            post(migration_handlers::rollback_migration),
        )
}

pub fn compatibility_dashboard_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/compatibility-dashboard",
            get(compatibility_testing_handlers::get_compatibility_dashboard),
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
