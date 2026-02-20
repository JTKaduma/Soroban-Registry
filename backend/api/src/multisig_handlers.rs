// multisig_handlers.rs
// Axum handlers for Multi-Signature Contract Deployment (issue #47)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use shared::{
    CreatePolicyRequest, CreateProposalRequest, MultisigPolicy, DeployProposal,
    ProposalSignature, ProposalStatus, ProposalWithSignatures, SignProposalRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helper
// ─────────────────────────────────────────────────────────────────────────────

fn map_json_rejection(err: axum::extract::rejection::JsonRejection) -> ApiError {
    ApiError::bad_request(
        "InvalidRequest",
        format!("Invalid JSON payload: {}", err.body_text()),
    )
}

/// Fetch a proposal by its UUID, returning 404 if not found.
async fn fetch_proposal(state: &AppState, id: Uuid) -> ApiResult<DeployProposal> {
    sqlx::query_as("SELECT * FROM deploy_proposals WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ProposalNotFound",
                format!("No proposal found with ID: {}", id),
            ),
            _ => db_internal_error("fetch proposal", err),
        })
}

/// Transition an expired proposal to `expired` status in the DB.
async fn expire_proposal(state: &AppState, id: Uuid) -> ApiResult<()> {
    sqlx::query(
        "UPDATE deploy_proposals SET status = 'expired', updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("expire proposal", err))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/multisig/policies
// ─────────────────────────────────────────────────────────────────────────────

/// Create a new multi-sig policy that defines signer list and threshold.
pub async fn create_policy(
    State(state): State<AppState>,
    payload: Result<Json<CreatePolicyRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<MultisigPolicy>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    // Validation
    if req.threshold < 1 {
        return Err(ApiError::bad_request(
            "InvalidThreshold",
            "threshold must be at least 1",
        ));
    }
    if req.signer_addresses.is_empty() {
        return Err(ApiError::bad_request(
            "InvalidSigners",
            "signer_addresses must not be empty",
        ));
    }
    if req.threshold as usize > req.signer_addresses.len() {
        return Err(ApiError::bad_request(
            "ThresholdExceedsSigners",
            format!(
                "threshold ({}) cannot exceed the number of signers ({})",
                req.threshold,
                req.signer_addresses.len()
            ),
        ));
    }
    if req.created_by.is_empty() {
        return Err(ApiError::bad_request(
            "MissingProposer",
            "created_by field is required",
        ));
    }

    let expiry_seconds = req.expiry_seconds.unwrap_or(86_400);

    let policy: MultisigPolicy = sqlx::query_as(
        "INSERT INTO multisig_policies (name, threshold, signer_addresses, expiry_seconds, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(&req.name)
    .bind(req.threshold)
    .bind(&req.signer_addresses)
    .bind(expiry_seconds)
    .bind(&req.created_by)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create multisig policy", err))?;

    tracing::info!(policy_id = %policy.id, threshold = policy.threshold, "multisig policy created");

    Ok(Json(policy))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/contracts/deploy-proposal
// ─────────────────────────────────────────────────────────────────────────────

/// Create an unsigned deployment proposal. The proposal will stay `pending`
/// until enough signers have signed it (threshold reached → `approved`).
pub async fn create_proposal(
    State(state): State<AppState>,
    payload: Result<Json<CreateProposalRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<DeployProposal>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    // Validate required fields
    if req.contract_id.is_empty() {
        return Err(ApiError::bad_request("MissingContractId", "contract_id is required"));
    }
    if req.wasm_hash.is_empty() {
        return Err(ApiError::bad_request("MissingWasmHash", "wasm_hash is required"));
    }
    if req.proposer.is_empty() {
        return Err(ApiError::bad_request("MissingProposer", "proposer is required"));
    }

    // Look up the policy to compute expires_at
    let policy: MultisigPolicy =
        sqlx::query_as("SELECT * FROM multisig_policies WHERE id = $1")
            .bind(req.policy_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "PolicyNotFound",
                    format!("No policy found with ID: {}", req.policy_id),
                ),
                _ => db_internal_error("fetch policy for proposal", err),
            })?;

    let expires_at = Utc::now()
        + chrono::Duration::seconds(policy.expiry_seconds as i64);

    let proposal: DeployProposal = sqlx::query_as(
        "INSERT INTO deploy_proposals
            (contract_name, contract_id, wasm_hash, network, description, policy_id, expires_at, proposer)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING *",
    )
    .bind(&req.contract_name)
    .bind(&req.contract_id)
    .bind(&req.wasm_hash)
    .bind(&req.network)
    .bind(&req.description)
    .bind(req.policy_id)
    .bind(expires_at)
    .bind(&req.proposer)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create deploy proposal", err))?;

    tracing::info!(
        proposal_id = %proposal.id,
        contract_id = %proposal.contract_id,
        threshold   = policy.threshold,
        expires_at  = %proposal.expires_at,
        "deployment proposal created"
    );

    Ok(Json(proposal))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/contracts/{id}/sign
// ─────────────────────────────────────────────────────────────────────────────

/// Add one signature to a proposal. Validates:
/// - Proposal exists and is still `pending`
/// - Proposal has not expired
/// - Signer is in the policy's signer list
/// - Signer has not already signed
///
/// If the threshold is met after this signature the proposal moves to `approved`.
pub async fn sign_proposal(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
    payload: Result<Json<SignProposalRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    let mut proposal = fetch_proposal(&state, proposal_id).await?;

    // Check expiry
    if Utc::now() > proposal.expires_at {
        if proposal.status == ProposalStatus::Pending {
            expire_proposal(&state, proposal_id).await?;
        }
        return Err(ApiError::new(
            StatusCode::GONE,
            "ProposalExpired",
            "This proposal has expired and can no longer be signed",
        ));
    }

    // Only pending proposals can be signed
    if proposal.status != ProposalStatus::Pending {
        return Err(ApiError::bad_request(
            "ProposalNotPending",
            format!(
                "Proposal is in '{}' status and cannot be signed",
                proposal.status
            ),
        ));
    }

    // Fetch the policy to validate the signer
    let policy: MultisigPolicy =
        sqlx::query_as("SELECT * FROM multisig_policies WHERE id = $1")
            .bind(proposal.policy_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch policy for signing", err))?;

    if !policy.signer_addresses.contains(&req.signer_address) {
        return Err(ApiError::bad_request(
            "UnauthorizedSigner",
            format!(
                "'{}' is not an authorized signer for this proposal",
                req.signer_address
            ),
        ));
    }

    // Insert signature (UNIQUE constraint on (proposal_id, signer_address) handles duplicates)
    let signature: ProposalSignature = sqlx::query_as(
        "INSERT INTO proposal_signatures (proposal_id, signer_address, signature_data)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(proposal_id)
    .bind(&req.signer_address)
    .bind(&req.signature_data)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(ref db_err)
            if db_err.constraint() == Some("proposal_signatures_proposal_id_signer_address_key") =>
        {
            ApiError::bad_request(
                "AlreadySigned",
                format!("'{}' has already signed this proposal", req.signer_address),
            )
        }
        _ => db_internal_error("insert proposal signature", err),
    })?;

    // Count total signatures so far
    let sig_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM proposal_signatures WHERE proposal_id = $1")
            .bind(proposal_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("count signatures", err))?;

    // Promote to approved if threshold met
    if sig_count >= policy.threshold as i64 {
        sqlx::query(
            "UPDATE deploy_proposals SET status = 'approved', updated_at = NOW() WHERE id = $1",
        )
        .bind(proposal_id)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("approve proposal", err))?;
        proposal.status = ProposalStatus::Approved;

        tracing::info!(
            proposal_id = %proposal_id,
            sig_count   = sig_count,
            threshold   = policy.threshold,
            "proposal threshold reached — status: approved"
        );
    }

    let signatures_needed = (policy.threshold as i64 - sig_count).max(0) as i32;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "signature": signature,
            "proposal_status": proposal.status.to_string(),
            "signatures_collected": sig_count,
            "signatures_needed": signatures_needed,
            "threshold_met": signatures_needed == 0,
        })),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/contracts/{id}/execute
