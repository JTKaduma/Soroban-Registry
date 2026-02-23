// api/src/feature_flag_handlers.rs
//
// Axum handlers for Contract Experimental Feature Flags.
// These expose the on-chain feature_flags.rs module's state through the
// Soroban Registry REST API, allowing off-chain management and analytics.
//
// Routes (see feature_flag_routes.rs):
//   POST   /contracts/:id/feature-flags               → create_flag
//   GET    /contracts/:id/feature-flags               → list_flags
//   GET    /contracts/:id/feature-flags/:name         → get_flag
//   PATCH  /contracts/:id/feature-flags/:name/enable  → enable_flag
//   PATCH  /contracts/:id/feature-flags/:name/disable → disable_flag
//   PATCH  /contracts/:id/feature-flags/:name/sunset  → sunset_flag
//   PATCH  /contracts/:id/feature-flags/:name/rollout → update_rollout
//   GET    /contracts/:id/feature-flags/:name/analytics → get_analytics
//   POST   /contracts/:id/feature-flags/:name/ab-test → configure_ab_test
//   GET    /contracts/:id/feature-flags/:name/ab-test → get_ab_test
//   POST   /contracts/:id/feature-flags/sweep         → sweep_expired
//   GET    /contracts/:id/feature-flags/:name/check   → check_enabled

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use shared::{
    AbTestConfig, CheckEnabledParams, ConfigureAbTestRequest, CreateFeatureFlagRequest,
    FeatureFlag, FeatureFlagAnalytics, FeatureFlagListResponse, UpdateRolloutRequest,
};

// ─────────────────────────────────────────────────────────
// POST /contracts/:id/feature-flags
// Create a new feature flag for a contract (starts Inactive).
// ─────────────────────────────────────────────────────────

