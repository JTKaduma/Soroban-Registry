// backend/api/src/quality_handlers.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use sqlx::Row;

use shared::{
    CategoryBenchmark, ComputeQualityRequest, QualityBadge, QualityRecord,
    QualityResponse, QualityScoreBreakdown, QualityThreshold,
    SetThresholdRequest, ThresholdCheckResult, ThresholdViolation,
    // QualityWeights removed — only used via req.weights.unwrap_or_default(),
    // which is resolved through the type on ComputeQualityRequest, not a direct name here.
};

use crate::{
    quality_calculator::QualityCalculator,
    state::AppState,
};

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/quality
// Returns the most recent quality record for the contract.
// ─────────────────────────────────────────────────────────

pub async fn get_contract_quality(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    let record = sqlx::query_as::<_, QualityRecord>(
        r#"
        SELECT * FROM contract_quality
        WHERE contract_id = $1
        ORDER BY computed_at DESC
        LIMIT 1
        "#,
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await;

    match record {
        Err(e) => {
            tracing::error!("DB error fetching quality for {}: {:?}", contract_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "DB_ERROR",
                    "message": "Failed to fetch quality record"
                })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": "No quality record found. POST to /quality to compute one."
            })),
        )
            .into_response(),
        Ok(Some(record)) => {
            let code_metrics = serde_json::from_value(record.code_metrics.clone())
                .unwrap_or_default();
            let test_metrics = serde_json::from_value(record.test_metrics.clone())
                .unwrap_or_default();
            let doc_metrics = serde_json::from_value(record.doc_metrics.clone())
                .unwrap_or_default();
            let security_metrics = serde_json::from_value(record.security_metrics.clone())
                .unwrap_or_default();

            let breakdown = QualityScoreBreakdown {
                code_score: record.code_score,
                test_score: record.test_score,
                doc_score: record.doc_score,
                security_score: record.security_score,
                overall_score: record.overall_score,
            };

            let badge = QualityBadge::from_score(record.overall_score);
            let threshold_result =
                get_threshold_check(&state, contract_id, &breakdown).await;
            let benchmark =
                get_category_benchmark(&state, contract_id, record.overall_score).await;

            let response = QualityResponse {
                record,
                breakdown,
                code_metrics,
                test_metrics,
                doc_metrics,
                security_metrics,
                badge,
                threshold_result,
                benchmark,
            };

            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

// ─────────────────────────────────────────────────────────
// POST /contracts/:id/quality
// Triggers a fresh quality calculation from submitted source.
// ─────────────────────────────────────────────────────────

pub async fn compute_contract_quality(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<ComputeQualityRequest>,
) -> impl IntoResponse {
    let weights = req.weights.unwrap_or_default();
    if !weights.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_WEIGHTS",
                "message": "weights.code + weights.tests + weights.docs + weights.security must equal 1.0"
            })),
        )
            .into_response();
    }

    let (audit_score, critical, high, medium, low, is_verified, has_formal_audit) =
        if let Some(audit_id) = req.audit_id {
            fetch_audit_data(&state, audit_id).await
        } else {
            (50.0, 0, 0, 0, 0, false, false)
        };

    let metrics = QualityCalculator::compute(
        &req.source_code,
        req.test_output.as_deref(),
        audit_score,
        critical,
        high,
        medium,
        low,
        is_verified,
        has_formal_audit,
    );
    let breakdown = QualityCalculator::score(&metrics, &weights);
    let badge = QualityBadge::from_score(breakdown.overall_score);

    // FIX: use sqlx::query_as (non-macro) to avoid DATABASE_URL requirement at compile time.
    // The macro variant (query_as!) validates SQL against a live DB; the non-macro variant
    // defers validation to runtime, which is correct for projects without offline query cache.
    let record = sqlx::query_as::<_, QualityRecord>(
        r#"
        INSERT INTO contract_quality (
            id, contract_id, contract_version,
            code_metrics, test_metrics, doc_metrics, security_metrics,
            code_score, test_score, doc_score, security_score,
            overall_score, badge, computed_at
        ) VALUES (
            gen_random_uuid(), $1, $2,
            $3, $4, $5, $6,
            $7, $8, $9, $10,
            $11, $12, NOW()
        )
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(req.version)
    .bind(serde_json::to_value(&metrics.code).unwrap())
    .bind(serde_json::to_value(&metrics.tests).unwrap())
    .bind(serde_json::to_value(&metrics.docs).unwrap())
    .bind(serde_json::to_value(&metrics.security).unwrap())
    .bind(breakdown.code_score)
    .bind(breakdown.test_score)
    .bind(breakdown.doc_score)
    .bind(breakdown.security_score)
    .bind(breakdown.overall_score)
    .bind(badge.to_string())
    .fetch_one(&state.db)
    .await;

    match record {
        Err(e) => {
            tracing::error!("Failed to persist quality record: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "DB_ERROR",
                    "message": "Failed to save quality record"
                })),
            )
                .into_response()
        }
        Ok(record) => {
            let threshold_result =
                get_threshold_check(&state, contract_id, &breakdown).await;
            let benchmark =
                get_category_benchmark(&state, contract_id, breakdown.overall_score).await;

            let response = QualityResponse {
                record,
                breakdown,
                code_metrics: metrics.code,
                test_metrics: metrics.tests,
                doc_metrics: metrics.docs,
                security_metrics: metrics.security,
                badge,
                threshold_result,
                benchmark,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/quality/trend
// Returns all historical quality scores for charting.
// ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TrendParams {
    #[serde(default = "default_trend_limit")]
    pub limit: i64,
}
fn default_trend_limit() -> i64 {
    30
}

pub async fn get_quality_trend(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(params): Query<TrendParams>,
) -> impl IntoResponse {
    if params.limit < 1 || params.limit > 365 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_LIMIT",
                "message": "limit must be between 1 and 365"
            })),
        )
            .into_response();
    }

    let rows = sqlx::query(
        r#"
        SELECT
            id            AS quality_id,
            contract_version AS version,
            computed_at,
            overall_score,
            code_score,
            test_score,
            doc_score,
            security_score,
            badge
        FROM contract_quality
        WHERE contract_id = $1
        ORDER BY computed_at DESC
        LIMIT $2
        "#,
    )
    .bind(contract_id)
    .bind(params.limit)
    .fetch_all(&state.db)
    .await;

    match rows {
        Err(e) => {
            tracing::error!("Trend query error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(rows) => {
            // FIX: `points` does not need `mut` — collect directly, use len before move.
            let points: Vec<serde_json::Value> = rows
                .into_iter()
                .rev()
                .map(|r| {
                    serde_json::json!({
                        "quality_id":     r.get::<Uuid, _>("quality_id"),
                        "version":        r.get::<String, _>("version"),
                        "computed_at":    r.get::<chrono::DateTime<chrono::Utc>, _>("computed_at"),
                        "overall_score":  r.get::<f64, _>("overall_score"),
                        "code_score":     r.get::<f64, _>("code_score"),
                        "test_score":     r.get::<f64, _>("test_score"),
                        "doc_score":      r.get::<f64, _>("doc_score"),
                        "security_score": r.get::<f64, _>("security_score"),
                        "badge":          r.get::<String, _>("badge"),
                    })
                })
                .collect();

            let count = points.len();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "contract_id": contract_id,
                    "points": points,
                    "count": count,
                })),
            )
                .into_response()
        }
    }
}

