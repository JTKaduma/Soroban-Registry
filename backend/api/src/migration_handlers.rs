// migration_handlers.rs
// Database migration versioning, rollback, and validation handlers (Issue #252).
// Provides schema version tracking, checksum validation, advisory locking,
// and rollback capability for safe database deployments.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ─────────────────────────────────────────────────────────
// Models
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SchemaVersion {
    pub id: i32,
    pub version: i32,
    pub description: String,
    pub filename: String,
    pub checksum: String,
    pub applied_at: DateTime<Utc>,
    pub applied_by: String,
    pub execution_time_ms: Option<i32>,
    pub rolled_back_at: Option<DateTime<Utc>>,
    pub rollback_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SchemaRollbackScript {
    pub id: i32,
    pub version: i32,
    pub down_sql: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MigrationStatusResponse {
    pub current_version: Option<i32>,
    pub total_applied: i64,
    pub total_rolled_back: i64,
    pub pending_count: i64,
    pub versions: Vec<SchemaVersion>,
    pub has_lock: bool,
    pub healthy: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MigrationValidationResponse {
    pub valid: bool,
    pub mismatches: Vec<ChecksumMismatch>,
    pub missing: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct ChecksumMismatch {
    pub version: i32,
    pub filename: String,
    pub expected_checksum: String,
    pub actual_checksum: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterMigrationRequest {
    pub version: i32,
    pub description: String,
    pub filename: String,
    pub sql_content: String,
    pub down_sql: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterMigrationResponse {
    pub version: i32,
    pub checksum: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RollbackResponse {
    pub version: i32,
    pub rolled_back_at: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LockStatusResponse {
    pub locked: bool,
    pub locked_by: Option<String>,
    pub locked_at: Option<DateTime<Utc>>,
}

// ─────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────

/// Compute SHA-256 hex checksum of SQL content.
pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Advisory lock key for migration operations (arbitrary fixed i64).
const MIGRATION_ADVISORY_LOCK_KEY: i64 = 252_252_252;

/// Try to acquire the PostgreSQL advisory lock. Returns true if acquired.
async fn try_acquire_lock(pool: &sqlx::PgPool) -> Result<bool, sqlx::Error> {
    let acquired: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(MIGRATION_ADVISORY_LOCK_KEY)
            .fetch_one(pool)
            .await?;
    Ok(acquired)
}

/// Release the PostgreSQL advisory lock.
async fn release_lock(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_ADVISORY_LOCK_KEY)
        .execute(pool)
        .await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────

/// GET /api/admin/migrations/status
///
/// Returns the current migration status: applied versions, pending count,
/// lock state, and any warnings about checksum mismatches.
pub async fn get_migration_status(
    State(state): State<AppState>,
) -> ApiResult<Json<MigrationStatusResponse>> {
    let versions: Vec<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        ORDER BY version ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let current_version = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_none())
        .map(|v| v.version)
        .max();

    let total_applied = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_none())
        .count() as i64;

    let total_rolled_back = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_some())
        .count() as i64;

    // Check advisory lock status
    let has_lock: bool = sqlx::query_scalar(
        "SELECT NOT pg_try_advisory_lock($1) OR pg_advisory_unlock($1)",
    )
    .bind(MIGRATION_ADVISORY_LOCK_KEY)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let mut warnings = Vec::new();

    // Check for gaps in version numbers
    let active_versions: Vec<i32> = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_none())
        .map(|v| v.version)
        .collect();

    if let (Some(&min), Some(&max)) = (active_versions.first(), active_versions.last()) {
        for expected in min..=max {
            if !active_versions.contains(&expected) {
                warnings.push(format!("Gap detected: version {} is missing", expected));
            }
        }
    }

    let healthy = warnings.is_empty();

    Ok(Json(MigrationStatusResponse {
        current_version,
        total_applied,
        total_rolled_back,
        pending_count: 0, // Pending is determined by comparing filesystem vs DB
        versions,
        has_lock,
        healthy,
        warnings,
    }))
}

/// POST /api/admin/migrations/register
///
/// Register a new migration with its SQL content and optional rollback script.
/// Computes SHA-256 checksum and stores it for future validation.
/// Uses advisory lock to prevent concurrent registration.
pub async fn register_migration(
    State(state): State<AppState>,
    Json(body): Json<RegisterMigrationRequest>,
) -> ApiResult<Json<RegisterMigrationResponse>> {
    // Acquire advisory lock
    let acquired = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if !acquired {
        return Err(ApiError::conflict(
            "MigrationLocked",
            "Another migration operation is in progress. Please try again later.",
        ));
    }

    // Ensure we release the lock on all exit paths
    let result = register_migration_inner(&state, &body).await;

    let _ = release_lock(&state.db).await;

    result
}

async fn register_migration_inner(
    state: &AppState,
    body: &RegisterMigrationRequest,
) -> ApiResult<Json<RegisterMigrationResponse>> {
    // Check if version already exists
    let exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM schema_versions WHERE version = $1",
    )
    .bind(body.version)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if exists {
        return Err(ApiError::conflict(
            "VersionExists",
            format!("Migration version {} is already registered", body.version),
        ));
    }

    let checksum = compute_checksum(&body.sql_content);
    let start = std::time::Instant::now();

    // Register the migration
    sqlx::query(
        r#"
        INSERT INTO schema_versions (version, description, filename, checksum, execution_time_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(body.version)
    .bind(&body.description)
    .bind(&body.filename)
    .bind(&checksum)
    .bind(start.elapsed().as_millis() as i32)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    // Register rollback script if provided
    if let Some(down_sql) = &body.down_sql {
        let down_checksum = compute_checksum(down_sql);
        sqlx::query(
            r#"
            INSERT INTO schema_rollback_scripts (version, down_sql, checksum)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(body.version)
        .bind(down_sql)
        .bind(&down_checksum)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;
    }

    Ok(Json(RegisterMigrationResponse {
        version: body.version,
        checksum,
        message: format!("Migration version {} registered successfully", body.version),
    }))
}

/// POST /api/admin/migrations/:version/rollback
///
/// Roll back a specific migration version by executing its DOWN script.
/// Uses advisory lock to prevent concurrent operations.
pub async fn rollback_migration(
    State(state): State<AppState>,
    Path(version): Path<i32>,
) -> ApiResult<Json<RollbackResponse>> {
    // Acquire advisory lock
    let acquired = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if !acquired {
        return Err(ApiError::conflict(
            "MigrationLocked",
            "Another migration operation is in progress. Please try again later.",
        ));
    }

    let result = rollback_migration_inner(&state, version).await;

    let _ = release_lock(&state.db).await;

    result
}

async fn rollback_migration_inner(
    state: &AppState,
    version: i32,
) -> ApiResult<Json<RollbackResponse>> {
    // Verify migration exists and is not already rolled back
    let migration: Option<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        WHERE version = $1
        "#,
    )
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let migration = migration.ok_or_else(|| {
        ApiError::not_found("NotFound", format!("Migration version {} not found", version))
    })?;

    if migration.rolled_back_at.is_some() {
        return Err(ApiError::conflict(
            "AlreadyRolledBack",
            format!("Migration version {} has already been rolled back", version),
        ));
    }

    // Get the rollback script
    let rollback: Option<SchemaRollbackScript> = sqlx::query_as(
        r#"
        SELECT id, version, down_sql, checksum, created_at
        FROM schema_rollback_scripts
        WHERE version = $1
        "#,
    )
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let rollback = rollback.ok_or_else(|| {
        ApiError::not_found(
            "NoRollbackScript",
            format!("No rollback script found for migration version {}", version),
        )
    })?;

    // Validate rollback script checksum
    let actual_checksum = compute_checksum(&rollback.down_sql);
    if actual_checksum != rollback.checksum {
        return Err(ApiError::conflict(
            "ChecksumMismatch",
            format!(
                "Rollback script checksum mismatch for version {}. Expected: {}, Got: {}",
                version, rollback.checksum, actual_checksum
            ),
        ));
    }

    // Execute the rollback SQL
    sqlx::query(&rollback.down_sql)
        .execute(&state.db)
        .await
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to execute rollback for version {}: {}",
                version, e
            ))
        })?;

    // Mark migration as rolled back
    let now = Utc::now();
    sqlx::query(
        "UPDATE schema_versions SET rolled_back_at = $1, rollback_by = current_user WHERE version = $2",
    )
    .bind(now)
    .bind(version)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(RollbackResponse {
        version,
        rolled_back_at: now,
        message: format!("Migration version {} rolled back successfully", version),
    }))
}

