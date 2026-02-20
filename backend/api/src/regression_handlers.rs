// api/src/regression_handlers.rs
// HTTP handlers for regression testing endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    regression_engine::{RegressionEngine, RegressionStatistics, TestBaseline, TestRun, TestSuite},
    state::AppState,
};

// ─────────────────────────────────────────────────────────
// Request/Response types
// ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EstablishBaselineRequest {
    pub version: String,
    pub test_suite_name: String,
    pub function_name: String,
    pub output: serde_json::Value,
    pub established_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RunTestRequest {
    pub version: String,
    pub test_suite_name: String,
    pub function_name: String,
    pub triggered_by: String,
    pub deployment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RunSuiteRequest {
    pub version: String,
    pub suite_name: String,
    pub triggered_by: String,
    pub deployment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTestSuiteRequest {
    pub name: String,
    pub description: Option<String>,
    pub test_functions: serde_json::Value,
    pub performance_thresholds: Option<serde_json::Value>,
    pub auto_run_on_deploy: Option<bool>,
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatisticsQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TestRunSummary {
    pub total_runs: usize,
    pub passed: usize,
    pub failed: usize,
    pub regressions_detected: usize,
    pub runs: Vec<TestRun>,
}

#[derive(Debug, Serialize)]
pub struct RegressionAlert {
    pub id: Uuid,
    pub test_run_id: Uuid,
    pub contract_id: Uuid,
    pub severity: String,
    pub alert_type: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
    pub resolved: bool,
}

// ─────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────

/// POST /api/contracts/:id/regression/baseline
/// Establish a new baseline for regression testing
pub async fn establish_baseline(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Json(req): Json<EstablishBaselineRequest>,
) -> ApiResult<Json<TestBaseline>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    // Verify contract exists
    let _: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    let engine = RegressionEngine::new(state.db.clone());

    let baseline = engine
        .establish_baseline(
            contract_uuid,
            req.version,
            req.test_suite_name,
            req.function_name,
            req.output,
            req.established_by,
        )
        .await
        .map_err(|e| ApiError::internal_server_error("BaselineCreationFailed", e.to_string()))?;

    Ok(Json(baseline))
}

/// POST /api/contracts/:id/regression/test
/// Run a single regression test
pub async fn run_regression_test(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Json(req): Json<RunTestRequest>,
) -> ApiResult<Json<TestRun>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let deployment_uuid = if let Some(ref dep_id) = req.deployment_id {
        Some(Uuid::parse_str(dep_id).map_err(|_| {
            ApiError::bad_request("InvalidDeploymentId", "Invalid deployment ID format")
        })?)
    } else {
        None
    };

    let engine = RegressionEngine::new(state.db.clone());

    let test_run = engine
        .run_regression_test(
            contract_uuid,
            req.version,
            req.test_suite_name,
            req.function_name,
            req.triggered_by,
            deployment_uuid,
        )
        .await
        .map_err(|e| ApiError::internal_server_error("TestExecutionFailed", e.to_string()))?;

    Ok(Json(test_run))
}

/// POST /api/contracts/:id/regression/suite
/// Run all tests in a suite
pub async fn run_test_suite(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Json(req): Json<RunSuiteRequest>,
) -> ApiResult<Json<TestRunSummary>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let deployment_uuid = if let Some(ref dep_id) = req.deployment_id {
        Some(Uuid::parse_str(dep_id).map_err(|_| {
            ApiError::bad_request("InvalidDeploymentId", "Invalid deployment ID format")
        })?)
    } else {
        None
    };

    let engine = RegressionEngine::new(state.db.clone());

    let runs = engine
        .run_test_suite(
            contract_uuid,
            req.version,
            req.suite_name,
            req.triggered_by,
            deployment_uuid,
        )
        .await
        .map_err(|e| ApiError::internal_server_error("SuiteExecutionFailed", e.to_string()))?;

    let total_runs = runs.len();
    let passed = runs.iter().filter(|r| matches!(r.status, crate::regression_engine::TestStatus::Passed)).count();
    let failed = runs.iter().filter(|r| matches!(r.status, crate::regression_engine::TestStatus::Failed)).count();
    let regressions_detected = runs.iter().filter(|r| r.regression_detected).count();

    Ok(Json(TestRunSummary {
        total_runs,
        passed,
        failed,
        regressions_detected,
        runs,
    }))
}

