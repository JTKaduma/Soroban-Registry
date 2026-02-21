use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn maintenance_check(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method();
    let path = request.uri().path();

    // Only check write operations
    if !matches!(method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE") {
        return next.run(request).await;
    }

    // Extract contract_id from path if present
    if let Some(contract_id) = extract_contract_id(path) {
        let is_maintenance = sqlx::query_scalar::<_, bool>(
            "SELECT is_maintenance FROM contracts WHERE id = $1::uuid",
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(Some(false))
        .unwrap_or(false);

        if is_maintenance {
            let message = sqlx::query_scalar::<_, String>(
                "SELECT message FROM maintenance_windows WHERE contract_id = $1::uuid AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
            )
            .bind(contract_id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "Contract is in maintenance mode".to_string());

            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "maintenance_mode",
                    "message": message
                })),
            )
                .into_response();
        }
    }

    next.run(request).await
}

fn extract_contract_id(path: &str) -> Option<&str> {
    // Match patterns like /api/contracts/{id}/...
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 && parts[2] == "contracts" {
        Some(parts[3])
    } else {
        None
    }
}