// ─────────────────────────────────────────────────────────────────────────────

/// Execute an approved deployment proposal. The proposal must be in `approved`
/// status and not expired. Once executed the status transitions to `executed`.
pub async fn execute_proposal(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let proposal = fetch_proposal(&state, proposal_id).await?;

    // Check expiry even for approved proposals
    if Utc::now() > proposal.expires_at {
        if proposal.status != ProposalStatus::Executed {
            expire_proposal(&state, proposal_id).await?;
        }
        return Err(ApiError::new(
            StatusCode::GONE,
            "ProposalExpired",
            "This proposal has expired and cannot be executed",
        ));
    }

    if proposal.status != ProposalStatus::Approved {
        return Err(ApiError::bad_request(
            "ProposalNotApproved",
            format!(
                "Proposal must be in 'approved' status to execute. Current status: '{}'",
                proposal.status
            ),
        ));
    }

    // Mark as executed
    sqlx::query(
        "UPDATE deploy_proposals
         SET status = 'executed', executed_at = NOW(), updated_at = NOW()
         WHERE id = $1",
    )
    .bind(proposal_id)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("execute proposal", err))?;

    tracing::info!(
        proposal_id  = %proposal_id,
        contract_id  = %proposal.contract_id,
        wasm_hash    = %proposal.wasm_hash,
        "deployment proposal executed"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "proposal_id": proposal_id,
        "contract_id": proposal.contract_id,
        "wasm_hash": proposal.wasm_hash,
        "executed_at": Utc::now().to_rfc3339(),
        "message": "Deployment proposal executed successfully"
    })))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/{id}/proposal
