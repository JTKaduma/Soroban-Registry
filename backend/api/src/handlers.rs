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
use shared::{
    Contract, ContractDeployment, ContractSearchParams, ContractVersion, DeployGreenRequest,
    DeploymentEnvironment, DeploymentStatus, DeploymentSwitch, HealthCheckRequest,
    PaginatedResponse, PublishRequest, Publisher, SwitchDeploymentRequest, VerifyRequest,
};
use uuid::Uuid;

use crate::{
    analytics,
    error::{ApiError, ApiResult},
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

/// Health check — probes DB connectivity and reports uptime.
/// Returns 200 when everything is reachable, 503 when the database
/// connection pool cannot satisfy a trivial query.
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();

    // Quick connectivity probe — keeps the query as cheap as possible
    // so that frequent polling from orchestrators doesn't add load.
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    if db_ok {
        tracing::info!(uptime_secs = uptime, "health check passed");

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
        tracing::warn!(
            uptime_secs = uptime,
            "health check degraded — db unreachable"
        );

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

/// Get registry statistics
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
        "total_publishers": total_publishers,
    })))
}

/// List and search contracts
pub async fn list_contracts(
    State(state): State<AppState>,
    params: Result<Query<ContractSearchParams>, QueryRejection>,
) -> axum::response::Response {
    let Query(params) = match params {
        Ok(q) => q,
        Err(err) => return map_query_rejection(err).into_response(),
    };

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);

    // bad input, bail early
    if page < 1 || limit < 1 || limit > 100 {
        return ApiError::bad_request(
            "InvalidPagination",
            "page must be >= 1 and limit must be between 1 and 100",
        )
        .into_response();
    }

    let offset = (page - 1) * limit;

    // Build dynamic query based on filters
    let mut query = String::from("SELECT * FROM contracts WHERE 1=1");
    let mut count_query = String::from("SELECT COUNT(*) FROM contracts WHERE 1=1");

    if let Some(ref q) = params.query {
        let search_clause = format!(" AND (name ILIKE '%{}%' OR description ILIKE '%{}%')", q, q);
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

    query.push_str(&format!(
        " ORDER BY created_at DESC LIMIT {} OFFSET {}",
        limit, offset
    ));

    let contracts: Vec<Contract> = match sqlx::query_as(&query).fetch_all(&state.db).await {
        Ok(rows) => rows,
        Err(err) => return db_internal_error("list contracts", err).into_response(),
    };

    let total: i64 = match sqlx::query_scalar(&count_query).fetch_one(&state.db).await {
        Ok(n) => n,
        Err(err) => return db_internal_error("count filtered contracts", err).into_response(),
    };

    let paginated = PaginatedResponse::new(contracts, total, page, limit);

    // link headers for pagination
    let total_pages = paginated.total_pages;
    let mut links: Vec<String> = Vec::new();

    if page > 1 {
        links.push(format!(
            "</api/contracts?page={}&limit={}>; rel=\"prev\"",
            page - 1,
            limit
        ));
    }
    if page < total_pages {
        links.push(format!(
            "</api/contracts?page={}&limit={}>; rel=\"next\"",
            page + 1,
            limit
        ));
    }

    let mut response = (StatusCode::OK, Json(paginated)).into_response();

    if !links.is_empty() {
        if let Ok(value) = axum::http::HeaderValue::from_str(&links.join(", ")) {
            response.headers_mut().insert("link", value);
        }
    }

    response
    Ok(Json(PaginatedResponse::new(
        contracts, total, page, page_size,
    )))
}

pub async fn get_contract(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Contract>> {
    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("get contract by id", err),
        })?;

    let active_deployment: Option<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments 
         WHERE contract_id = $1 AND status = 'active'",
    )
    .bind(contract.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("get active deployment", err))?;

    if let Some(deployment) = active_deployment {
        let mut contract_with_deployment = contract.clone();
        contract_with_deployment.wasm_hash = deployment.wasm_hash;
        Ok(Json(contract_with_deployment))
    } else {
    Ok(Json(contract))
    }
}

