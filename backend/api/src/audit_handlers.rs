// api/src/audit_handlers.rs
// Axum handlers for the security audit system.

use axum::{
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::models::{
    AuditCheckRow, AuditRecord, AuditResponse, CategoryScore, CheckStatus, CheckWithStatus,
    ContractSecuritySummary, CreateAuditRequest, DetectionMethod, ExportRequest,
    UpdateCheckRequest,
};
use crate::{
    checklist::all_checks,
    detector::detect_all,
    error::{ApiError, ApiResult},
    models::{
        AuditCheckRow, AuditRecord, AuditResponse, CheckStatus, CheckWithStatus, ChecklistItem,
        ContractSecuritySummary, CreateAuditRequest, DetectionMethod, ExportRequest,
        UpdateCheckRequest,
    },
    scoring::{build_markdown_report, calculate_scores, score_badge},
    state::AppState,
};

// ─────────────────────────────────────────────────────────
// GET /api/contracts/:id/security-audit
// ─────────────────────────────────────────────────────────
pub async fn get_security_audit(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<AuditResponse>> {
    let audit: AuditRecord = sqlx::query_as(
        "SELECT * FROM security_audits WHERE contract_id = $1 ORDER BY audit_date DESC LIMIT 1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::not_found("AuditNotFound", format!("No security audit found for contract: {}", contract_id)))?;

    build_audit_response(&state, audit).await
}

// ─────────────────────────────────────────────────────────
// GET /api/contracts/:id/security-audit/:audit_id
// ─────────────────────────────────────────────────────────
pub async fn get_security_audit_by_id(
    State(state): State<AppState>,
    Path((contract_id, audit_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<AuditResponse>> {
    let audit: AuditRecord =
        sqlx::query_as("SELECT * FROM security_audits WHERE id = $1 AND contract_id = $2")
            .bind(audit_id)
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| ApiError::not_found("AuditNotFound", format!("No audit found with ID: {}", audit_id)))?;

    build_audit_response(&state, audit).await
}

// ─────────────────────────────────────────────────────────
// GET /api/contracts/:id/security-audits
// ─────────────────────────────────────────────────────────
pub async fn list_security_audits(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<AuditRecord>>> {
    let audits: Vec<AuditRecord> = sqlx::query_as(
        "SELECT * FROM security_audits WHERE contract_id = $1 ORDER BY audit_date DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::db_error("Failed to fetch security audits"))?;

    Ok(Json(audits))
}

// ─────────────────────────────────────────────────────────
// POST /api/contracts/:id/security-audit
// ─────────────────────────────────────────────────────────
pub async fn create_security_audit(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreateAuditRequest>,
) -> ApiResult<Json<AuditResponse>> {
    // Verify contract exists
    let _: (Uuid,) = sqlx::query_as("SELECT id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::not_found("ContractNotFound", format!("No contract found with ID: {}", contract_id)))?;

    // Run auto-detection if source provided
    let auto_results = req
        .source_code
        .as_deref()
        .map(detect_all)
        .unwrap_or_default();

    // Create the audit record
    let audit: AuditRecord = sqlx::query_as(
        r#"INSERT INTO security_audits
               (contract_id, contract_source, auditor, audit_date, overall_score)
           VALUES ($1, $2, $3, NOW(), 0.0)
           RETURNING *"#,
    )
    .bind(contract_id)
    .bind(&req.source_code)
    .bind(&req.auditor)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::db_error("Failed to create security audit record"))?;

    // Seed all check rows
    let all = all_checks();
    for item in &all {
        let (status, evidence, auto_detected) = match auto_results.get(item.id) {
            Some(result) => (result.status.clone(), result.evidence.clone(), true),
            None => (CheckStatus::Pending, None, false),
        };

        sqlx::query(
            r#"INSERT INTO audit_checks
                   (audit_id, check_id, status, auto_detected, evidence)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(audit.id)
        .bind(&item.id)
        .bind(&status)
        .bind(auto_detected)
        .bind(&evidence)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to seed audit check rows"))?;
    }

    // Calculate and persist initial score
    let checks = fetch_check_rows(&state, audit.id).await?;
    let (score, _) = calculate_scores(&checks);
    sqlx::query("UPDATE security_audits SET overall_score = $1 WHERE id = $2")
        .bind(score)
        .bind(audit.id)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to update audit score"))?;

    let audit: AuditRecord = sqlx::query_as("SELECT * FROM security_audits WHERE id = $1")
        .bind(audit.id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to reload audit record"))?;

    tracing::info!(
        audit_id = %audit.id,
        contract_id = %contract_id,
        auto_detected = auto_results.len(),
        "New security audit created"
    );

    build_audit_response(&state, audit).await
}

// ─────────────────────────────────────────────────────────
// PATCH /api/contracts/:id/security-audit/:audit_id/checks/:check_id
// ─────────────────────────────────────────────────────────
pub async fn update_check(
    State(state): State<AppState>,
    Path((_contract_id, audit_id, check_id)): Path<(Uuid, Uuid, String)>,
    Json(req): Json<UpdateCheckRequest>,
) -> ApiResult<Json<AuditResponse>> {
    // Validate check_id exists in static checklist
    let all = all_checks();
    if !all.iter().any(|c| c.id == check_id) {
        return Err(ApiError::bad_request(
            "InvalidCheckId",
            format!("Check ID '{}' does not exist in the audit checklist", check_id),
        ));
    }

    let rows_affected = sqlx::query(
        r#"UPDATE audit_checks
           SET status = $1, notes = $2, updated_at = NOW()
           WHERE audit_id = $3 AND check_id = $4"#,
    )
    .bind(&req.status)
    .bind(&req.notes)
    .bind(audit_id)
    .bind(&check_id)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::db_error("Failed to update audit check"))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ApiError::not_found(
            "CheckNotFound",
            format!("No check found with ID '{}' for audit: {}", check_id, audit_id),
        ));
    }

    let checks = fetch_check_rows(&state, audit_id).await?;
    let (score, _) = calculate_scores(&checks);

    sqlx::query("UPDATE security_audits SET overall_score = $1, updated_at = NOW() WHERE id = $2")
        .bind(score)
        .bind(audit_id)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to update audit score"))?;

    let audit: AuditRecord = sqlx::query_as("SELECT * FROM security_audits WHERE id = $1")
        .bind(audit_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to reload audit record"))?;

    build_audit_response(&state, audit).await
}

// ─────────────────────────────────────────────────────────
// POST /api/contracts/:id/security-audit/:audit_id/run-autocheck
// ─────────────────────────────────────────────────────────
pub async fn run_autocheck(
    State(state): State<AppState>,
    Path((_contract_id, audit_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<AuditResponse>> {
    let audit: AuditRecord = sqlx::query_as("SELECT * FROM security_audits WHERE id = $1")
        .bind(audit_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::not_found("AuditNotFound", format!("No audit found with ID: {}", audit_id)))?;

    let source = audit.contract_source.as_deref().ok_or_else(|| {
        tracing::warn!(audit_id = %audit_id, "No source code stored for auto-check");
        ApiError::unprocessable(
            "NoSourceCode",
            "No source code is stored for this audit. Upload source code first.",
        )
    })?;

    let auto_results = detect_all(source);

    for (check_id, result) in &auto_results {
        sqlx::query(
            r#"UPDATE audit_checks
               SET status = $1, evidence = $2, auto_detected = true, updated_at = NOW()
               WHERE audit_id = $3 AND check_id = $4
                 AND (auto_detected = true OR status = 'pending')"#,
        )
        .bind(&result.status)
        .bind(&result.evidence)
        .bind(audit_id)
        .bind(check_id)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to update auto-check results"))?;
    }

    let checks = fetch_check_rows(&state, audit_id).await?;
    let (score, _) = calculate_scores(&checks);
    sqlx::query("UPDATE security_audits SET overall_score = $1, updated_at = NOW() WHERE id = $2")
        .bind(score)
        .bind(audit_id)
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to update audit score"))?;

    let audit: AuditRecord = sqlx::query_as("SELECT * FROM security_audits WHERE id = $1")
        .bind(audit_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to reload audit record"))?;

    tracing::info!(audit_id = %audit_id, checks = auto_results.len(), "Auto-check completed");

    build_audit_response(&state, audit).await
}

// ─────────────────────────────────────────────────────────
// GET /api/contracts/:id/security-audit/:audit_id/export
// ─────────────────────────────────────────────────────────
pub async fn export_audit_markdown(
    State(state): State<AppState>,
    Path((contract_id, audit_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<ExportRequest>,
) -> ApiResult<Response> {
    let audit: AuditRecord =
        sqlx::query_as("SELECT * FROM security_audits WHERE id = $1 AND contract_id = $2")
            .bind(audit_id)
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| ApiError::not_found("AuditNotFound", format!("No audit found with ID: {}", audit_id)))?;

    let (contract_name,): (String,) = sqlx::query_as("SELECT name FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::not_found("ContractNotFound", format!("No contract found with ID: {}", contract_id)))?;

    let checks = fetch_check_rows(&state, audit_id).await?;
    let (_, category_scores) = calculate_scores(&checks);

    let audit_date_str = audit.audit_date.format("%Y-%m-%d %H:%M UTC").to_string();
    let markdown = build_markdown_report(
        &contract_name,
        &contract_id.to_string(),
        &audit.auditor,
        &audit_date_str,
        audit.overall_score,
        &checks,
        &category_scores,
        params.include_descriptions,
        params.failures_only,
    );

    let filename = format!(
        "security-audit-{}-{}.md",
        contract_name.to_lowercase().replace(' ', "-"),
        audit.audit_date.format("%Y%m%d")
    );

    Ok((
        axum::http::StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        markdown,
    )
        .into_response())
}

// ─────────────────────────────────────────────────────────
// GET /api/contracts/:id/security-score
// ─────────────────────────────────────────────────────────
pub async fn get_security_score(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<ContractSecuritySummary>> {
    let summary: ContractSecuritySummary = sqlx::query_as(
        r#"SELECT
               id          AS audit_id,
               audit_date,
               auditor,
               overall_score,
               '' AS score_badge
           FROM security_audits
           WHERE contract_id = $1
           ORDER BY audit_date DESC
           LIMIT 1"#,
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::not_found("AuditNotFound", format!("No security audit found for contract: {}", contract_id)))?;

    Ok(Json(ContractSecuritySummary {
        score_badge: score_badge(summary.overall_score).to_string(),
        ..summary
    }))
}

// ─────────────────────────────────────────────────────────
// GET /api/security-audit/checklist
// ─────────────────────────────────────────────────────────
pub async fn get_checklist_definition() -> Json<serde_json::Value> {
    let checks = all_checks();
    let items: Vec<serde_json::Value> = checks
        .iter()
        .map(|c| {
            let (detection_type, auto_patterns): (&str, Vec<String>) = match &c.detection {
                DetectionMethod::Automatic { patterns } => ("automatic", patterns.clone()),
                DetectionMethod::SemiAutomatic { patterns } => ("semi_automatic", patterns.clone()),
                DetectionMethod::Manual => ("manual", vec![]),
            };
            serde_json::json!({
                "id": c.id,
                "category": c.category.to_string(),
                "title": c.title,
                "description": c.description,
                "severity": format!("{:?}", c.severity),
                "detection_type": detection_type,
                "auto_patterns": auto_patterns,
                "remediation": c.remediation,
                "references": c.references,
            })
        })
        .collect();

    Json(serde_json::json!({
        "total": items.len(),
        "items": items,
    }))
}

// ─────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────

async fn fetch_check_rows(
    state: &AppState,
    audit_id: Uuid,
) -> ApiResult<Vec<AuditCheckRow>> {
    sqlx::query_as("SELECT * FROM audit_checks WHERE audit_id = $1 ORDER BY check_id")
        .bind(audit_id)
        .fetch_all(&state.db)
        .await
        .map_err(|_| ApiError::db_error("Failed to fetch audit check rows"))
}

async fn build_audit_response(
    state: &AppState,
    audit: AuditRecord,
) -> ApiResult<Json<AuditResponse>> {
    let check_rows = fetch_check_rows(state, audit.id).await?;
    let (_, category_scores) = calculate_scores(&check_rows);

    let all = all_checks();
    let status_map: std::collections::HashMap<String, &AuditCheckRow> =
        check_rows.iter().map(|r| (r.check_id.clone(), r)).collect();

    let auto_detected_count = check_rows.iter().filter(|r| r.auto_detected).count();

    let checks_with_status: Vec<CheckWithStatus> = all
        .iter()
        .map(|item| {
            let row = status_map.get(item.id);
            let (detection_type, auto_patterns): (&'static str, Vec<String>) = match &item.detection {
                DetectionMethod::Automatic { patterns } => ("automatic", patterns.clone()),
                DetectionMethod::SemiAutomatic { patterns } => ("semi_automatic", patterns.clone()),
                DetectionMethod::Manual => ("manual", vec![]),
            };
            CheckWithStatus {
                id: item.id,
                category: item.category.to_string(),
                title: item.title,
                description: item.description,
                severity: format!("{:?}", item.severity),
                detection_type,
                auto_patterns,
                remediation: item.remediation,
                references: item.references.clone(),
                status: row.map(|r| r.status.clone()).unwrap_or_default(),
                notes: row.and_then(|r| r.notes.clone()),
                auto_detected: row.map(|r| r.auto_detected).unwrap_or(false),
                evidence: row.and_then(|r| r.evidence.clone()),
            }
        })
        .collect();

    Ok(Json(AuditResponse {
        audit,
        checks: checks_with_status,
        category_scores,
        auto_detected_count,
    }))
}
