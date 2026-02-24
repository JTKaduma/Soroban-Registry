use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shared::{ActivityFeedEntry, AnalyticsEventType, Network, PaginatedResponse};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ActivityFeedQuery {
    #[serde(default)]
    pub event_types: Vec<AnalyticsEventType>,
    pub network: Option<Network>,
    pub publisher_id: Option<Uuid>,
    pub days: Option<i64>,
    pub cursor: Option<DateTime<Utc>>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn get_activity_feed(
    State(state): State<AppState>,
    Query(query): Query<ActivityFeedQuery>,
) -> ApiResult<Json<PaginatedResponse<ActivityFeedEntry>>> {
    let limit = query.limit.clamp(1, 100);
    let days = query.days.unwrap_or(7).clamp(1, 365);
    let start_time = Utc::now() - chrono::Duration::days(days);

    let mut sql = String::from(
        r#"
        SELECT 
            ae.id,
            ae.event_type as "event_type: AnalyticsEventType",
            ae.contract_id,
            c.name as contract_name,
            c.contract_id as contract_stellar_id,
            ae.publisher_id,
            p.username as publisher_name,
            ae.network as "network: Network",
            ae.metadata,
            ae.created_at
        FROM analytics_events ae
        LEFT JOIN contracts c ON ae.contract_id = c.id
        LEFT JOIN publishers p ON ae.publisher_id = p.id
        WHERE ae.created_at >= $1
        "#,
    );

    let mut bind_index = 2; // $1 is start_time

    if !query.event_types.is_empty() {
        sql.push_str(&format!(" AND ae.event_type = ANY(${})", bind_index));
        bind_index += 1;
    }

    if let Some(ref net) = query.network {
        sql.push_str(&format!(" AND ae.network = ${}", bind_index));
        bind_index += 1;
    }

    if let Some(pub_id) = query.publisher_id {
        sql.push_str(&format!(" AND ae.publisher_id = ${}", bind_index));
        bind_index += 1;
    }

    if let Some(cursor) = query.cursor {
        sql.push_str(&format!(" AND ae.created_at < ${}", bind_index));
        bind_index += 1;
    }

    sql.push_str(&format!(
        " ORDER BY ae.created_at DESC LIMIT ${}",
        bind_index
    ));

    let mut db_query = sqlx::query_as::<_, ActivityFeedEntry>(&sql).bind(start_time);

    if !query.event_types.is_empty() {
        // sqlx doesn't handle Vec<Enum> directly well for ANY, might need string conversion
        let types: Vec<String> = query.event_types.iter().map(|t| t.to_string()).collect();
        db_query = db_query.bind(types);
    }

    if let Some(ref net) = query.network {
        db_query = db_query.bind(net);
    }

    if let Some(pub_id) = query.publisher_id {
        db_query = db_query.bind(pub_id);
    }

    if let Some(cursor) = query.cursor {
        db_query = db_query.bind(cursor);
    }

    db_query = db_query.bind(limit);

    let entries = db_query
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch activity feed", err))?;

    // For simplicity in this demo, we return a fixed total of 0 or entries.len()
    // Real cursor-based pagination usually doesn't return total count unless requested separately
    let total = entries.len() as i64;

    Ok(Json(PaginatedResponse::new(entries, total, 1, limit)))
}
