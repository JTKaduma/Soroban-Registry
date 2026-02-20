use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};
use shared::models::{
    CheckResidencyRequest, CreateResidencyPolicyRequest, ListResidencyLogsParams,
    ResidencyAuditLog, ResidencyDecision, ResidencyPolicy, ResidencyViolation,
    UpdateResidencyPolicyRequest,
};

fn db_err(ctx: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(context = ctx, error = %err, "database error");
    ApiError::internal(format!("Database error during: {}", ctx))
}

fn not_found(id: Uuid) -> ApiError {
    ApiError::not_found("PolicyNotFound", format!("No residency policy found with ID: {}", id))
}

async fn fetch_policy(state: &AppState, id: Uuid) -> ApiResult<ResidencyPolicy> {
    sqlx::query_as("SELECT * FROM residency_policies WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => not_found(id),
            _ => db_err("fetch policy", e),
        })
}

pub async fn create_policy(
    State(state): State<AppState>,
    Json(req): Json<CreateResidencyPolicyRequest>,
) -> ApiResult<(StatusCode, Json<ResidencyPolicy>)> {
    if req.contract_id.is_empty() {
        return Err(ApiError::bad_request("MissingContractId", "contract_id is required"));
    }
    if req.allowed_regions.is_empty() {
        return Err(ApiError::bad_request("MissingRegions", "allowed_regions must not be empty"));
    }
    if req.created_by.is_empty() {
        return Err(ApiError::bad_request("MissingCreatedBy", "created_by is required"));
    }

    let policy: ResidencyPolicy = sqlx::query_as(
        "INSERT INTO residency_policies (contract_id, allowed_regions, description, created_by)
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(&req.contract_id)
    .bind(&req.allowed_regions)
    .bind(&req.description)
    .bind(&req.created_by)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("create residency policy", e))?;

    tracing::info!(policy_id = %policy.id, contract_id = %policy.contract_id, "residency policy created");

    Ok((StatusCode::CREATED, Json(policy)))
}

pub async fn get_policy(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ResidencyPolicy>> {
    fetch_policy(&state, id).await.map(Json)
}

pub async fn list_policies(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ResidencyPolicy>>> {
    let policies: Vec<ResidencyPolicy> =
        sqlx::query_as("SELECT * FROM residency_policies WHERE is_active = TRUE ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_err("list residency policies", e))?;

    Ok(Json(policies))
}

pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateResidencyPolicyRequest>,
) -> ApiResult<Json<ResidencyPolicy>> {
    if let Some(ref regions) = req.allowed_regions {
        if regions.is_empty() {
            return Err(ApiError::bad_request("MissingRegions", "allowed_regions must not be empty"));
        }
    }

    let policy: ResidencyPolicy = sqlx::query_as(
        "UPDATE residency_policies
         SET allowed_regions = COALESCE($1, allowed_regions),
             description     = COALESCE($2, description),
             is_active       = COALESCE($3, is_active),
             updated_at      = NOW()
         WHERE id = $4
         RETURNING *",
    )
    .bind(&req.allowed_regions)
    .bind(&req.description)
    .bind(req.is_active)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => not_found(id),
        _ => db_err("update residency policy", e),
    })?;

    tracing::info!(policy_id = %id, "residency policy updated");

    Ok(Json(policy))
}

pub async fn check_residency(
    State(state): State<AppState>,
    Json(req): Json<CheckResidencyRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let policy = fetch_policy(&state, req.policy_id).await?;

    if !policy.is_active {
        return Err(ApiError::bad_request("PolicyInactive", "The referenced residency policy is not active"));
    }

    let is_allowed = policy.allowed_regions.iter().any(|r| r.eq_ignore_ascii_case(&req.requested_region));
    let decision = if is_allowed { ResidencyDecision::Allowed } else { ResidencyDecision::Denied };
    let reason = if is_allowed {
        format!("Region '{}' is permitted by policy", req.requested_region)
    } else {
        format!(
            "Region '{}' is not in the allowed list: {:?}",
            req.requested_region, policy.allowed_regions
        )
    };

    sqlx::query(
        "INSERT INTO residency_audit_logs
             (policy_id, contract_id, requested_region, decision, action, requested_by, reason)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(policy.id)
    .bind(&req.contract_id)
    .bind(&req.requested_region)
    .bind(&decision)
    .bind(&req.action)
    .bind(&req.requested_by)
    .bind(&reason)
    .execute(&state.db)
    .await
    .map_err(|e| db_err("insert residency audit log", e))?;

    if !is_allowed {
        sqlx::query(
            "INSERT INTO residency_violations
                 (policy_id, contract_id, attempted_region, action, attempted_by)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(policy.id)
        .bind(&req.contract_id)
        .bind(&req.requested_region)
        .bind(&req.action)
        .bind(&req.requested_by)
        .execute(&state.db)
        .await
        .map_err(|e| db_err("insert residency violation", e))?;

        tracing::warn!(
            contract_id    = %req.contract_id,
            policy_id      = %policy.id,
            region         = %req.requested_region,
            "residency violation detected and prevented"
        );
    }

    Ok(Json(serde_json::json!({
        "decision":         decision,
        "reason":           reason,
        "contract_id":      req.contract_id,
        "policy_id":        policy.id,
        "requested_region": req.requested_region,
        "allowed_regions":  policy.allowed_regions,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ResidencyLogQuery {
    pub contract_id: Option<String>,
    pub decision:    Option<String>,
    pub limit:       Option<i64>,
    pub page:        Option<i64>,
}

pub async fn get_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<ResidencyLogQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let limit  = params.limit.unwrap_or(20).min(100);
    let page   = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let mut filters: Vec<String> = Vec::new();
    if params.contract_id.is_some() { filters.push("contract_id = $1".into()); }
    if params.decision.is_some()    { filters.push(format!("decision = ${}", filters.len() + 1)); }

    let where_sql = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let list_sql = format!(
        "SELECT * FROM residency_audit_logs {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_sql, limit, offset
    );

    let mut q = sqlx::query_as::<_, ResidencyAuditLog>(&list_sql);
    if let Some(ref cid) = params.contract_id { q = q.bind(cid); }
    if let Some(ref dec) = params.decision    { q = q.bind(dec); }

    let logs: Vec<ResidencyAuditLog> = q
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list residency audit logs", e))?;

    Ok(Json(serde_json::json!({ "items": logs, "page": page, "limit": limit })))
}

pub async fn list_violations(
    State(state): State<AppState>,
    Query(params): Query<ListResidencyLogsParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let limit  = params.limit.unwrap_or(20).min(100);
    let page   = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let (where_sql, bind_cid) = match &params.contract_id {
        Some(_) => ("WHERE contract_id = $1", true),
        None    => ("", false),
    };

    let sql = format!(
        "SELECT * FROM residency_violations {} ORDER BY prevented_at DESC LIMIT {} OFFSET {}",
        where_sql, limit, offset
    );

    let mut q = sqlx::query_as::<_, ResidencyViolation>(&sql);
    if bind_cid { q = q.bind(params.contract_id.as_ref().unwrap()); }

    let violations: Vec<ResidencyViolation> = q
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list residency violations", e))?;

    Ok(Json(serde_json::json!({ "items": violations, "page": page, "limit": limit })))
}
