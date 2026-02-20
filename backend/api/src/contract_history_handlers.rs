// api/src/contract_history_handlers.rs
//
// Audit-log and version-history endpoints for the Soroban Registry.
//
// Routes (all registered in contract_history_routes.rs):
//   GET  /api/contracts/:id/history              – last 10 log entries (sidebar)
//   GET  /api/contracts/:id/history/all          – paginated full history
//   GET  /api/contracts/:id/history/export       – CSV download
//   GET  /api/contracts/:id/versions/:v1/diff/:v2 – field-level diff
//   POST /api/contracts/:id/rollback/:snapshot_id – admin rollback

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};
use shared::{
    AuditActionType, AuditLogPage, ContractAuditLog, ContractSnapshot, FieldChange, RollbackRequest,
    VersionDiff,
};

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/:id/history
// Returns the 10 most recent audit log entries for the history sidebar.
// ─────────────────────────────────────────────────────────────────────────────
pub async fn get_contract_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ContractAuditLog>>> {
    verify_contract_exists(&state, contract_id).await?;

    let entries: Vec<ContractAuditLog> = sqlx::query_as(
        "SELECT id, contract_id, action_type, old_value, new_value, changed_by, timestamp
           FROM contract_audit_log
          WHERE contract_id = $1
          ORDER BY timestamp DESC
          LIMIT 10",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list recent audit log", e))?;

    Ok(Json(entries))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/:id/history/all?page=1&limit=20
// Full paginated history.
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 20 }

pub async fn get_full_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<AuditLogPage>> {
    if params.page < 1 || params.limit < 1 || params.limit > 100 {
        return Err(ApiError::bad_request(
            "InvalidPagination",
            "page >= 1 and 1 <= limit <= 100",
        ));
    }

    verify_contract_exists(&state, contract_id).await?;

    let offset = (params.page - 1) * params.limit;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_audit_log WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("count audit log", e))?;

    let items: Vec<ContractAuditLog> = sqlx::query_as(
        "SELECT id, contract_id, action_type, old_value, new_value, changed_by, timestamp
           FROM contract_audit_log
          WHERE contract_id = $1
          ORDER BY timestamp DESC
          LIMIT $2 OFFSET $3",
    )
    .bind(contract_id)
    .bind(params.limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list audit log page", e))?;

    let total_pages = if params.limit > 0 {
        (total as f64 / params.limit as f64).ceil() as i64
    } else {
        0
    };

    Ok(Json(AuditLogPage {
        items,
        total,
        page: params.page,
        total_pages,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/:id/history/export
// Streams audit log as CSV for compliance export.
// ─────────────────────────────────────────────────────────────────────────────
pub async fn export_history_csv(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    verify_contract_exists(&state, contract_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let entries: Vec<ContractAuditLog> = sqlx::query_as(
        "SELECT id, contract_id, action_type, old_value, new_value, changed_by, timestamp
           FROM contract_audit_log
          WHERE contract_id = $1
          ORDER BY timestamp ASC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut csv = String::from("id,contract_id,action_type,old_value,new_value,changed_by,timestamp\n");

    for entry in &entries {
        let old = entry
            .old_value
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default()
            .replace('"', "\"\"");
        let new = entry
            .new_value
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default()
            .replace('"', "\"\"");

        csv.push_str(&format!(
            "{},{},{},\"{}\",\"{}\",{},{}\n",
            entry.id,
            entry.contract_id,
            entry.action_type,
            old,
            new,
            entry.changed_by,
            entry.timestamp.to_rfc3339(),
        ));
    }

    let filename = format!("audit-{}.csv", contract_id);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        csv,
    )
        .into_response())
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/:id/versions/:v1/diff/:v2
// Computes a field-level diff between two snapshots.
// ─────────────────────────────────────────────────────────────────────────────
pub async fn diff_versions(
    State(state): State<AppState>,
    Path((contract_id, v1, v2)): Path<(Uuid, i32, i32)>,
) -> ApiResult<Json<VersionDiff>> {
    verify_contract_exists(&state, contract_id).await?;

    let snap_a: ContractSnapshot = sqlx::query_as(
        "SELECT id, contract_id, version_number, snapshot_data, audit_log_id, created_at
           FROM contract_snapshots
          WHERE contract_id = $1 AND version_number = $2",
    )
    .bind(contract_id)
    .bind(v1)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "SnapshotNotFound",
            format!("No snapshot found for version {v1}"),
        ),
        _ => db_err("fetch snapshot v1", err),
    })?;

    let snap_b: ContractSnapshot = sqlx::query_as(
        "SELECT id, contract_id, version_number, snapshot_data, audit_log_id, created_at
           FROM contract_snapshots
          WHERE contract_id = $1 AND version_number = $2",
    )
    .bind(contract_id)
    .bind(v2)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "SnapshotNotFound",
            format!("No snapshot found for version {v2}"),
        ),
        _ => db_err("fetch snapshot v2", err),
    })?;

    let diff = compute_diff(contract_id, v1, v2, &snap_a.snapshot_data, &snap_b.snapshot_data);
    Ok(Json(diff))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/contracts/:id/rollback/:snapshot_id