// ─────────────────────────────────────────────────────────
// GET /contracts/:id/quality/benchmark
// Compare this contract to its category peers.
// ─────────────────────────────────────────────────────────

pub async fn get_quality_benchmark(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> impl IntoResponse {
    let latest: Result<Option<f64>, sqlx::Error> = sqlx::query_scalar(
        "SELECT overall_score FROM contract_quality WHERE contract_id = $1 ORDER BY computed_at DESC LIMIT 1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await;

    match latest {
        // FIX: bind error to `_e` to suppress unused-variable warning
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "DB_ERROR" })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "NOT_FOUND",
                "message": "Compute quality first via POST /quality"
            })),
        )
            .into_response(),
        Ok(Some(score)) => {
            let benchmark = get_category_benchmark(&state, contract_id, score).await;
            (StatusCode::OK, Json(benchmark)).into_response()
        }
    }
}

// ─────────────────────────────────────────────────────────
// POST /contracts/:id/quality/threshold
// Set minimum quality gates for this contract.
// ─────────────────────────────────────────────────────────

pub async fn set_quality_threshold(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<SetThresholdRequest>,
) -> impl IntoResponse {
    for (name, val) in [
        ("min_overall_score", req.min_overall_score),
        ("min_code_score", req.min_code_score),
        ("min_test_score", req.min_test_score),
        ("min_doc_score", req.min_doc_score),
        ("min_security_score", req.min_security_score),
    ] {
        if !(0.0..=100.0).contains(&val) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "INVALID_THRESHOLD",
                    "message": format!("{} must be between 0 and 100", name)
                })),
            )
                .into_response();
        }
    }

    // FIX: use sqlx::query_as (non-macro) — same reason as the INSERT above.
    // query_as! requires DATABASE_URL at compile time or an offline cache.
    // query_as::<_, T>(...) defers to runtime and compiles without either.
    let threshold = sqlx::query_as::<_, QualityThreshold>(
        r#"
        INSERT INTO quality_thresholds (
            id, contract_id,
            min_overall_score, min_code_score, min_test_score,
            min_doc_score, min_security_score,
            fail_on_critical_finding, created_by,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), $1,
            $2, $3, $4, $5, $6, $7, $8,
            NOW(), NOW()
        )
        ON CONFLICT (contract_id) DO UPDATE SET
            min_overall_score        = EXCLUDED.min_overall_score,
            min_code_score           = EXCLUDED.min_code_score,
            min_test_score           = EXCLUDED.min_test_score,
            min_doc_score            = EXCLUDED.min_doc_score,
            min_security_score       = EXCLUDED.min_security_score,
            fail_on_critical_finding = EXCLUDED.fail_on_critical_finding,
            created_by               = EXCLUDED.created_by,
            updated_at               = NOW()
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(req.min_overall_score)
    .bind(req.min_code_score)
    .bind(req.min_test_score)
    .bind(req.min_doc_score)
    .bind(req.min_security_score)
    .bind(req.fail_on_critical_finding)
    .bind(req.created_by)
    .fetch_one(&state.db)
    .await;

    match threshold {
        Err(e) => {
            tracing::error!("Failed to set threshold: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "DB_ERROR" })),
            )
                .into_response()
        }
        Ok(t) => (StatusCode::OK, Json(t)).into_response(),
    }
}

