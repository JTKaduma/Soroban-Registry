// api/src/capacity_handlers.rs
//
// Axum handlers for Contract Capacity Planning.
//
// Routes (register in capacity_routes.rs):
//   POST   /contracts/:id/resource-snapshots       → record_snapshot
//   GET    /contracts/:id/resource-snapshots        → list_snapshots
//   GET    /contracts/:id/capacity-plan             → get_capacity_plan
//   GET    /contracts/:id/capacity-alerts           → list_alerts
//   PATCH  /contracts/:id/capacity-alerts/:aid/ack  → acknowledge_alert
//   GET    /contracts/:id/capacity-recommendations  → list_recommendations

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;

// ── Types come from the shared crate (shared/src/capacity_models.rs) ─────────
use shared::{
    AcknowledgeAlertRequest, CapacityAlert, CapacityPlanParams, CapacityPlanResponse,
    RecordSnapshotRequest, ResourceKind, ResourceLimits, ResourceSnapshot,
};

// ── Engine is api-internal: pure computation, no DB, lives in api/src/ ───────
use crate::{
    capacity_engine::{
        build_scenario_bundle, current_value_from_snapshots, estimate_cost,
        evaluate_alert, generate_recommendations, limit_for, nearest_breach_days,
        overall_status,
    },
    state::AppState,
};

// ─────────────────────────────────────────────────────────
// POST /contracts/:id/resource-snapshots
// Record a new resource measurement for a contract.
// ─────────────────────────────────────────────────────────