// Admin-only: restores contract to a previous snapshot.
// Creates a new audit log entry and a new snapshot for the rolled-back state.
// ─────────────────────────────────────────────────────────────────────────────
pub async fn rollback_contract(
    State(state): State<AppState>,
    Path((contract_id, snapshot_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<RollbackRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // 1. Load the target snapshot
    let snapshot: ContractSnapshot = sqlx::query_as(
        "SELECT id, contract_id, version_number, snapshot_data, audit_log_id, created_at
           FROM contract_snapshots
          WHERE id = $1 AND contract_id = $2",
    )
    .bind(snapshot_id)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "SnapshotNotFound",
            format!("No snapshot found with id {snapshot_id} for contract {contract_id}"),
        ),
        _ => db_err("fetch rollback snapshot", err),
    })?;

    // 2. Read the current contract state (for old_value in the audit log)
    let current_data: serde_json::Value = sqlx::query_scalar(
        "SELECT row_to_json(contracts.*) FROM contracts WHERE id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("read current contract for rollback", e))?;

    // 3. Begin transaction
    let mut tx = state.db.begin().await.map_err(|e| db_err("begin rollback tx", e))?;

    // 4. Extract fields from the snapshot and apply them back
    let snap = &snapshot.snapshot_data;
    sqlx::query(
        "UPDATE contracts
            SET name        = $2,
                description = $3,
                wasm_hash   = $4,
                category    = $5,
                tags        = $6,
                is_verified = $7,
                updated_at  = NOW()
          WHERE id = $1",
    )
    .bind(contract_id)
    .bind(snap["name"].as_str().unwrap_or(""))
    .bind(snap["description"].as_str())
    .bind(snap["wasm_hash"].as_str().unwrap_or(""))
    .bind(snap["category"].as_str())
    .bind(
        snap["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default(),
    )
    .bind(snap["is_verified"].as_bool().unwrap_or(false))
    .execute(&mut *tx)
    .await
    .map_err(|e| db_err("apply rollback to contract", e))?;

    // 5. Write audit log entry
    let log_entry: ContractAuditLog = sqlx::query_as(
        "INSERT INTO contract_audit_log
               (contract_id, action_type, old_value, new_value, changed_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, contract_id, action_type, old_value, new_value, changed_by, timestamp",
    )
    .bind(contract_id)
    .bind(AuditActionType::Rollback)
    .bind(&current_data)
    .bind(&snapshot.snapshot_data)
    .bind(&req.changed_by)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_err("insert rollback audit log", e))?;

    // 6. Determine next version number and write new snapshot
    let next_ver: i32 = sqlx::query_scalar(
        "SELECT next_contract_version($1)",
    )
    .bind(contract_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_err("next version number", e))?;

    sqlx::query(
        "INSERT INTO contract_snapshots
               (contract_id, version_number, snapshot_data, audit_log_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(contract_id)
    .bind(next_ver)
    .bind(&snapshot.snapshot_data)
    .bind(log_entry.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| db_err("insert post-rollback snapshot", e))?;

    tx.commit().await.map_err(|e| db_err("commit rollback tx", e))?;

    tracing::info!(
        contract_id = %contract_id,
        target_snapshot = %snapshot_id,
        rolled_back_to_version = snapshot.version_number,
        new_version = next_ver,
        changed_by = %req.changed_by,
        "Contract rolled back successfully"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "contract_id": contract_id,
        "rolled_back_to_version": snapshot.version_number,
        "new_version": next_ver,
        "audit_log_id": log_entry.id,
    })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Insert one audit log entry + snapshot atomically.
/// Called from publish_contract and any future mutation hooks.
pub async fn log_contract_change(
    db: &sqlx::PgPool,
    contract_id: Uuid,
    action_type: AuditActionType,
    old_value: Option<serde_json::Value>,
    new_value: Option<serde_json::Value>,
    changed_by: &str,
) -> Result<Uuid, sqlx::Error> {
    let mut tx = db.begin().await?;

    // Insert audit log row
    let (log_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO contract_audit_log
               (contract_id, action_type, old_value, new_value, changed_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(contract_id)
    .bind(&action_type)
    .bind(&old_value)
    .bind(&new_value)
    .bind(changed_by)
    .fetch_one(&mut *tx)
    .await?;

    // If we have a new_value, persist a snapshot
    if let Some(ref snap_data) = new_value {
        let next_ver: i32 =
            sqlx::query_scalar("SELECT next_contract_version($1)")
                .bind(contract_id)
                .fetch_one(&mut *tx)
                .await?;

        sqlx::query(
            "INSERT INTO contract_snapshots
                   (contract_id, version_number, snapshot_data, audit_log_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(contract_id)
        .bind(next_ver)
        .bind(snap_data)
        .bind(log_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(log_id)
}

/// Compute a field-level diff between two JSONB objects.
fn compute_diff(
    contract_id: Uuid,
    from_version: i32,
    to_version: i32,
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> VersionDiff {
    let mut added   = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    let a_obj = a.as_object().cloned().unwrap_or_default();
    let b_obj = b.as_object().cloned().unwrap_or_default();

    // Fields in b but not in a → added
    for (key, val) in &b_obj {
        if !a_obj.contains_key(key) {
            added.push(FieldChange {
                field: key.clone(),
                from: serde_json::Value::Null,
                to: val.clone(),
            });
        }
    }

    // Fields in a but not in b → removed
    for (key, val) in &a_obj {
        if !b_obj.contains_key(key) {
            removed.push(FieldChange {
                field: key.clone(),
                from: val.clone(),
                to: serde_json::Value::Null,
            });
        }
    }

    // Fields in both but different → modified
    for (key, a_val) in &a_obj {
        if let Some(b_val) = b_obj.get(key) {
            if a_val != b_val {
                modified.push(FieldChange {
                    field: key.clone(),
                    from: a_val.clone(),
                    to: b_val.clone(),
                });
            }
        }
    }

    VersionDiff {
        contract_id,
        from_version,
        to_version,
        added,
        removed,
        modified,
    }
}

/// Verify a contract row exists; returns 404 error if not.
async fn verify_contract_exists(state: &AppState, contract_id: Uuid) -> ApiResult<()> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("check contract exists", e))
        .and_then(|count| {
            if count == 0 {
                Err(ApiError::not_found(
                    "ContractNotFound",
                    format!("No contract found with ID: {contract_id}"),
                ))
            } else {
                Ok(())
            }
        })
}

fn db_err(op: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = op, error = ?err, "database error");
    ApiError::internal("An unexpected database error occurred")
}
