use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use shared::models::{
    BackupRestoration, ContractBackup, CreateBackupRequest, RestoreBackupRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn create_backup(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreateBackupRequest>,
) -> ApiResult<Json<ContractBackup>> {
    let contract = sqlx::query!("SELECT * FROM contracts WHERE id = $1", contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    let backup_date = Utc::now().date_naive();
    
    let metadata = serde_json::json!({
        "name": contract.name,
        "description": contract.description,
        "network": contract.network,
        "category": contract.category,
        "tags": contract.tags,
    });

    let state_snapshot = if req.include_state {
        Some(serde_json::json!({"placeholder": "state data"}))
    } else {
        None
    };

    let backup = sqlx::query_as::<_, ContractBackup>(
        r#"
        INSERT INTO contract_backups 
        (contract_id, backup_date, wasm_hash, metadata, state_snapshot, storage_size_bytes, primary_region, backup_regions)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (contract_id, backup_date) DO UPDATE 
        SET wasm_hash = $3, metadata = $4, state_snapshot = $5
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(backup_date)
    .bind(&contract.wasm_hash)
    .bind(&metadata)
    .bind(&state_snapshot)
    .bind(1024i64) // Placeholder size
    .bind("us-east-1")
    .bind(vec!["us-west-2", "eu-west-1"])
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create backup: {}", e)))?;

    Ok(Json(backup))
}

pub async fn list_backups(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ContractBackup>>> {
    let backups = sqlx::query_as::<_, ContractBackup>(
        "SELECT * FROM contract_backups WHERE contract_id = $1 ORDER BY backup_date DESC LIMIT 30",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(backups))
}

pub async fn restore_backup(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<RestoreBackupRequest>,
) -> ApiResult<Json<BackupRestoration>> {
    let start = std::time::Instant::now();

    let backup_date = NaiveDate::parse_from_str(&req.backup_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("invalid_date", "Invalid date format"))?;

    let backup = sqlx::query_as::<_, ContractBackup>(
        "SELECT * FROM contract_backups WHERE contract_id = $1 AND backup_date = $2",
    )
    .bind(contract_id)
    .bind(backup_date)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("backup", "Backup not found"))?;

    // Simulate restoration
    let duration_ms = start.elapsed().as_millis() as i32;

    let contract = sqlx::query!("SELECT publisher_id FROM contracts WHERE id = $1", contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let restoration = sqlx::query_as::<_, BackupRestoration>(
        r#"
        INSERT INTO backup_restorations (backup_id, restored_by, restore_duration_ms, success)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(backup.id)
    .bind(contract.publisher_id)
    .bind(duration_ms)
    .bind(true)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to log restoration: {}", e)))?;

    Ok(Json(restoration))
}

pub async fn verify_backup(
    State(state): State<AppState>,
    Path((contract_id, backup_date)): Path<(Uuid, String)>,
) -> ApiResult<StatusCode> {
    let date = NaiveDate::parse_from_str(&backup_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("invalid_date", "Invalid date format"))?;

    sqlx::query(
        "UPDATE contract_backups SET verified = true WHERE contract_id = $1 AND backup_date = $2",
    )
    .bind(contract_id)
    .bind(date)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to verify backup: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_backup_stats(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let stats = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total_backups,
            COUNT(*) FILTER (WHERE verified = true) as verified_backups,
            SUM(storage_size_bytes) as total_size_bytes,
            MAX(backup_date) as latest_backup
        FROM contract_backups 
        WHERE contract_id = $1
        "#,
        contract_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "total_backups": stats.total_backups.unwrap_or(0),
        "verified_backups": stats.verified_backups.unwrap_or(0),
        "total_size_bytes": stats.total_size_bytes.unwrap_or(0),
        "latest_backup": stats.latest_backup,
    })))
}