pub async fn record_snapshot(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<RecordSnapshotRequest>,
) -> impl IntoResponse {
    if req.value < 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_VALUE",
                "message": "value must be >= 0"
            })),
        )
            .into_response();
    }

    let row = sqlx::query_as::<_, ResourceSnapshot>(
        r#"
        INSERT INTO resource_snapshots
            (id, contract_id, resource, value, tag, recorded_at)
        VALUES
            (gen_random_uuid(), $1, $2, $3, $4, NOW())
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&req.resource)
    .bind(req.value)
    .bind(&req.tag)
    .fetch_one(&state.db)
    .await;

    match row {
        Err(e) => {
            tracing::error!("record_snapshot DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(snap) => (StatusCode::CREATED, Json(snap)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/resource-snapshots
// Return the most recent 90 snapshots per resource kind.
// ─────────────────────────────────────────────────────────

pub async fn list_snapshots(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, ResourceSnapshot>(
        r#"
        SELECT * FROM resource_snapshots
        WHERE contract_id = $1
        ORDER BY recorded_at DESC
        LIMIT 540
        "#,
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await;

    match rows {
        Err(e) => {
            tracing::error!("list_snapshots DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(snaps) => (StatusCode::OK, Json(snaps)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/capacity-plan
// The main endpoint: full forecast + alerts + recs + costs.
// ─────────────────────────────────────────────────────────

pub async fn get_capacity_plan(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(params): Query<CapacityPlanParams>,
) -> impl IntoResponse {
    let horizon = params.horizon_months.clamp(1, 36);
    let xlm_usd = params.xlm_usd.clamp(0.001, 1000.0);

    let snaps = sqlx::query_as::<_, ResourceSnapshot>(
        "SELECT * FROM resource_snapshots WHERE contract_id = $1 ORDER BY recorded_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await;

    let snaps = match snaps {
        Err(e) => {
            tracing::error!("capacity_plan snapshots error: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response();
        }
        Ok(s) => s,
    };

    let limits = ResourceLimits::default();
    let now    = Utc::now();

    let all_resources = [
        ResourceKind::StorageEntries,
        ResourceKind::CpuInstructions,
        ResourceKind::UniqueUsers,
        ResourceKind::TransactionVolume,
        ResourceKind::WasmSizeBytes,
        ResourceKind::FeePerOperation,
    ];

    let scenarios: Vec<_> = all_resources.iter().map(|resource| {
        let current = current_value_from_snapshots(&snaps, resource);
        let limit   = limit_for(resource, &limits);
        build_scenario_bundle(
            contract_id, resource.clone(), current, limit,
            horizon, params.custom_rate, now,
        )
    }).collect();

    let alerts: Vec<CapacityAlert> = scenarios.iter()
        .filter_map(|bundle| evaluate_alert(contract_id, bundle, now))
        .collect();

    // Upsert alerts into DB so they can be acknowledged later
    for alert in &alerts {
        let _ = sqlx::query(
            r#"
            INSERT INTO capacity_alerts
                (id, contract_id, resource, severity, current_value,
                 limit_value, pct_consumed, breach_predicted_at,
                 days_until_breach, message, acknowledged, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, false, NOW())
            ON CONFLICT (contract_id, resource) DO UPDATE SET
                severity            = EXCLUDED.severity,
                current_value       = EXCLUDED.current_value,
                pct_consumed        = EXCLUDED.pct_consumed,
                breach_predicted_at = EXCLUDED.breach_predicted_at,
                days_until_breach   = EXCLUDED.days_until_breach,
                message             = EXCLUDED.message,
                resolved_at         = NULL
            "#,
        )
        .bind(alert.id)
        .bind(alert.contract_id)
        .bind(&alert.resource)
        .bind(&alert.severity)
        .bind(alert.current_value)
        .bind(alert.limit_value)
        .bind(alert.pct_consumed)
        .bind(alert.breach_predicted_at)
        .bind(alert.days_until_breach)
        .bind(&alert.message)
        .execute(&state.db)
        .await;
    }

    let recommendations = generate_recommendations(contract_id, &scenarios, now);

    let cost_estimates: Vec<_> = scenarios.iter().map(|bundle| {
        let projected = bundle.base.points.last()
            .map(|p| p.projected_value)
            .unwrap_or(bundle.current_value);
        estimate_cost(
            bundle.resource.clone(),
            bundle.current_value,
            projected,
            horizon,
            xlm_usd,
        )
    }).collect();

    let status  = overall_status(&alerts);
    let nearest = nearest_breach_days(&scenarios);

    let response = CapacityPlanResponse {
        contract_id,
        generated_at: now,
        scenarios,
        alerts,
        recommendations,
        cost_estimates,
        overall_status: status,
        nearest_breach_days: nearest,
    };

    (StatusCode::OK, Json(response)).into_response()
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/capacity-alerts
// List all active (unresolved) alerts for a contract.
// ─────────────────────────────────────────────────────────

pub async fn list_alerts(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, CapacityAlert>(
        r#"
        SELECT * FROM capacity_alerts
        WHERE contract_id = $1
          AND resolved_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await;

    match rows {
        Err(e) => {
            tracing::error!("list_alerts DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(alerts) => (StatusCode::OK, Json(alerts)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// PATCH /contracts/:id/capacity-alerts/:alert_id/ack
// Mark an alert as acknowledged.
// ─────────────────────────────────────────────────────────

pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Path((contract_id, alert_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AcknowledgeAlertRequest>,
) -> impl IntoResponse {
    let result = sqlx::query(
        r#"
        UPDATE capacity_alerts
        SET acknowledged = true,
            message = message || ' [ack: ' || $3 || ']'
        WHERE id = $1 AND contract_id = $2
        "#,
    )
    .bind(alert_id)
    .bind(contract_id)
    .bind(&req.acknowledged_by)
    .execute(&state.db)
    .await;

    match result {
        Err(e) => {
            tracing::error!("acknowledge_alert DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(r) if r.rows_affected() == 0 => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": "Alert not found for this contract"
            })),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "acknowledged": true, "alert_id": alert_id })),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/capacity-recommendations
// Lightweight — returns recommendations without a full plan recompute.
// ─────────────────────────────────────────────────────────

pub async fn list_recommendations(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    let snaps = sqlx::query_as::<_, ResourceSnapshot>(
        "SELECT * FROM resource_snapshots WHERE contract_id = $1 ORDER BY recorded_at DESC LIMIT 100",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await;

    let snaps = match snaps {
        Err(e) => {
            tracing::error!("list_recommendations error: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response();
        }
        Ok(s) => s,
    };

    let limits = ResourceLimits::default();
    let now    = Utc::now();

    let bundles: Vec<_> = [
        ResourceKind::StorageEntries,
        ResourceKind::CpuInstructions,
        ResourceKind::UniqueUsers,
        ResourceKind::TransactionVolume,
        ResourceKind::WasmSizeBytes,
        ResourceKind::FeePerOperation,
    ]
    .iter()
    .map(|r| {
        let current = current_value_from_snapshots(&snaps, r);
        let limit   = limit_for(r, &limits);
        build_scenario_bundle(contract_id, r.clone(), current, limit, 12, None, now)
    })
    .collect();

    let recs = generate_recommendations(contract_id, &bundles, now);
    (StatusCode::OK, Json(recs)).into_response()
}