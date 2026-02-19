use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    Json,
};
use shared::{
    Contract, ContractSearchParams, ContractVersion, PaginatedResponse, PublishRequest, Publisher,
    VerifyRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn map_json_rejection(err: JsonRejection) -> ApiError {
    ApiError::bad_request("InvalidRequest", format!("Invalid JSON payload: {}", err.body_text()))
}

fn map_query_rejection(err: QueryRejection) -> ApiError {
    ApiError::bad_request("InvalidQuery", format!("Invalid query parameters: {}", err.body_text()))
}

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}

/// Get registry statistics
pub async fn get_stats(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
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
        "total_publishers": total_publishers,
    })))
}

/// List and search contracts
pub async fn list_contracts(
    State(state): State<AppState>,
    params: Result<Query<ContractSearchParams>, QueryRejection>,
) -> ApiResult<Json<PaginatedResponse<Contract>>> {
    let Query(params) = params.map_err(map_query_rejection)?;
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    // Build dynamic query based on filters
    let mut query = String::from("SELECT * FROM contracts WHERE 1=1");
    let mut count_query = String::from("SELECT COUNT(*) FROM contracts WHERE 1=1");

    if let Some(ref q) = params.query {
        let search_clause = format!(
            " AND (name ILIKE '%{}%' OR description ILIKE '%{}%')",
            q, q
        );
        query.push_str(&search_clause);
        count_query.push_str(&search_clause);
    }

    if let Some(verified) = params.verified_only {
        if verified {
            query.push_str(" AND is_verified = true");
            count_query.push_str(" AND is_verified = true");
        }
    }

    if let Some(ref category) = params.category {
        let category_clause = format!(" AND category = '{}'", category);
        query.push_str(&category_clause);
        count_query.push_str(&category_clause);
    }

    query.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", page_size, offset));

    let contracts: Vec<Contract> = sqlx::query_as(&query)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("list contracts", err))?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count filtered contracts", err))?;

    Ok(Json(PaginatedResponse::new(contracts, total, page, page_size)))
}

/// Get a specific contract by ID
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
            _ => db_internal_error("get contract by id", err),
        })?;

    Ok(Json(contract))
}

/// Get contract version history
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
    .map_err(|err| db_internal_error("get contract versions", err))?;

    Ok(Json(versions))
}

/// Publish a new contract
pub async fn publish_contract(
    State(state): State<AppState>,
    payload: Result<Json<PublishRequest>, JsonRejection>,
) -> ApiResult<Json<Contract>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    // First, ensure publisher exists or create one
    let publisher: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address) VALUES ($1)
         ON CONFLICT (stellar_address) DO UPDATE SET stellar_address = EXCLUDED.stellar_address
         RETURNING *",
    )
    .bind(&req.publisher_address)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("upsert publisher", err))?;

    // TODO: Fetch WASM hash from Stellar network
    let wasm_hash = "placeholder_hash".to_string();

    // Insert contract
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

/// Verify a contract
pub async fn verify_contract(
    State(_state): State<AppState>,
    payload: Result<Json<VerifyRequest>, JsonRejection>,
) -> ApiResult<Json<serde_json::Value>> {
    let Json(_req) = payload.map_err(map_json_rejection)?;

    // TODO: Implement verification logic
    Ok(Json(serde_json::json!({
        "status": "pending",
        "message": "Verification started"
    })))
}

/// Create a publisher
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

/// Get publisher by ID
pub async fn get_publisher(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Publisher>> {
    let publisher_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidPublisherId",
            format!("Invalid publisher ID format: {}", id),
        )
    })?;

    let publisher: Publisher = sqlx::query_as("SELECT * FROM publishers WHERE id = $1")
        .bind(publisher_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "PublisherNotFound",
                format!("No publisher found with ID: {}", id),
            ),
            _ => db_internal_error("get publisher by id", err),
        })?;

    Ok(Json(publisher))
}

/// Get all contracts by a publisher
pub async fn get_publisher_contracts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<Contract>>> {
    let publisher_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidPublisherId",
            format!("Invalid publisher ID format: {}", id),
        )
    })?;

    let contracts: Vec<Contract> = sqlx::query_as(
        "SELECT * FROM contracts WHERE publisher_id = $1 ORDER BY created_at DESC",
    )
    .bind(publisher_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get publisher contracts", err))?;

    Ok(Json(contracts))
}

/// Fallback endpoint for unknown routes
pub async fn route_not_found() -> ApiError {
    ApiError::not_found("RouteNotFound", "The requested endpoint does not exist")
}
