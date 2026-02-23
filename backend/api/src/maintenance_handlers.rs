use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use shared::models::{MaintenanceStatusResponse, MaintenanceWindow, StartMaintenanceRequest};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn start_maintenance(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<StartMaintenanceRequest>,
) -> ApiResult<Json<MaintenanceWindow>> {
    let window = sqlx::query_as::<_, MaintenanceWindow>(
        r#"
        WITH updated AS (
            UPDATE contracts SET is_maintenance = true WHERE id = $1 RETURNING publisher_id
        )
        INSERT INTO maintenance_windows (contract_id, message, scheduled_end_at, created_by)
        SELECT $1, $2, $3, publisher_id FROM updated
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&req.message)
    .bind(req.scheduled_end_at)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to start maintenance: {}", e)))?;

    Ok(Json(window))
}

pub async fn end_maintenance(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    sqlx::query(
        r#"
        UPDATE contracts SET is_maintenance = false WHERE id = $1;
        UPDATE maintenance_windows SET ended_at = $2 
        WHERE contract_id = $1 AND ended_at IS NULL
        "#,
    )
    .bind(contract_id)
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to end maintenance: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_maintenance_status(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<MaintenanceStatusResponse>> {
    let contract = sqlx::query_as::<_, (bool,)>(
        "SELECT is_maintenance FROM contracts WHERE id = $1"
    )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    let current_window = if contract.0 {
        sqlx::query_as::<_, MaintenanceWindow>(
            "SELECT * FROM maintenance_windows WHERE contract_id = $1 AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    } else {
        None
    };

    Ok(Json(MaintenanceStatusResponse {
        is_maintenance: contract.0,
        current_window,
    }))
}

pub async fn get_maintenance_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<MaintenanceWindow>>> {
    let windows = sqlx::query_as::<_, MaintenanceWindow>(
        "SELECT * FROM maintenance_windows WHERE contract_id = $1 ORDER BY started_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(windows))
}
