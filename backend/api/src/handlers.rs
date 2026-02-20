pub mod migrations;

use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use shared::{
    Contract, ContractSearchParams, ContractVersion, PaginatedResponse, PublishRequest, Publisher,
    VerifyRequest,
};
use std::time::Duration;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    resource_tracking::ResourceUsage,
    state::AppState,
};

pub fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn map_json_rejection(err: JsonRejection) -> ApiError {
    ApiError::bad_request(
        "InvalidRequest",
        format!("Invalid JSON payload: {}", err.body_text()),
    )
}

fn map_query_rejection(err: QueryRejection) -> ApiError {
    ApiError::bad_request(
        "InvalidQuery",
        format!("Invalid query parameters: {}", err.body_text()),
    )
}

pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();
    if db_ok {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "version": "0.1.0",
                "timestamp": now,
                "uptime_secs": uptime
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "degraded",
                "version": "0.1.0",
                "timestamp": now,
                "uptime_secs": uptime
            })),
        )
    }
}

pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count contracts", err))?;

    let verified_contracts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE is_verified = true")
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("count verified contracts", err))?;

    let total_publishers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publishers")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count publishers", err))?;

    Ok(Json(serde_json::json!({
        "total_contracts": total_contracts,
        "verified_contracts": verified_contracts,
        "total_publishers": total_publishers
    })))
}

pub async fn list_contracts(
    State(state): State<AppState>,
    params: Result<Query<ContractSearchParams>, QueryRejection>,
) -> axum::response::Response {
    let Query(params) = match params {
        Ok(q) => q,
        Err(err) => return map_query_rejection(err).into_response(),
    };
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).max(0) * limit;

    let contracts: Vec<Contract> = match sqlx::query_as(
        "SELECT * FROM contracts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(err) => return db_internal_error("list contracts", err).into_response(),
    };
    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
    {
        Ok(v) => v,
        Err(err) => return db_internal_error("count contracts", err).into_response(),
    };
    (StatusCode::OK, Json(PaginatedResponse::new(contracts, total, page, limit))).into_response()
}

pub async fn get_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Contract>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;
    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("get contract", err),
        })?;
    Ok(Json(contract))
}

pub async fn get_contract_abi(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let abi: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT abi FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    abi.map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn get_contract_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ContractVersion>>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;
    let versions: Vec<ContractVersion> = sqlx::query_as(
        "SELECT * FROM contract_versions WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list contract versions", err))?;
    Ok(Json(versions))
}

pub async fn publish_contract(
    State(state): State<AppState>,
    payload: Result<Json<PublishRequest>, JsonRejection>,
) -> ApiResult<Json<Contract>> {
    let Json(req) = payload.map_err(map_json_rejection)?;
    let publisher: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address) VALUES ($1)
         ON CONFLICT (stellar_address) DO UPDATE SET stellar_address = EXCLUDED.stellar_address
         RETURNING *",
    )
    .bind(&req.publisher_address)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("upsert publisher", err))?;
    let wasm_hash = "placeholder_hash".to_string();
    let contract: Contract = sqlx::query_as(
        "INSERT INTO contracts (contract_id, wasm_hash, name, description, publisher_id, network, category, tags)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING *",
    )
    .bind(&req.contract_id)
    .bind(&wasm_hash)
    .bind(&req.name)
    .bind(&req.description)
    .bind(publisher.id)
    .bind(&req.network)
    .bind(&req.category)
    .bind(&req.tags)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create contract", err))?;
    Ok(Json(contract))
}

pub async fn verify_contract(
    State(_state): State<AppState>,
    payload: Result<Json<VerifyRequest>, JsonRejection>,
) -> ApiResult<Json<serde_json::Value>> {
    let Json(_req) = payload.map_err(map_json_rejection)?;
    Ok(Json(serde_json::json!({
        "status": "pending",
        "message": "Verification started"
    })))
}

