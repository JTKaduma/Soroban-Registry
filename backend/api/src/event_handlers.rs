use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shared::{ContractEvent, EventStats, IndexEventRequest};
use sqlx::Row;

use crate::{error::{ApiError, ApiResult}, state::AppState};

fn db_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation, error = ?err, "database operation failed");
    ApiError::internal("Database operation failed")
}

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub topic: Option<String>,
    pub data_pattern: Option<String>,
    pub from_timestamp: Option<String>,
    pub to_timestamp: Option<String>,
    pub from_ledger: Option<i64>,
    pub to_ledger: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_contract_events(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(query): Query<EventQuery>,
) -> ApiResult<Json<Vec<ContractEvent>>> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let mut sql = String::from(
        "SELECT id, contract_id, topic, data, ledger_sequence, transaction_hash, \
         timestamp, network, created_at FROM contract_events WHERE contract_id = $1"
    );
    let mut param_count = 1;

    let mut bindings: Vec<String> = vec![contract_id.clone()];

    if let Some(ref topic) = query.topic {
        param_count += 1;
        sql.push_str(&format!(" AND topic = ${}", param_count));
        bindings.push(topic.clone());
    }

    if let Some(from_ledger) = query.from_ledger {
        param_count += 1;
        sql.push_str(&format!(" AND ledger_sequence >= ${}", param_count));
        bindings.push(from_ledger.to_string());
    }

    if let Some(to_ledger) = query.to_ledger {
        param_count += 1;
        sql.push_str(&format!(" AND ledger_sequence <= ${}", param_count));
        bindings.push(to_ledger.to_string());
    }

    if let Some(ref from_ts) = query.from_timestamp {
        if let Ok(dt) = from_ts.parse::<DateTime<Utc>>() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
            bindings.push(dt.to_rfc3339());
        }
    }

    if let Some(ref to_ts) = query.to_timestamp {
        if let Ok(dt) = to_ts.parse::<DateTime<Utc>>() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp <= ${}", param_count));
            bindings.push(dt.to_rfc3339());
        }
    }

    param_count += 1;
    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_count));
    bindings.push(limit.to_string());

    param_count += 1;
    sql.push_str(&format!(" OFFSET ${}", param_count));
    bindings.push(offset.to_string());

    let mut query_builder = sqlx::query_as::<_, ContractEvent>(&sql);
    for binding in bindings {
        query_builder = query_builder.bind(binding);
    }

    let events = query_builder
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_error("fetch events", e))?;

    tracing::info!(
        contract_id = %contract_id,
        count = events.len(),
        "fetched contract events"
    );

    Ok(Json(events))
}

pub async fn get_event_stats(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<EventStats>> {
    let total_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_events WHERE contract_id = $1"
    )
    .bind(&contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_error("count events", e))?;

    let unique_topics: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT topic) FROM contract_events WHERE contract_id = $1"
    )
    .bind(&contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_error("count unique topics", e))?;

    let first_event: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MIN(timestamp) FROM contract_events WHERE contract_id = $1"
    )
    .bind(&contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_error("get first event", e))?;

    let last_event: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MAX(timestamp) FROM contract_events WHERE contract_id = $1"
    )
    .bind(&contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_error("get last event", e))?;

    let events_by_topic: serde_json::Value = serde_json::to_value(
        sqlx::query(
            "SELECT topic, COUNT(*) as count FROM contract_events \
             WHERE contract_id = $1 GROUP BY topic ORDER BY count DESC LIMIT 20"
        )
        .bind(&contract_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_error("get events by topic", e))?
        .iter()
        .filter_map(|row| {
            let topic: String = row.try_get("topic").ok()?;
            let count: i64 = row.try_get("count").ok()?;
            Some((topic, count))
        })
        .collect::<std::collections::HashMap<String, i64>>()
    ).unwrap_or(serde_json::json!({}));

    Ok(Json(EventStats {
        contract_id,
        total_events,
        unique_topics,
        first_event,
        last_event,
        events_by_topic,
    }))
}

pub async fn export_events_csv(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(query): Query<EventQuery>,
) -> ApiResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(10000).min(100000);

    let events = sqlx::query_as::<_, ContractEvent>(
        "SELECT id, contract_id, topic, data, ledger_sequence, transaction_hash, \
         timestamp, network, created_at FROM contract_events \
         WHERE contract_id = $1 \
         ORDER BY timestamp DESC LIMIT $2"
    )
    .bind(&contract_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_error("export events", e))?;

    let mut csv = String::from("id,contract_id,topic,data,ledger_sequence,transaction_hash,timestamp,network,created_at\n");

    for event in &events {
        let data_str = event.data
            .as_ref()
            .map(|d| serde_json::to_string(d).unwrap_or_default())
            .unwrap_or_default();
        
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            event.id,
            event.contract_id,
            event.topic,
            data_str.replace("\"", "\"\""),
            event.ledger_sequence,
            event.transaction_hash.as_deref().unwrap_or(""),
            event.timestamp.to_rfc3339(),
            event.network.to_string().to_lowercase(),
            event.created_at.to_rfc3339()
        ));
    }

    let filename = format!("events_{}_{}.csv", contract_id, chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .body(axum::body::Body::from(csv))
        .map_err(|_| ApiError::internal("Failed to build response"))
        .map(IntoResponse::into_response)
}

pub async fn index_event(
    State(state): State<AppState>,
    Json(event): Json<IndexEventRequest>,
) -> ApiResult<Json<ContractEvent>> {
    let created_event = sqlx::query_as::<_, ContractEvent>(
        "INSERT INTO contract_events (contract_id, topic, data, ledger_sequence, transaction_hash, network) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, contract_id, topic, data, ledger_sequence, transaction_hash, timestamp, network, created_at"
    )
    .bind(&event.contract_id)
    .bind(&event.topic)
    .bind(&event.data)
    .bind(event.ledger_sequence)
    .bind(&event.transaction_hash)
    .bind(&event.network)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_error("index event", e))?;

    tracing::info!(
        contract_id = %event.contract_id,
        topic = %event.topic,
        ledger = event.ledger_sequence,
        "indexed contract event"
    );

    Ok(Json(created_event))
}

pub async fn index_events_batch(
    State(state): State<AppState>,
    Json(events): Json<Vec<IndexEventRequest>>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut inserted = 0u64;
    let mut errors = 0u64;

    for event in events {
        let result = sqlx::query(
            "INSERT INTO contract_events (contract_id, topic, data, ledger_sequence, transaction_hash, network) \
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(&event.contract_id)
        .bind(&event.topic)
        .bind(&event.data)
        .bind(event.ledger_sequence)
        .bind(&event.transaction_hash)
        .bind(&event.network)
        .execute(&state.db)
        .await;

        match result {
            Ok(_) => inserted += 1,
            Err(e) => {
                tracing::warn!(error = ?e, "failed to insert event");
                errors += 1;
            }
        }
    }

    tracing::info!(inserted, errors, "batch event indexing complete");

    Ok(Json(serde_json::json!({
        "inserted": inserted,
        "errors": errors,
        "total": inserted + errors
    })))
}
