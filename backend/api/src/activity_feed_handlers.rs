//! Activity feed handler — GET /api/activity-feed
//!
//! Implements cursor-based pagination so clients can scroll through
//! analytics events without the bugs described in issue #337:
//!
//!  • total  = real COUNT(*), not entries.len()
//!  • page   = removed in favour of next_cursor
//!  • next_cursor is the created_at of the last returned entry

use axum::{
    extract::{Query, State},
    Json,
};
use shared::{ActivityFeedParams, AnalyticsEvent, CursorPaginatedResponse};

use crate::{error::AppError, state::AppState};

/// GET /api/activity-feed
///
/// Query params (all optional):
///   cursor     – ISO-8601 timestamp (from previous response's next_cursor)
///   limit      – page size, default 20, capped at 100
///   event_type – filter to a single event type
pub async fn get_activity_feed(
    State(state): State<AppState>,
    Query(mut params): Query<ActivityFeedParams>,
) -> Result<Json<CursorPaginatedResponse<AnalyticsEvent>>, AppError> {
    // ── Clamp limit to a safe maximum ────────────────────────────────────
    if params.limit > 100 {
        params.limit = 100;
    }

    // ── 1. Real COUNT(*) with the same filters ────────────────────────────
    //
    // Mirror the WHERE clause below exactly so total always matches what
    // the SELECT returns.
    let total: i64 = match &params.event_type {
        Some(et) => {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                FROM   analytics_events
                WHERE  ($1::timestamptz IS NULL OR created_at < $1)
                AND    event_type = $2
                "#,
            )
            .bind(params.cursor)
            .bind(et)
            .fetch_one(&state.db)
            .await?
        }
        None => {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                FROM   analytics_events
                WHERE  ($1::timestamptz IS NULL OR created_at < $1)
                "#,
            )
            .bind(params.cursor)
            .fetch_one(&state.db)
            .await?
        }
    };

    // ── 2. Fetch the page ─────────────────────────────────────────────────
    let entries: Vec<AnalyticsEvent> = match &params.event_type {
        Some(et) => {
            sqlx::query_as(
                r#"
                SELECT id, event_type, contract_id, user_address,
                       network, metadata, created_at
                FROM   analytics_events
                WHERE  ($1::timestamptz IS NULL OR created_at < $1)
                AND    event_type = $2
                ORDER  BY created_at DESC
                LIMIT  $3
                "#,
            )
            .bind(params.cursor)
            .bind(et)
            .bind(params.limit)
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as(
                r#"
                SELECT id, event_type, contract_id, user_address,
                       network, metadata, created_at
                FROM   analytics_events
                WHERE  ($1::timestamptz IS NULL OR created_at < $1)
                ORDER  BY created_at DESC
                LIMIT  $2
                "#,
            )
            .bind(params.cursor)
            .bind(params.limit)
            .fetch_all(&state.db)
            .await?
        }
    };

    // ── 3. Compute next_cursor from the oldest entry on this page ─────────
    let next_cursor = entries.last().map(|e| e.created_at);

    Ok(Json(CursorPaginatedResponse::new(
        entries,
        total,
        params.limit,
        next_cursor,
    )))
}