/// GET /api/contracts/:id/regression/runs
/// Get regression test run history
pub async fn get_test_runs(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<Vec<TestRun>>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let runs: Vec<TestRun> = sqlx::query_as(
        r#"SELECT 
            id, contract_id, version, baseline_id, test_suite_name,
            function_name, status as "status: _", execution_time_ms,
            memory_bytes, output_data, output_hash, output_matches_baseline,
            regression_detected, regression_severity as "regression_severity: _",
            performance_degradation_percent, started_at, completed_at,
            error_message, triggered_by
        FROM regression_test_runs
        WHERE contract_id = $1
        ORDER BY started_at DESC
        LIMIT 100"#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    Ok(Json(runs))
}

/// GET /api/contracts/:id/regression/baselines
/// Get active baselines for a contract
pub async fn get_baselines(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<Vec<TestBaseline>>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let baselines: Vec<TestBaseline> = sqlx::query_as(
        r#"SELECT 
            id, contract_id, version, test_suite_name, function_name,
            baseline_execution_time_ms, baseline_memory_bytes,
            baseline_cpu_instructions, output_snapshot, output_hash,
            established_at
        FROM regression_test_baselines
        WHERE contract_id = $1 AND is_active = TRUE
        ORDER BY established_at DESC"#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    Ok(Json(baselines))
}

/// GET /api/contracts/:id/regression/alerts
/// Get unresolved regression alerts
pub async fn get_alerts(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<Vec<RegressionAlert>>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let alerts: Vec<RegressionAlert> = sqlx::query_as(
        r#"SELECT 
            id, test_run_id, contract_id, 
            severity::text as severity,
            alert_type, message, details,
            triggered_at, acknowledged, resolved
        FROM regression_alerts
        WHERE contract_id = $1 AND resolved = FALSE
        ORDER BY triggered_at DESC"#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    Ok(Json(alerts))
}

/// POST /api/contracts/:id/regression/alerts/:alert_id/acknowledge
/// Acknowledge a regression alert
pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Path((contract_id, alert_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let alert_uuid = Uuid::parse_str(&alert_id).map_err(|_| {
        ApiError::bad_request("InvalidAlertId", "Invalid alert ID format")
    })?;

    let acknowledged_by = body
        .get("acknowledged_by")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    sqlx::query(
        "UPDATE regression_alerts 
         SET acknowledged = TRUE, acknowledged_at = NOW(), acknowledged_by = $1
         WHERE id = $2",
    )
    .bind(acknowledged_by)
    .bind(alert_uuid)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "alert_id": alert_id
    })))
}

/// POST /api/contracts/:id/regression/alerts/:alert_id/resolve
/// Resolve a regression alert
pub async fn resolve_alert(
    State(state): State<AppState>,
    Path((contract_id, alert_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let alert_uuid = Uuid::parse_str(&alert_id).map_err(|_| {
        ApiError::bad_request("InvalidAlertId", "Invalid alert ID format")
    })?;

    let resolution_notes = body
        .get("resolution_notes")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    sqlx::query(
        "UPDATE regression_alerts 
         SET resolved = TRUE, resolved_at = NOW(), resolution_notes = $1
         WHERE id = $2",
    )
    .bind(resolution_notes)
    .bind(alert_uuid)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "alert_id": alert_id
    })))
}

/// GET /api/contracts/:id/regression/statistics
/// Get regression testing statistics
pub async fn get_statistics(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(query): Query<StatisticsQuery>,
) -> ApiResult<Json<RegressionStatistics>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let days = query.days.unwrap_or(30);
    let period_end = Utc::now();
    let period_start = period_end - Duration::days(days);

    let engine = RegressionEngine::new(state.db.clone());

    let stats = engine
        .get_statistics(contract_uuid, period_start, period_end)
        .await
        .map_err(|e| ApiError::internal_server_error("StatisticsCalculationFailed", e.to_string()))?;

    Ok(Json(stats))
}

/// POST /api/contracts/:id/regression/suites
/// Create a new test suite
pub async fn create_test_suite(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Json(req): Json<CreateTestSuiteRequest>,
) -> ApiResult<Json<TestSuite>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let suite: TestSuite = sqlx::query_as(
        r#"INSERT INTO regression_test_suites (
            contract_id, name, description, test_functions,
            performance_thresholds, auto_run_on_deploy, created_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, contract_id, name, description, test_functions,
                  performance_thresholds, auto_run_on_deploy"#,
    )
    .bind(contract_uuid)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.test_functions)
    .bind(&req.performance_thresholds)
    .bind(req.auto_run_on_deploy.unwrap_or(true))
    .bind(&req.created_by)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("SuiteCreationFailed", e.to_string()))?;

    Ok(Json(suite))
}

/// GET /api/contracts/:id/regression/suites
/// Get test suites for a contract
pub async fn get_test_suites(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<Vec<TestSuite>>> {
    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request("InvalidContractId", "Invalid contract ID format")
    })?;

    let suites: Vec<TestSuite> = sqlx::query_as(
        r#"SELECT id, contract_id, name, description, test_functions,
                  performance_thresholds, auto_run_on_deploy
        FROM regression_test_suites
        WHERE contract_id = $1 AND is_active = TRUE
        ORDER BY name"#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_server_error("DatabaseError", e.to_string()))?;

    Ok(Json(suites))
}