pub async fn create_flag(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreateFeatureFlagRequest>,
) -> impl IntoResponse {
    // Validate name: alphanumeric + underscores only, max 64 chars
    if req.name.is_empty()
        || req.name.len() > 64
        || !req.name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_NAME",
                "message": "Flag name must be 1–64 alphanumeric/underscore characters"
            })),
        )
            .into_response();
    }

    // Validate rollout percentage if gradual
    if let Some(pct) = req.rollout_percentage {
        if pct > 100 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "INVALID_ROLLOUT",
                    "message": "rollout_percentage must be 0–100"
                })),
            )
                .into_response();
        }
    }

    // Check flag name is unique for this contract
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM feature_flags WHERE contract_id = $1 AND name = $2)",
    )
    .bind(contract_id)
    .bind(&req.name)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if exists {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "ALREADY_EXISTS",
                "message": format!("Flag '{}' already exists for this contract", req.name)
            })),
        )
            .into_response();
    }

    let rollout_strategy = req.rollout_percentage
        .map(|p| if p == 100 { "full".to_string() } else { "gradual".to_string() })
        .unwrap_or_else(|| "full".to_string());

    let row = sqlx::query_as::<_, FeatureFlag>(
        r#"
        INSERT INTO feature_flags (
            id, contract_id, name, description,
            state, rollout_strategy, rollout_percentage,
            sunset_at, created_by, ab_enabled,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), $1, $2, $3,
            'inactive', $4, $5,
            $6, $7, false,
            NOW(), NOW()
        )
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&rollout_strategy)
    .bind(req.rollout_percentage.unwrap_or(100) as i32)
    .bind(req.sunset_at)
    .bind(&req.created_by)
    .fetch_one(&state.db)
    .await;

    match row {
        Err(e) => {
            tracing::error!("create_flag DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(flag) => (StatusCode::CREATED, Json(flag)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/feature-flags
// List all flags for a contract, with optional state filter.
// ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListFlagsParams {
    /// Filter by state: "active" | "inactive" | "sunset"
    pub state: Option<String>,
}

pub async fn list_flags(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(params): Query<ListFlagsParams>,
) -> impl IntoResponse {
    let rows = if let Some(ref filter_state) = params.state {
        sqlx::query_as::<_, FeatureFlag>(
            "SELECT * FROM feature_flags WHERE contract_id = $1 AND state = $2 ORDER BY created_at DESC",
        )
        .bind(contract_id)
        .bind(filter_state)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, FeatureFlag>(
            "SELECT * FROM feature_flags WHERE contract_id = $1 ORDER BY created_at DESC",
        )
        .bind(contract_id)
        .fetch_all(&state.db)
        .await
    };

    match rows {
        Err(e) => {
            tracing::error!("list_flags DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(flags) => {
            let active_count   = flags.iter().filter(|f| f.state == "active").count();
            let inactive_count = flags.iter().filter(|f| f.state == "inactive").count();
            let sunset_count   = flags.iter().filter(|f| f.state == "sunset").count();

            let response = FeatureFlagListResponse {
                contract_id,
                flags,
                active_count,
                inactive_count,
                sunset_count,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/feature-flags/:name
// Get a single flag by name.
// ─────────────────────────────────────────────────────────

pub async fn get_flag(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, FeatureFlag>(
        "SELECT * FROM feature_flags WHERE contract_id = $1 AND name = $2",
    )
    .bind(contract_id)
    .bind(&name)
    .fetch_optional(&state.db)
    .await;

    match row {
        Err(e) => {
            tracing::error!("get_flag DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": format!("Flag '{}' not found", name)
            })),
        )
            .into_response(),
        Ok(Some(flag)) => (StatusCode::OK, Json(flag)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// PATCH /contracts/:id/feature-flags/:name/enable
// Transition Inactive → Active.
// ─────────────────────────────────────────────────────────

pub async fn enable_flag(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    transition_flag_state(&state, contract_id, &name, "inactive", "active", "ALREADY_ACTIVE").await
}

// ─────────────────────────────────────────────────────────
// PATCH /contracts/:id/feature-flags/:name/disable
// Transition Active → Inactive.
// ─────────────────────────────────────────────────────────

pub async fn disable_flag(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    transition_flag_state(&state, contract_id, &name, "active", "inactive", "ALREADY_INACTIVE").await
}

// ─────────────────────────────────────────────────────────
// PATCH /contracts/:id/feature-flags/:name/sunset
// Immediately sunset a flag (terminal state — cannot be re-enabled).
// ─────────────────────────────────────────────────────────

pub async fn sunset_flag(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, FeatureFlag>(
        r#"
        UPDATE feature_flags
        SET state = 'sunset', updated_at = NOW()
        WHERE contract_id = $1 AND name = $2 AND state != 'sunset'
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&name)
    .fetch_optional(&state.db)
    .await;

    match result {
        Err(e) => {
            tracing::error!("sunset_flag DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "ALREADY_SUNSET",
                "message": "Flag is already sunsetted or does not exist"
            })),
        )
            .into_response(),
        Ok(Some(flag)) => (StatusCode::OK, Json(flag)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// PATCH /contracts/:id/feature-flags/:name/rollout
// Adjust rollout percentage or strategy.
// ─────────────────────────────────────────────────────────

pub async fn update_rollout(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
    Json(req): Json<UpdateRolloutRequest>,
) -> impl IntoResponse {
    if req.percentage > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_PERCENTAGE",
                "message": "percentage must be 0–100"
            })),
        )
            .into_response();
    }

    let strategy = if req.percentage == 100 { "full" } else { "gradual" };

    let result = sqlx::query_as::<_, FeatureFlag>(
        r#"
        UPDATE feature_flags
        SET rollout_strategy  = $3,
            rollout_percentage = $4,
            updated_at         = NOW()
        WHERE contract_id = $1 AND name = $2 AND state != 'sunset'
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&name)
    .bind(strategy)
    .bind(req.percentage as i32)
    .fetch_optional(&state.db)
    .await;

    match result {
        Err(e) => {
            tracing::error!("update_rollout DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": "Flag not found or is already sunsetted"
            })),
        )
            .into_response(),
        Ok(Some(flag)) => (StatusCode::OK, Json(flag)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/feature-flags/:name/analytics
// Return usage analytics for a flag.
// ─────────────────────────────────────────────────────────

pub async fn get_analytics(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, FeatureFlagAnalytics>(
        r#"
        SELECT
            ffa.*,
            -- hit rate in basis points (e.g. 7500 = 75.00%)
            CASE WHEN ffa.total_checks > 0
                 THEN (ffa.enabled_hits * 10000 / ffa.total_checks)
                 ELSE 0
            END AS hit_rate_bps
        FROM feature_flag_analytics ffa
        JOIN feature_flags ff ON ff.id = ffa.flag_id
        WHERE ff.contract_id = $1 AND ff.name = $2
        "#,
    )
    .bind(contract_id)
    .bind(&name)
    .fetch_optional(&state.db)
    .await;

    match row {
        Err(e) => {
            tracing::error!("get_analytics DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": format!("No analytics found for flag '{}'", name)
            })),
        )
            .into_response(),
        Ok(Some(analytics)) => (StatusCode::OK, Json(analytics)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// POST /contracts/:id/feature-flags/:name/ab-test
// Configure or update an A/B test for a flag.
// ─────────────────────────────────────────────────────────

pub async fn configure_ab_test(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
    Json(req): Json<ConfigureAbTestRequest>,
) -> impl IntoResponse {
    if req.variant_a_pct + req.variant_b_pct != 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_SPLIT",
                "message": "variant_a_pct + variant_b_pct must equal 100"
            })),
        )
            .into_response();
    }

    // Mark the flag as A/B enabled and upsert the config
    let result = sqlx::query(
        "UPDATE feature_flags SET ab_enabled = true, updated_at = NOW() WHERE contract_id = $1 AND name = $2",
    )
    .bind(contract_id)
    .bind(&name)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        tracing::error!("configure_ab_test flag update error: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "DB_ERROR" })),
        )
            .into_response();
    }

    let row = sqlx::query_as::<_, AbTestConfig>(
        r#"
        INSERT INTO ab_test_configs (
            id, contract_id, flag_name,
            variant_a_pct, variant_b_pct,
            variant_a_label, variant_b_label,
            started_at, ends_at
        ) VALUES (
            gen_random_uuid(), $1, $2,
            $3, $4, $5, $6,
            NOW(), $7
        )
        ON CONFLICT (contract_id, flag_name) DO UPDATE SET
            variant_a_pct   = EXCLUDED.variant_a_pct,
            variant_b_pct   = EXCLUDED.variant_b_pct,
            variant_a_label = EXCLUDED.variant_a_label,
            variant_b_label = EXCLUDED.variant_b_label,
            ends_at         = EXCLUDED.ends_at
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&name)
    .bind(req.variant_a_pct as i32)
    .bind(req.variant_b_pct as i32)
    .bind(&req.variant_a_label)
    .bind(&req.variant_b_label)
    .bind(req.ends_at)
    .fetch_one(&state.db)
    .await;

    match row {
        Err(e) => {
            tracing::error!("configure_ab_test upsert error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/feature-flags/:name/ab-test
// Get the current A/B test config for a flag.
// ─────────────────────────────────────────────────────────

pub async fn get_ab_test(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, AbTestConfig>(
        "SELECT * FROM ab_test_configs WHERE contract_id = $1 AND flag_name = $2",
    )
    .bind(contract_id)
    .bind(&name)
    .fetch_optional(&state.db)
    .await;

    match row {
        Err(e) => {
            tracing::error!("get_ab_test DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": "No A/B test configured for this flag"
            })),
        )
            .into_response(),
        Ok(Some(config)) => (StatusCode::OK, Json(config)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// POST /contracts/:id/feature-flags/sweep
// Auto-sunset all flags whose sunset_at has passed.
// Returns count of flags swept.
// ─────────────────────────────────────────────────────────

pub async fn sweep_expired(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE feature_flags
        SET state = 'sunset', updated_at = NOW()
        WHERE contract_id = $1
          AND state != 'sunset'
          AND sunset_at IS NOT NULL
          AND sunset_at <= $2
        "#,
    )
    .bind(contract_id)
    .bind(now)
    .execute(&state.db)
    .await;

    match result {
        Err(e) => {
            tracing::error!("sweep_expired DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(r) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "swept": r.rows_affected(),
                "swept_at": now,
            })),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/feature-flags/:name/check
// Check if a flag is enabled for a given user address.
// Used by clients before calling experimental contract functions.
// ─────────────────────────────────────────────────────────

pub async fn check_enabled(
    State(state): State<AppState>,
    Path((contract_id, name)): Path<(Uuid, String)>,
    Query(params): Query<CheckEnabledParams>,
) -> impl IntoResponse {
    let flag = sqlx::query_as::<_, FeatureFlag>(
        "SELECT * FROM feature_flags WHERE contract_id = $1 AND name = $2",
    )
    .bind(contract_id)
    .bind(&name)
    .fetch_optional(&state.db)
    .await;

    let flag = match flag {
        Err(e) => {
            tracing::error!("check_enabled DB error: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response();
        }
        // Unknown flags → disabled (backward compatible)
        Ok(None) => {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "flag": name,
                    "enabled": false,
                    "reason": "flag_not_found"
                })),
            )
                .into_response();
        }
        Ok(Some(f)) => f,
    };

    // Check auto-sunset
    if let Some(sunset_at) = flag.sunset_at {
        if Utc::now() >= sunset_at {
            // Auto-sunset in DB (fire-and-forget)
            let _ = sqlx::query(
                "UPDATE feature_flags SET state = 'sunset', updated_at = NOW() WHERE id = $1 AND state != 'sunset'",
            )
            .bind(flag.id)
            .execute(&state.db)
            .await;

            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "flag": name,
                    "enabled": false,
                    "reason": "flag_expired"
                })),
            )
                .into_response();
        }
    }

    if flag.state == "inactive" || flag.state == "sunset" {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "flag": name,
                "enabled": false,
                "reason": flag.state
            })),
        )
            .into_response();
    }

    // Apply rollout: deterministic hash of user address
    let enabled = match flag.rollout_strategy.as_str() {
        "full" => true,
        "gradual" => {
            let pct = flag.rollout_percentage as u64;
            if let Some(ref user) = params.user {
                // Simple deterministic hash: sum of bytes mod 100
                let bucket: u64 = user
                    .as_bytes()
                    .iter()
                    .fold(0u64, |acc, &b| acc.wrapping_add(b as u64))
                    % 100;
                bucket < pct
            } else {
                false // No user provided — treat as not in rollout
            }
        }
        _ => false,
    };

    // Record this check in analytics (fire-and-forget)
    let _ = sqlx::query(
        r#"
        INSERT INTO feature_flag_analytics
            (id, flag_id, total_checks, enabled_hits, disabled_hits,
             first_check_at, last_check_at, unique_users_approx)
        SELECT
            gen_random_uuid(), ff.id,
            1,
            CASE WHEN $3 THEN 1 ELSE 0 END,
            CASE WHEN $3 THEN 0 ELSE 1 END,
            NOW(), NOW(),
            1
        FROM feature_flags ff
        WHERE ff.contract_id = $1 AND ff.name = $2
        ON CONFLICT (flag_id) DO UPDATE SET
            total_checks       = feature_flag_analytics.total_checks + 1,
            enabled_hits       = feature_flag_analytics.enabled_hits  + CASE WHEN $3 THEN 1 ELSE 0 END,
            disabled_hits      = feature_flag_analytics.disabled_hits + CASE WHEN $3 THEN 0 ELSE 1 END,
            last_check_at      = NOW(),
            unique_users_approx = feature_flag_analytics.unique_users_approx + 1
        "#,
    )
    .bind(contract_id)
    .bind(&name)
    .bind(enabled)
    .execute(&state.db)
    .await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "flag":     name,
            "enabled":  enabled,
            "strategy": flag.rollout_strategy,
            "pct":      flag.rollout_percentage,
            "reason":   if enabled { "active" } else { "not_in_rollout" }
        })),
    )
        .into_response()
}

// ─────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────

/// Generic state transition with conflict guard.
async fn transition_flag_state(
    state: &AppState,
    contract_id: Uuid,
    name: &str,
    from_state: &str,
    to_state: &str,
    conflict_error: &str,
) -> axum::response::Response {
    let result = sqlx::query_as::<_, FeatureFlag>(
        r#"
        UPDATE feature_flags
        SET state = $4, updated_at = NOW()
        WHERE contract_id = $1 AND name = $2 AND state = $3
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(name)
    .bind(from_state)
    .bind(to_state)
    .fetch_optional(&state.db)
    .await;

    match result {
        Err(e) => {
            tracing::error!("transition_flag_state DB error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": conflict_error,
                "message": format!(
                    "Flag '{}' is not in '{}' state or does not exist",
                    name, from_state
                )
            })),
        )
            .into_response(),
        Ok(Some(flag)) => (StatusCode::OK, Json(flag)).into_response(),
    }
}