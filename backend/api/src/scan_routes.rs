use axum::{
    routing::{get, post},
    Router,
};
use crate::{scan_handlers, state::AppState};

pub fn scan_routes() -> Router<AppState> {
    Router::new()
        .route("/api/vulnerabilities/sync", post(scan_handlers::ingest_cves))
        .route("/api/contracts/:id/scan", post(scan_handlers::scan_contract))
        .route("/api/contracts/:id/scan", get(scan_handlers::get_scan_report))
}