/// GET /api/admin/migrations/validate
///
/// Validate all applied migrations by recomputing checksums and checking
/// for mismatches (tampering detection).
pub async fn validate_migrations(
    State(state): State<AppState>,
) -> ApiResult<Json<MigrationValidationResponse>> {
    let versions: Vec<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        WHERE rolled_back_at IS NULL
        ORDER BY version ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    // Check for gaps
    let mut missing = Vec::new();
    if let (Some(first), Some(last)) = (versions.first(), versions.last()) {
        for v in first.version..=last.version {
            if !versions.iter().any(|sv| sv.version == v) {
                missing.push(v);
            }
        }
    }

    // Checksums are stored at registration time; we report the stored state.
    // In a full implementation, we'd re-read migration files from disk and compare.
    // Here we verify rollback script checksums are consistent.
    let mut mismatches = Vec::new();

    let rollback_scripts: Vec<SchemaRollbackScript> = sqlx::query_as(
        r#"
        SELECT id, version, down_sql, checksum, created_at
        FROM schema_rollback_scripts
        ORDER BY version ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    for script in &rollback_scripts {
        let actual = compute_checksum(&script.down_sql);
        if actual != script.checksum {
            mismatches.push(ChecksumMismatch {
                version: script.version,
                filename: format!("rollback_v{}", script.version),
                expected_checksum: script.checksum.clone(),
                actual_checksum: actual,
            });
        }
    }

    let valid = mismatches.is_empty() && missing.is_empty();

    Ok(Json(MigrationValidationResponse {
        valid,
        mismatches,
        missing,
    }))
}

/// GET /api/admin/migrations/:version
///
/// Get details for a specific migration version.
pub async fn get_migration_version(
    State(state): State<AppState>,
    Path(version): Path<i32>,
) -> ApiResult<Json<SchemaVersion>> {
    let migration: Option<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        WHERE version = $1
        "#,
    )
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    migration
        .map(Json)
        .ok_or_else(|| ApiError::not_found("NotFound", format!("Migration version {} not found", version)))
}

