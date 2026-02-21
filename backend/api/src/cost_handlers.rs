use axum::{
    extract::{Path, State},
    Json,
};
use shared::models::{
    BatchCostEstimate, CostEstimate, CostEstimateRequest, CostForecast, CostOptimization,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// Stellar network constants (approximate)
const STROOPS_PER_XLM: i64 = 10_000_000;
const BASE_GAS_COST: i64 = 100_000; // stroops
const STORAGE_COST_PER_KB: i64 = 50_000; // stroops
const BANDWIDTH_COST_PER_KB: i64 = 10_000; // stroops

pub async fn estimate_cost(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CostEstimateRequest>,
) -> ApiResult<Json<CostEstimate>> {
    let invocations = req.invocations.unwrap_or(1);
    let storage_kb = req.storage_growth_kb.unwrap_or(0);

    // Try to get historical data
    let historical_gas = sqlx::query_scalar::<_, i64>(
        "SELECT avg_gas_cost FROM cost_estimates WHERE contract_id = $1 AND method_name = $2",
    )
    .bind(contract_id)
    .bind(&req.method_name)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let gas_cost = historical_gas.unwrap_or(BASE_GAS_COST) * invocations;
    let storage_cost = storage_kb * STORAGE_COST_PER_KB;
    let bandwidth_cost = (storage_kb / 4) * BANDWIDTH_COST_PER_KB; // Estimate 4:1 ratio

    let total_stroops = gas_cost + storage_cost + bandwidth_cost;
    let total_xlm = total_stroops as f64 / STROOPS_PER_XLM as f64;

    Ok(Json(CostEstimate {
        method_name: req.method_name,
        gas_cost,
        storage_cost,
        bandwidth_cost,
        total_stroops,
        total_xlm,
        invocations,
    }))
}

pub async fn batch_estimate(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(requests): Json<Vec<CostEstimateRequest>>,
) -> ApiResult<Json<BatchCostEstimate>> {
    let mut estimates = Vec::new();
    let mut total_stroops = 0i64;

    for req in requests {
        let invocations = req.invocations.unwrap_or(1);
        let storage_kb = req.storage_growth_kb.unwrap_or(0);

        let historical_gas = sqlx::query_scalar::<_, i64>(
            "SELECT avg_gas_cost FROM cost_estimates WHERE contract_id = $1 AND method_name = $2",
        )
        .bind(contract_id)
        .bind(&req.method_name)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        let gas_cost = historical_gas.unwrap_or(BASE_GAS_COST) * invocations;
        let storage_cost = storage_kb * STORAGE_COST_PER_KB;
        let bandwidth_cost = (storage_kb / 4) * BANDWIDTH_COST_PER_KB;

        let estimate_total = gas_cost + storage_cost + bandwidth_cost;
        total_stroops += estimate_total;

        estimates.push(CostEstimate {
            method_name: req.method_name,
            gas_cost,
            storage_cost,
            bandwidth_cost,
            total_stroops: estimate_total,
            total_xlm: estimate_total as f64 / STROOPS_PER_XLM as f64,
            invocations,
        });
    }

    Ok(Json(BatchCostEstimate {
        estimates,
        total_stroops,
        total_xlm: total_stroops as f64 / STROOPS_PER_XLM as f64,
    }))
}

pub async fn optimize_costs(
    State(_state): State<AppState>,
    Path(_contract_id): Path<Uuid>,
    Json(estimate): Json<CostEstimate>,
) -> ApiResult<Json<CostOptimization>> {
    let mut suggestions = Vec::new();
    let current_cost = estimate.total_stroops;
    let mut optimized_cost = current_cost;

    // Suggest batching if multiple invocations
    if estimate.invocations > 1 {
        suggestions.push("Batch multiple operations into single transaction".to_string());
        optimized_cost = (optimized_cost as f64 * 0.85) as i64; // 15% savings
    }

    // Suggest storage optimization
    if estimate.storage_cost > estimate.gas_cost {
        suggestions.push("Optimize data structures to reduce storage footprint".to_string());
        optimized_cost = (optimized_cost as f64 * 0.90) as i64; // 10% savings
    }

    // Suggest caching
    if estimate.gas_cost > 500_000 {
        suggestions.push("Implement caching to reduce redundant computations".to_string());
        optimized_cost = (optimized_cost as f64 * 0.92) as i64; // 8% savings
    }

    let savings_percent = ((current_cost - optimized_cost) as f64 / current_cost as f64) * 100.0;

    Ok(Json(CostOptimization {
        current_cost,
        optimized_cost,
        savings_percent,
        suggestions,
    }))
}

pub async fn forecast_costs(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CostEstimateRequest>,
) -> ApiResult<Json<CostForecast>> {
    let daily_invocations = req.invocations.unwrap_or(100);
    let storage_kb = req.storage_growth_kb.unwrap_or(1);

    let historical_gas = sqlx::query_scalar::<_, i64>(
        "SELECT avg_gas_cost FROM cost_estimates WHERE contract_id = $1 AND method_name = $2",
    )
    .bind(contract_id)
    .bind(&req.method_name)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let gas_per_call = historical_gas.unwrap_or(BASE_GAS_COST);
    let storage_cost = storage_kb * STORAGE_COST_PER_KB;
    let bandwidth_cost = (storage_kb / 4) * BANDWIDTH_COST_PER_KB;

    let daily_cost_stroops = (gas_per_call * daily_invocations) + storage_cost + bandwidth_cost;
    let daily_cost_xlm = daily_cost_stroops as f64 / STROOPS_PER_XLM as f64;

    Ok(Json(CostForecast {
        daily_cost_xlm,
        monthly_cost_xlm: daily_cost_xlm * 30.0,
        yearly_cost_xlm: daily_cost_xlm * 365.0,
        usage_pattern: format!("{} invocations/day, {} KB storage/day", daily_invocations, storage_kb),
    }))
}