// ─────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────

async fn get_threshold_check(
    state: &AppState,
    contract_id: Uuid,
    breakdown: &QualityScoreBreakdown,
) -> Option<ThresholdCheckResult> {
    let threshold = sqlx::query_as::<_, QualityThreshold>(
        "SELECT * FROM quality_thresholds WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .ok()??;

    let mut violations = vec![];

    for (dim, required, actual) in [
        ("overall_score",  threshold.min_overall_score,  breakdown.overall_score),
        ("code_score",     threshold.min_code_score,     breakdown.code_score),
        ("test_score",     threshold.min_test_score,     breakdown.test_score),
        ("doc_score",      threshold.min_doc_score,      breakdown.doc_score),
        ("security_score", threshold.min_security_score, breakdown.security_score),
    ] {
        if actual < required {
            violations.push(ThresholdViolation {
                dimension: dim.to_string(),
                required,
                actual,
                gap: required - actual,
            });
        }
    }

    Some(ThresholdCheckResult {
        passed: violations.is_empty(),
        violations,
    })
}

async fn get_category_benchmark(
    state: &AppState,
    contract_id: Uuid,
    this_score: f64,
) -> Option<CategoryBenchmark> {
    let category: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT category FROM contracts WHERE id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .ok()?;

    let category = category?;

    // FIX: was `row` but then referenced as `stats` — rename binding to `row` and
    // extract each column via .get() so the type is clear at each call site.
    let row = sqlx::query(
        r#"
        SELECT
            COUNT(DISTINCT c.id)                   AS peer_count,
            AVG(q.overall_score)                   AS avg_score,
            PERCENTILE_CONT(0.25) WITHIN GROUP
                (ORDER BY q.overall_score)         AS p25_score,
            PERCENTILE_CONT(0.75) WITHIN GROUP
                (ORDER BY q.overall_score)         AS p75_score,
            PERCENTILE_CONT(0.95) WITHIN GROUP
                (ORDER BY q.overall_score)         AS p95_score,
            (COUNT(*) FILTER (WHERE q.overall_score < $2) * 100.0
                / NULLIF(COUNT(*), 0))             AS percentile_rank
        FROM contracts c
        JOIN LATERAL (
            SELECT overall_score FROM contract_quality
            WHERE contract_id = c.id
            ORDER BY computed_at DESC
            LIMIT 1
        ) q ON true
        WHERE c.category = $1
          AND c.id != $3
        "#,
    )
    .bind(&category)
    .bind(this_score)
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .ok()??;  // None → caller gets None; Err → same

    // FIX: extract columns from `row` (not the nonexistent `stats`)
    let peer_count:      i64 = row.try_get("peer_count").unwrap_or(0);
    let avg_score:       f64 = row.try_get("avg_score").unwrap_or(0.0);
    let p25_score:       f64 = row.try_get("p25_score").unwrap_or(0.0);
    let p75_score:       f64 = row.try_get("p75_score").unwrap_or(0.0);
    let p95_score:       f64 = row.try_get("p95_score").unwrap_or(0.0);
    let percentile_rank: f64 = row.try_get("percentile_rank").unwrap_or(0.0);

    Some(CategoryBenchmark {
        category,
        peer_count,
        category_avg_score: avg_score,
        category_p25_score: p25_score,
        category_p75_score: p75_score,
        category_p95_score: p95_score,
        this_contract_score: this_score,
        percentile_rank,
        above_average: this_score > avg_score,
    })
}

async fn fetch_audit_data(
    state: &AppState,
    audit_id: Uuid,
) -> (f64, i64, i64, i64, i64, bool, bool) {
    // FIX: use sqlx::query (non-macro) — avoids DATABASE_URL requirement.
    // The original query! macro was the second compile error in the build log.
    let row = sqlx::query(
        r#"
        SELECT
            a.overall_score,
            a.contract_id,
            COUNT(ac.id) FILTER (WHERE ac.status = 'failed' AND ci.severity = 'Critical') AS critical,
            COUNT(ac.id) FILTER (WHERE ac.status = 'failed' AND ci.severity = 'High')     AS high,
            COUNT(ac.id) FILTER (WHERE ac.status = 'failed' AND ci.severity = 'Medium')   AS medium,
            COUNT(ac.id) FILTER (WHERE ac.status = 'failed' AND ci.severity = 'Low')      AS low,
            c.is_verified
        FROM security_audits a
        JOIN contracts c ON c.id = a.contract_id
        LEFT JOIN audit_checks ac ON ac.audit_id = a.id
        LEFT JOIN checklist_items ci ON ci.id = ac.check_id
        WHERE a.id = $1
        GROUP BY a.id, a.overall_score, a.contract_id, c.is_verified
        "#,
    )
    .bind(audit_id)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(r)) => (
            r.try_get("overall_score").unwrap_or(50.0),
            r.try_get("critical").unwrap_or(0),
            r.try_get("high").unwrap_or(0),
            r.try_get("medium").unwrap_or(0),
            r.try_get("low").unwrap_or(0),
            r.try_get("is_verified").unwrap_or(false),
            true,
        ),
        _ => (50.0, 0, 0, 0, 0, false, false),
    }
}