/// Get contract ABI
pub async fn get_contract_abi(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let abi: Option<serde_json::Value> = sqlx::query_scalar("SELECT abi FROM contracts WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    abi.map(Json).ok_or(StatusCode::NOT_FOUND)
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
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list versions", err))?;

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

    // Fire-and-forget analytics event
    let pool = state.db.clone();
    let cid = contract.id;
    let addr = req.publisher_address.clone();
    let net = contract.network.clone();
    tokio::spawn(async move {
        if let Err(err) = analytics::record_event(
            &pool,
            AnalyticsEventType::ContractPublished,
            cid,
            Some(&addr),
            Some(&net),
            None,
        )
        .await
        {
            tracing::warn!(error = ?err, "failed to record contract_published event");
        }
    });
    sqlx::query(
        "INSERT INTO contract_deployments (contract_id, environment, status, wasm_hash, activated_at)
         VALUES ($1, 'blue', 'active', $2, NOW())
         ON CONFLICT (contract_id, environment) DO NOTHING",
    )
    .bind(contract.id)
    .bind(&wasm_hash)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("create initial blue deployment", err))?;

    // Fire-and-forget audit log entry + initial snapshot (version 1)
    {
        let pool = state.db.clone();
        let cid = contract.id;
        let publisher_addr = req.publisher_address.clone();
        let snap = serde_json::to_value(&contract).unwrap_or_default();
        tokio::spawn(async move {
            if let Err(err) = crate::contract_history_handlers::log_contract_change(
                &pool,
                cid,
                shared::AuditActionType::ContractPublished,
                None,
                Some(snap),
                &publisher_addr,
            )
            .await
            {
                tracing::warn!(error = ?err, "failed to write initial audit log entry");
            }
        });
    }

    Ok(Json(contract))
}

/// Verify a contract
pub async fn verify_contract(
    State(state): State<AppState>,
    payload: Result<Json<VerifyRequest>, JsonRejection>,
) -> ApiResult<Json<serde_json::Value>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    // TODO: Implement full verification logic

    // Fire-and-forget analytics event
    // We parse the contract_id string as UUID for the event; if it fails we skip.
    if let Ok(cid) = Uuid::parse_str(&req.contract_id) {
        let pool = state.db.clone();
        tokio::spawn(async move {
            if let Err(err) = analytics::record_event(
                &pool,
                AnalyticsEventType::ContractVerified,
                cid,
                None,
                None,
                Some(serde_json::json!({ "compiler_version": req.compiler_version })),
            )
            .await
            {
                tracing::warn!(error = ?err, "failed to record contract_verified event");
            }
        });
    }

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
            _ => db_internal_error("get publisher by id", err),
        })?;

    Ok(Json(publisher))
}

/// Get all contracts by a publisher
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

/// Get analytics for a specific contract
pub async fn get_contract_analytics(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ContractAnalyticsResponse>> {
    // Verify the contract exists
    let _contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(id)
pub async fn deploy_green(
    State(state): State<AppState>,
    payload: Result<Json<DeployGreenRequest>, JsonRejection>,
) -> ApiResult<Json<ContractDeployment>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE contract_id = $1")
        .bind(&req.contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("get contract for analytics", err),
        })?;

    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);

    // ── Deployment stats ────────────────────────────────────────────────
    let deploy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM analytics_events \
         WHERE contract_id = $1 AND event_type = 'contract_deployed'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("deployment count", e))?;

    let unique_deployers: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_address) FROM analytics_events \
         WHERE contract_id = $1 AND event_type = 'contract_deployed' AND user_address IS NOT NULL",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("unique deployers", e))?;

    let by_network: serde_json::Value = sqlx::query_scalar(
        r#"
        SELECT COALESCE(
            jsonb_object_agg(COALESCE(network::text, 'unknown'), cnt),
            '{}'::jsonb
        )
        FROM (
            SELECT network, COUNT(*) AS cnt
            FROM analytics_events
            WHERE contract_id = $1 AND event_type = 'contract_deployed'
            GROUP BY network
        ) sub
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("network breakdown", e))?;

    // ── Interactor stats ────────────────────────────────────────────────
    let unique_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_address) FROM analytics_events \
         WHERE contract_id = $1 AND user_address IS NOT NULL",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("unique interactors", e))?;

    let top_user_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT user_address, COUNT(*) AS cnt FROM analytics_events \
         WHERE contract_id = $1 AND user_address IS NOT NULL \
         GROUP BY user_address ORDER BY cnt DESC LIMIT 10",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("top users", e))?;

    let top_users: Vec<TopUser> = top_user_rows
        .into_iter()
        .map(|(address, count)| TopUser { address, count })
        .collect();

    // ── Timeline (last 30 days) ─────────────────────────────────────────
    let timeline_rows: Vec<(chrono::NaiveDate, i64)> = sqlx::query_as(
        r#"
        SELECT d::date AS date, COALESCE(e.cnt, 0) AS count
        FROM generate_series(
            ($1::timestamptz)::date,
            CURRENT_DATE,
            '1 day'::interval
        ) d
        LEFT JOIN (
            SELECT DATE(created_at) AS event_date, COUNT(*) AS cnt
            FROM analytics_events
            WHERE contract_id = $2
              AND created_at >= $1
            GROUP BY DATE(created_at)
        ) e ON d::date = e.event_date
        ORDER BY d::date
        "#,
    )
    .bind(thirty_days_ago)
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("timeline", e))?;

    let timeline: Vec<TimelineEntry> = timeline_rows
        .into_iter()
        .map(|(date, count)| TimelineEntry { date, count })
        .collect();

    // ── Build response ──────────────────────────────────────────────────
    Ok(Json(ContractAnalyticsResponse {
        contract_id: id,
        deployments: DeploymentStats {
            count: deploy_count,
            unique_users: unique_deployers,
            by_network,
        },
        interactors: InteractorStats {
            unique_count,
            top_users,
        },
        timeline,
    }))
}