pub async fn create_publisher(
    State(state): State<AppState>,
    payload: Result<Json<Publisher>, JsonRejection>,
) -> ApiResult<Json<Publisher>> {
    let Json(publisher) = payload.map_err(map_json_rejection)?;
    let created: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address, username, email, github_url, website)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(&publisher.stellar_address)
    .bind(&publisher.username)
    .bind(&publisher.email)
    .bind(&publisher.github_url)
    .bind(&publisher.website)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create publisher", err))?;
    Ok(Json(created))
}

pub async fn get_publisher(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Publisher>> {
    let publisher: Publisher = sqlx::query_as("SELECT * FROM publishers WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "PublisherNotFound",
                format!("No publisher found with ID: {}", id),
            ),
            _ => db_internal_error("get publisher", err),
        })?;
    Ok(Json(publisher))
}

pub async fn get_publisher_contracts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<Contract>>> {
    let contracts: Vec<Contract> =
        sqlx::query_as("SELECT * FROM contracts WHERE publisher_id = $1 ORDER BY created_at DESC")
            .bind(id)
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("list publisher contracts", err))?;
    Ok(Json(contracts))
}

#[derive(Deserialize)]
pub struct CacheParams {
    pub cache: Option<String>,
}

pub async fn get_contract_state(
    State(state): State<AppState>,
    Path((contract_id, key)): Path<(String, String)>,
    Query(params): Query<CacheParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let use_cache = params.cache.as_deref() == Some("on");
    if use_cache {
        let (cached_value, was_hit) = state.cache.get(&contract_id, &key).await;
        if was_hit && cached_value.is_some() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cached_value.unwrap()) {
                return Ok(Json(val));
            }
        }
    }
    let fetch_start = std::time::Instant::now();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let fetch_duration = fetch_start.elapsed();
    let value = serde_json::json!({
        "contract_id": contract_id,
        "key": key,
        "value": &format!("state_of_{}_{}", contract_id, key),
        "fetched_at": &chrono::Utc::now().to_rfc3339()
    });
    if use_cache {
        state
            .cache
            .put(&contract_id, &key, value.to_string(), None)
            .await;
    } else {
        state.cache.record_uncached_latency(fetch_duration);
    }
    Ok(Json(value))
}

pub async fn update_contract_state(
    State(state): State<AppState>,
    Path((contract_id, key)): Path<(String, String)>,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    tokio::time::sleep(Duration::from_millis(200)).await;
    state.cache.invalidate(&contract_id, &key).await;
    let payload_size = payload.to_string().len() as u64;
    let cpu = 180_000 + payload_size.saturating_mul(90);
    let mem = 1_200_000 + payload_size.saturating_mul(64);
    let mut mgr = state.resource_mgr.write().unwrap();
    let _ = mgr.record_usage(
        &contract_id,
        ResourceUsage {
            cpu_instructions: cpu,
            mem_bytes: mem,
            storage_bytes: payload_size,
            timestamp: chrono::Utc::now(),
        },
    );
    Ok(Json(
        serde_json::json!({ "status": "updated", "invalidated": true }),
    ))
}

pub async fn get_cache_stats(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let metrics = state.cache.metrics();
    let hits = metrics.hits.load(std::sync::atomic::Ordering::Relaxed);
    let misses = metrics.misses.load(std::sync::atomic::Ordering::Relaxed);
    Ok(Json(serde_json::json!({
        "metrics": {
            "hit_rate_percent": metrics.hit_rate(),
            "avg_cached_hit_latency_us": metrics.avg_cached_hit_latency(),
            "avg_cache_miss_latency_us": metrics.avg_cache_miss_latency(),
            "avg_uncached_latency_us": metrics.avg_uncached_latency(),
            "improvement_factor": metrics.improvement_factor(),
            "hits": hits,
            "misses": misses
        },
        "config": {
            "enabled": state.cache.config().enabled,
            "ttl_seconds": state.cache.config().global_ttl.as_secs(),
            "max_capacity": state.cache.config().max_capacity
        }
    })))
}

pub async fn route_not_found() -> ApiError {
    ApiError::not_found("RouteNotFound", "The requested endpoint does not exist")
}
