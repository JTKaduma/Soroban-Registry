// api/src/contract_history_routes.rs
// Audit-log and version-history route definitions.

use axum::{
    routing::{get, post},
    Router,
};

use crate::{contract_history_handlers, state::AppState};

/// Mount alongside existing routes in main.rs:
///
/// ```rust
/// let app = Router::new()
///     .merge(routes::contract_routes())
///     .merge(contract_history_routes::contract_history_routes())
///     ...
/// ```
pub fn contract_history_routes() -> Router<AppState> {
    Router::new()
        // History sidebar — last 10 changes
        .route(
            "/api/contracts/:id/history",
            get(contract_history_handlers::get_contract_history),
        )
        // Full paginated audit log
        .route(
            "/api/contracts/:id/history/all",
            get(contract_history_handlers::get_full_history),
        )
        // CSV export for compliance
        .route(
            "/api/contracts/:id/history/export",
            get(contract_history_handlers::export_history_csv),
        )
        // Version diff: compare any two snapshot versions
        .route(
            "/api/contracts/:id/versions/:v1/diff/:v2",
            get(contract_history_handlers::diff_versions),
        )
        // Admin rollback to a specific snapshot
        .route(
            "/api/contracts/:id/rollback/:snapshot_id",
            post(contract_history_handlers::rollback_contract),
        )
        // Verify entire audit log hash-chain
        .route(
            "/api/contracts/:id/history/verify",
            get(contract_history_handlers::verify_contract_history),
        )
}
