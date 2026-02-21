// src/audit_routes.rs
// Security audit route definitions.
// Merge these into the main Axum router alongside existing routes.

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{audit_handlers, state::AppState};

/// All security audit routes.
///
/// Mount alongside existing contract_routes() and publisher_routes():
///
/// ```rust
/// let app = Router::new()
///     .merge(routes::contract_routes())
///     .merge(routes::publisher_routes())
///     .merge(routes::health_routes())
///     .merge(audit_routes::security_audit_routes())  // ← add this
///     .layer(CorsLayer::permissive())
///     .with_state(state);
/// ```
pub fn security_audit_routes() -> Router<AppState> {
    Router::new()
        // ── Static checklist definition (no auth required) ─────────────────
        .route(
            "/api/security-audit/checklist",
            get(audit_handlers::get_checklist_definition),
        )
        // ── Per-contract audit endpoints ───────────────────────────────────
        // Get security score summary (for contract card badge)
        .route(
            "/api/contracts/:id/security-score",
            get(audit_handlers::get_security_score),
        )
        // List all historical audits for a contract
        .route(
            "/api/contracts/:id/security-audits",
            get(audit_handlers::list_security_audits),
        )
        // Get latest audit / Create new audit
        .route(
            "/api/contracts/:id/security-audit",
            get(audit_handlers::get_security_audit).post(audit_handlers::create_security_audit),
        )
        // Get specific historical audit
        .route(
            "/api/contracts/:id/security-audit/:audit_id",
            get(audit_handlers::get_security_audit_by_id),
        )
        // Update a single check status (auditor interaction)
        .route(
            "/api/contracts/:id/security-audit/:audit_id/checks/:check_id",
            patch(audit_handlers::update_check),
        )
        // Re-run source-code auto-detection on an existing audit
        .route(
            "/api/contracts/:id/security-audit/:audit_id/run-autocheck",
            post(audit_handlers::run_autocheck),
        )
        // Export audit as Markdown download
        .route(
            "/api/contracts/:id/security-audit/:audit_id/export",
            get(audit_handlers::export_audit_markdown),
        )
}
