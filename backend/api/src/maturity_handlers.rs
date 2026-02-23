use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use shared::models::{
    Contract, MaturityChange, MaturityCriterion, MaturityLevel, MaturityRequirements,
    UpdateMaturityRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn update_maturity(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<UpdateMaturityRequest>,
) -> ApiResult<Json<Contract>> {
    let contract = sqlx::query_as::<_, Contract>("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    // Log the change
    sqlx::query(
        "INSERT INTO maturity_changes (contract_id, from_level, to_level, reason, changed_by) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(contract_id)
    .bind(&contract.maturity)
    .bind(&req.maturity)
    .bind(&req.reason)
    .bind(contract.publisher_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to log change: {}", e)))?;

    // Update contract
    let updated = sqlx::query_as::<_, Contract>(
        "UPDATE contracts SET maturity = $1 WHERE id = $2 RETURNING *",
    )
    .bind(&req.maturity)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update maturity: {}", e)))?;

    Ok(Json(updated))
}

pub async fn get_maturity_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<MaturityChange>>> {
    let changes = sqlx::query_as::<_, MaturityChange>(
        "SELECT * FROM maturity_changes WHERE contract_id = $1 ORDER BY changed_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(changes))
}

pub async fn check_maturity_requirements(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<MaturityRequirements>>> {
    let contract = sqlx::query_as::<_, Contract>("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    let versions_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contract_versions WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let interactions_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contract_interactions WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let requirements = vec![
        check_beta_requirements(&contract, versions_count, interactions_count),
        check_stable_requirements(&contract, versions_count, interactions_count),
        check_mature_requirements(&contract, versions_count, interactions_count),
    ];

    Ok(Json(requirements))
}

fn check_beta_requirements(
    contract: &Contract,
    versions: i64,
    _interactions: i64,
) -> MaturityRequirements {
    let criteria = vec![
        MaturityCriterion {
            name: "verified".to_string(),
            required: true,
            met: contract.is_verified,
            description: "Contract source code must be verified".to_string(),
        },
        MaturityCriterion {
            name: "versions".to_string(),
            required: true,
            met: versions >= 1,
            description: "At least 1 version published".to_string(),
        },
    ];

    let met = criteria.iter().all(|c| !c.required || c.met);

    MaturityRequirements {
        level: MaturityLevel::Beta,
        criteria,
        met,
    }
}

fn check_stable_requirements(
    contract: &Contract,
    versions: i64,
    interactions: i64,
) -> MaturityRequirements {
    let criteria = vec![
        MaturityCriterion {
            name: "verified".to_string(),
            required: true,
            met: contract.is_verified,
            description: "Contract source code must be verified".to_string(),
        },
        MaturityCriterion {
            name: "versions".to_string(),
            required: true,
            met: versions >= 2,
            description: "At least 2 versions published".to_string(),
        },
        MaturityCriterion {
            name: "usage".to_string(),
            required: true,
            met: interactions >= 10,
            description: "At least 10 contract interactions".to_string(),
        },
    ];

    let met = criteria.iter().all(|c| !c.required || c.met);

    MaturityRequirements {
        level: MaturityLevel::Stable,
        criteria,
        met,
    }
}

fn check_mature_requirements(
    contract: &Contract,
    versions: i64,
    interactions: i64,
) -> MaturityRequirements {
    let criteria = vec![
        MaturityCriterion {
            name: "verified".to_string(),
            required: true,
            met: contract.is_verified,
            description: "Contract source code must be verified".to_string(),
        },
        MaturityCriterion {
            name: "versions".to_string(),
            required: true,
            met: versions >= 5,
            description: "At least 5 versions published".to_string(),
        },
        MaturityCriterion {
            name: "usage".to_string(),
            required: true,
            met: interactions >= 100,
            description: "At least 100 contract interactions".to_string(),
        },
    ];

    let met = criteria.iter().all(|c| !c.required || c.met);

    MaturityRequirements {
        level: MaturityLevel::Mature,
        criteria,
        met,
    }
}