// ─────────────────────────────────────────────────────────────────────────────

/// Return a proposal with its policy and all collected signatures.
pub async fn get_proposal(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
) -> ApiResult<Json<ProposalWithSignatures>> {
    let proposal = fetch_proposal(&state, proposal_id).await?;

    let policy: MultisigPolicy =
        sqlx::query_as("SELECT * FROM multisig_policies WHERE id = $1")
            .bind(proposal.policy_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch policy for proposal info", err))?;

    let signatures: Vec<ProposalSignature> = sqlx::query_as(
        "SELECT * FROM proposal_signatures WHERE proposal_id = $1 ORDER BY signed_at ASC",
    )
    .bind(proposal_id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list proposal signatures", err))?;

    let collected = signatures.len() as i32;
    let signatures_needed = (policy.threshold - collected).max(0);

    Ok(Json(ProposalWithSignatures {
        proposal,
        policy,
        signatures,
        signatures_needed,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/multisig/proposals
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListProposalsParams {
    pub status: Option<String>,
    pub policy_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

/// List all deployment proposals, with optional status / policy filters.
pub async fn list_proposals(
    State(state): State<AppState>,
    Query(params): Query<ListProposalsParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    // Dynamic query builder (safe — values are bound, not interpolated)
    let mut where_clauses: Vec<String> = Vec::new();
    let mut arg_idx = 1usize;

    if let Some(ref s) = params.status {
        where_clauses.push(format!("status = ${}", arg_idx));
        arg_idx += 1;
    }
    if params.policy_id.is_some() {
        where_clauses.push(format!("policy_id = ${}", arg_idx));
        arg_idx += 1;
    }
    let _ = arg_idx; // suppress unused warning

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM deploy_proposals {}", where_sql);
    let list_sql = format!(
        "SELECT * FROM deploy_proposals {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_sql, limit, offset
    );

    // Build and execute count query
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(ref s) = params.status {
        count_q = count_q.bind(s.clone());
    }
    if let Some(pid) = params.policy_id {
        count_q = count_q.bind(pid);
    }
    let total: i64 = count_q
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count proposals", err))?;

    // Build and execute list query
    let mut list_q = sqlx::query_as::<_, DeployProposal>(&list_sql);
    if let Some(ref s) = params.status {
        list_q = list_q.bind(s.clone());
    }
    if let Some(pid) = params.policy_id {
        list_q = list_q.bind(pid);
    }
    let proposals: Vec<DeployProposal> = list_q
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("list proposals", err))?;

    let total_pages = ((total as f64) / (limit as f64)).ceil() as i64;

    Ok(Json(serde_json::json!({
        "items": proposals,
        "total": total,
        "page": page,
        "pages": total_pages,
    })))
}
