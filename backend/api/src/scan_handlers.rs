use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::state::AppState;
use crate::scanner_service::{self, VulnerabilityPayload, ScanRequest};

pub async fn ingest_cves(
    State(state): State<AppState>,
    Json(payload): Json<Vec<VulnerabilityPayload>>,
) -> impl IntoResponse {
    match scanner_service::sync_cves(&state.pool, payload).await {
        Ok(count) => {
            let msg = format!("Ingested {} CVEs successfully", count);
            (StatusCode::OK, Json(msg)).into_response()
        }
        Err(e) => {
            let err = format!("Database error ingesting CVEs: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
        }
    }
}

pub async fn scan_contract(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(payload): Json<ScanRequest>,
) -> impl IntoResponse {
    match scanner_service::perform_scan(&state.pool, contract_id, payload).await {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => {
            let err = format!("Failed to run contract scan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
        }
    }
}

pub async fn get_scan_report(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    match scanner_service::get_history(&state.pool, contract_id).await {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => {
            let err = format!("Failed to retrieve scan history: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
        }
    }
}