/// Fallback endpoint for unknown routes
                format!("Contract not found: {}", req.contract_id),
            ),
            _ => db_internal_error("get contract for deployment", err),
        })?;

    let deployment: ContractDeployment = sqlx::query_as(
        "INSERT INTO contract_deployments (contract_id, environment, status, wasm_hash)
         VALUES ($1, 'green', 'testing', $2)
         ON CONFLICT (contract_id, environment) 
         DO UPDATE SET wasm_hash = EXCLUDED.wasm_hash, status = 'testing', 
                       deployed_at = NOW(), error_message = NULL
         RETURNING *",
    )
    .bind(contract.id)
    .bind(&req.wasm_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("deploy green", err))?;

    Ok(Json(deployment))
}

pub async fn switch_deployment(
    State(state): State<AppState>,
    payload: Result<Json<SwitchDeploymentRequest>, JsonRejection>,
) -> ApiResult<Json<serde_json::Value>> {
    let Json(req) = payload.map_err(map_json_rejection)?;
    let force = req.force.unwrap_or(false);

    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE contract_id = $1")
        .bind(&req.contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("Contract not found: {}", req.contract_id),
            ),
            _ => db_internal_error("get contract for switch", err),
        })?;

    let mut tx = state.db.begin().await.map_err(|err| {
        db_internal_error("begin transaction for switch", err)
    })?;

    let active_deployment: Option<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments 
         WHERE contract_id = $1 AND status = 'active'",
    )
    .bind(contract.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| db_internal_error("get active deployment", err))?;

    let from_env = active_deployment
        .as_ref()
        .map(|d| d.environment.clone())
        .unwrap_or(DeploymentEnvironment::Blue);

    let to_env = match from_env {
        DeploymentEnvironment::Blue => DeploymentEnvironment::Green,
        DeploymentEnvironment::Green => DeploymentEnvironment::Blue,
    };

    let green_deployment: Option<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments 
         WHERE contract_id = $1 AND environment = 'green'",
    )
    .bind(contract.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| db_internal_error("get green deployment", err))?;

    if let Some(ref green) = green_deployment {
        if !force && green.status != DeploymentStatus::Testing {
            return Err(ApiError::bad_request(
                "InvalidDeploymentStatus",
                "Green deployment must be in testing status before switch",
            ));
        }
        if !force && green.health_checks_passed < 3 {
            return Err(ApiError::bad_request(
                "InsufficientHealthChecks",
                "Green deployment must pass at least 3 health checks before switch",
            ));
        }
    } else {
        return Err(ApiError::bad_request(
            "NoGreenDeployment",
            "No green deployment found",
        ));
    }

    if let Some(ref active) = active_deployment {
        sqlx::query("UPDATE contract_deployments SET status = 'inactive' WHERE id = $1")
            .bind(active.id)
            .execute(&mut *tx)
            .await
            .map_err(|err| db_internal_error("deactivate current deployment", err))?;
    }

    sqlx::query(
        "UPDATE contract_deployments 
         SET status = 'active', activated_at = NOW() 
         WHERE contract_id = $1 AND environment = $2",
    )
    .bind(contract.id)
    .bind(&to_env)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("activate new deployment", err))?;

    sqlx::query_as::<_, DeploymentSwitch>(
        "INSERT INTO deployment_switches (contract_id, from_environment, to_environment)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(contract.id)
    .bind(&from_env)
    .bind(&to_env)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("record deployment switch", err))?;

    tx.commit().await.map_err(|err| {
        db_internal_error("commit deployment switch", err)
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "switched_from": from_env,
        "switched_to": to_env,
        "contract_id": req.contract_id
    })))
}