/// GET /api/admin/migrations/lock
///
/// Check the current advisory lock status.
pub async fn get_lock_status(
    State(state): State<AppState>,
) -> ApiResult<Json<LockStatusResponse>> {
    // Try to acquire and immediately release to check if lock is free
    let can_lock = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if can_lock {
        let _ = release_lock(&state.db).await;
    }

    let lock_row: Option<(Option<String>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT locked_by, locked_at FROM schema_migration_locks WHERE id = 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let (locked_by, locked_at) = lock_row.unwrap_or((None, None));

    Ok(Json(LockStatusResponse {
        locked: !can_lock,
        locked_by,
        locked_at,
    }))
}

/// Startup check: validates migration state and logs warnings.
/// Called during application startup before serving requests.
pub async fn check_migrations_on_startup(pool: &sqlx::PgPool) {
    // Check if schema_versions table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'schema_versions'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !table_exists {
        tracing::warn!(
            "schema_versions table not found. Migration versioning is not initialized."
        );
        return;
    }

    // Get current state
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM schema_versions WHERE rolled_back_at IS NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let current_version: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_versions WHERE rolled_back_at IS NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);

    tracing::info!(
        applied_migrations = count,
        current_version = ?current_version,
        "Migration versioning status"
    );

    // Check for rollback script integrity
    let mismatch_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM schema_rollback_scripts s
        WHERE s.checksum != encode(sha256(s.down_sql::bytea), 'hex')
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if mismatch_count > 0 {
        tracing::warn!(
            mismatch_count = mismatch_count,
            "Rollback script checksum mismatches detected! Migration integrity may be compromised."
        );
    }

    // Check for version gaps
    let versions: Vec<i32> = sqlx::query_scalar(
        "SELECT version FROM schema_versions WHERE rolled_back_at IS NULL ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if let (Some(&min), Some(&max)) = (versions.first(), versions.last()) {
        let expected_count = (max - min + 1) as usize;
        if versions.len() != expected_count {
            tracing::warn!(
                expected = expected_count,
                actual = versions.len(),
                "Version gaps detected in migration history"
            );
        }
    }
}
