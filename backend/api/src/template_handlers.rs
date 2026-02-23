use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ContractTemplate {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub version: String,
    pub source_code: String,
    pub parameters: serde_json::Value,
    pub install_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TemplateListParams {
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloneRequest {
    pub name: String,
    pub parameters: Option<serde_json::Value>,
    pub user_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CloneResponse {
    pub template_slug: String,
    pub contract_name: String,
    pub source_code: String,
    pub version: String,
}

pub async fn list_templates(
    State(state): State<AppState>,
    Query(params): Query<TemplateListParams>,
) -> ApiResult<Json<Vec<ContractTemplate>>> {
    let templates: Vec<ContractTemplate> = match params.category {
        Some(cat) => sqlx::query_as(
                "SELECT * FROM contract_templates WHERE category = $1 ORDER BY install_count DESC",
            )
            .bind(cat)
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_internal_error("list templates by category", e))?,
        None => sqlx::query_as("SELECT * FROM contract_templates ORDER BY install_count DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_internal_error("list all templates", e))?,
    };

    Ok(Json(templates))
}

pub async fn get_template(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<ContractTemplate>> {
    let template: ContractTemplate =
        sqlx::query_as("SELECT * FROM contract_templates WHERE slug = $1")
            .bind(&slug)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "TemplateNotFound",
                    format!("No template found with slug: {}", slug),
                ),
                _ => db_internal_error("get template by slug", err),
            })?;

    Ok(Json(template))
}

pub async fn clone_template(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<CloneRequest>,
) -> impl IntoResponse {
    let template: ContractTemplate =
        match sqlx::query_as("SELECT * FROM contract_templates WHERE slug = $1")
            .bind(&slug)
            .fetch_one(&state.db)
            .await
        {
            Ok(t) => t,
            Err(sqlx::Error::RowNotFound) => {
                return ApiError::not_found(
                    "TemplateNotFound",
                    format!("No template found with slug: {}", slug),
                )
                .into_response()
            }
            Err(e) => return db_internal_error("get template for clone", e).into_response(),
        };

    let source = apply_parameters(&template.source_code, &req.name, &req.parameters);

    let pool = state.db.clone();
    let template_id = template.id;
    let user_addr = req.user_address.clone();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO template_installs (template_id, user_address) VALUES ($1, $2)",
        )
        .bind(template_id)
        .bind(user_addr)
        .execute(&pool)
        .await;

        let _ = sqlx::query(
            "UPDATE contract_templates SET install_count = install_count + 1 WHERE id = $1",
        )
        .bind(template_id)
        .execute(&pool)
        .await;
    });

    (
        axum::http::StatusCode::OK,
        Json(CloneResponse {
            template_slug: slug,
            contract_name: req.name,
            source_code: source,
            version: template.version,
        }),
    )
        .into_response()
}

fn apply_parameters(
    source: &str,
    contract_name: &str,
    params: &Option<serde_json::Value>,
) -> String {
    let mut out = source.replace("{{CONTRACT_NAME}}", contract_name);

    if let Some(serde_json::Value::Object(map)) = params {
        for (key, val) in map {
            let placeholder = format!("{{{{{}}}}}", key.to_uppercase());
            let replacement = match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            out = out.replace(&placeholder, &replacement);
        }
    }

    out
}