pub async fn rollback_deployment(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE contract_id = $1")
        .bind(&contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("Contract not found: {}", contract_id),
            ),
            _ => db_internal_error("get contract for rollback", err),
        })?;

    let mut tx = state.db.begin().await.map_err(|err| {
        db_internal_error("begin transaction for rollback", err)
    })?;

    let active_deployment: Option<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments 
         WHERE contract_id = $1 AND status = 'active'",
    )
    .bind(contract.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| db_internal_error("get active deployment", err))?;

    let from_env = active_deployment
        .as_ref()
        .map(|d| d.environment.clone())
        .unwrap_or(DeploymentEnvironment::Green);

    let to_env = match from_env {
        DeploymentEnvironment::Blue => DeploymentEnvironment::Green,
        DeploymentEnvironment::Green => DeploymentEnvironment::Blue,
    };

    let target_deployment: Option<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments 
         WHERE contract_id = $1 AND environment = $2",
    )
    .bind(contract.id)
    .bind(&to_env)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| db_internal_error("get target deployment", err))?;

    if target_deployment.is_none() {
        return Err(ApiError::bad_request(
            "NoDeploymentToRollback",
            format!("No {} deployment found to rollback to", to_env),
        ));
    }

    if let Some(ref active) = active_deployment {
        sqlx::query("UPDATE contract_deployments SET status = 'inactive' WHERE id = $1")
            .bind(active.id)
            .execute(&mut *tx)
            .await
            .map_err(|err| db_internal_error("deactivate current deployment", err))?;
    }

    sqlx::query(
        "UPDATE contract_deployments 
         SET status = 'active', activated_at = NOW() 
         WHERE contract_id = $1 AND environment = $2",
    )
    .bind(contract.id)
    .bind(&to_env)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("activate rollback deployment", err))?;

    sqlx::query_as::<_, DeploymentSwitch>(
        "INSERT INTO deployment_switches (contract_id, from_environment, to_environment, rollback)
         VALUES ($1, $2, $3, true)
         RETURNING *",
    )
    .bind(contract.id)
    .bind(&from_env)
    .bind(&to_env)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("record rollback switch", err))?;

    tx.commit().await.map_err(|err| {
        db_internal_error("commit rollback", err)
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "rolled_back_from": from_env,
        "rolled_back_to": to_env,
        "contract_id": contract_id
    })))
}

pub async fn report_health_check(
    State(state): State<AppState>,
    payload: Result<Json<HealthCheckRequest>, JsonRejection>,
) -> ApiResult<Json<serde_json::Value>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE contract_id = $1")
        .bind(&req.contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("Contract not found: {}", req.contract_id),
            ),
            _ => db_internal_error("get contract for health check", err),
        })?;

    let env_str = match req.environment {
        DeploymentEnvironment::Blue => "blue",
        DeploymentEnvironment::Green => "green",
    };

    if req.passed {
        sqlx::query(
            "UPDATE contract_deployments 
             SET health_checks_passed = health_checks_passed + 1, 
                 last_health_check_at = NOW()
             WHERE contract_id = $1 AND environment = $2",
        )
        .bind(contract.id)
        .bind(&req.environment)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("update health check passed", err))?;
    } else {
        sqlx::query(
            "UPDATE contract_deployments 
             SET health_checks_failed = health_checks_failed + 1, 
                 status = CASE WHEN health_checks_failed + 1 >= 3 THEN 'failed' ELSE status END,
                 last_health_check_at = NOW()
             WHERE contract_id = $1 AND environment = $2",
        )
        .bind(contract.id)
        .bind(&req.environment)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("update health check failed", err))?;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "environment": env_str,
        "passed": req.passed
    })))
}

pub async fn get_deployment_status(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE contract_id = $1")
        .bind(&contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("Contract not found: {}", contract_id),
            ),
            _ => db_internal_error("get contract", err),
        })?;

    let deployments: Vec<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments 
         WHERE contract_id = $1 
         ORDER BY deployed_at DESC",
    )
    .bind(contract.id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get deployments", err))?;

    let active = deployments.iter().find(|d| matches!(d.status, DeploymentStatus::Active));
    let blue = deployments.iter().find(|d| matches!(d.environment, DeploymentEnvironment::Blue));
    let green = deployments.iter().find(|d| matches!(d.environment, DeploymentEnvironment::Green));

    Ok(Json(serde_json::json!({
        "contract_id": contract_id,
        "active": active,
        "blue": blue,
        "green": green
    })))
}

pub async fn route_not_found() -> ApiError {
    ApiError::not_found("RouteNotFound", "The requested endpoint does not exist")
}
