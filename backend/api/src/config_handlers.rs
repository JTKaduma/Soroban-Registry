use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::Deserialize;
use serde_json::Value;
use shared::models::{
    ConfigCreateRequest, ConfigRollbackRequest, ContractConfig, ContractConfigResponse,
};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

fn get_encryption_key() -> [u8; 32] {
    let key = std::env::var("CONFIG_SECRET_KEY")
        .unwrap_or_else(|_| "0123456789abcdef0123456789abcdef".to_string());
    let mut bytes = [0u8; 32];
    let key_bytes = key.as_bytes();
    let len = key_bytes.len().min(32);
    bytes[..len].copy_from_slice(&key_bytes[..len]);
    bytes
}

fn encrypt_secrets(secrets: &Value) -> Result<Value, ApiError> {
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new(&key.into());

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = serde_json::to_vec(secrets)
        .map_err(|e| ApiError::internal(format!("Failed to serialize secrets: {}", e)))?;

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| ApiError::internal(format!("Encryption error: {}", e)))?;

    Ok(serde_json::json!({
        "ciphertext": BASE64.encode(ciphertext),
        "nonce": BASE64.encode(nonce_bytes)
    }))
}

#[derive(Deserialize)]
pub struct ConfigQuery {
    pub environment: String,
}

pub async fn get_contract_config(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<ConfigQuery>,
) -> Result<Json<ContractConfigResponse>, ApiError> {
    let config = sqlx::query_as::<_, ContractConfig>(
        r#"
        SELECT id, contract_id, environment, version, config_data, secrets_data, created_at, created_by
        FROM contract_configs
        WHERE contract_id = $1 AND environment = $2
        ORDER BY version DESC
        LIMIT 1
        "#,
    )
    .bind(contract_id)
    .bind(&query.environment)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .ok_or_else(|| ApiError::not_found("ConfigNotFound", "Configuration not found"))?;

    Ok(Json(config.into()))
}

pub async fn create_contract_config(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(payload): Json<ConfigCreateRequest>,
) -> Result<(StatusCode, Json<ContractConfigResponse>), ApiError> {
    let current_version: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(version), 0) FROM contract_configs
        WHERE contract_id = $1 AND environment = $2
        "#,
    )
    .bind(contract_id)
    .bind(&payload.environment)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let next_version = current_version + 1;

    let encrypted_secrets = match &payload.secrets_data {
        Some(s) => Some(encrypt_secrets(s)?),
        None => None,
    };

    let new_config = sqlx::query_as::<_, ContractConfig>(
        r#"
        INSERT INTO contract_configs (contract_id, environment, version, config_data, secrets_data, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, contract_id, environment, version, config_data, secrets_data, created_at, created_by
        "#,
    )
    .bind(contract_id)
    .bind(&payload.environment)
    .bind(next_version)
    .bind(&payload.config_data)
    .bind(encrypted_secrets)
    .bind(&payload.created_by)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(new_config.into())))
}

pub async fn get_config_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<ConfigQuery>,
) -> Result<Json<Vec<ContractConfigResponse>>, ApiError> {
    let configs = sqlx::query_as::<_, ContractConfig>(
        r#"
        SELECT id, contract_id, environment, version, config_data, secrets_data, created_at, created_by
        FROM contract_configs
        WHERE contract_id = $1 AND environment = $2
        ORDER BY version DESC
        "#,
    )
    .bind(contract_id)
    .bind(&query.environment)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let responses: Vec<ContractConfigResponse> = configs.into_iter().map(|c| c.into()).collect();
    Ok(Json(responses))
}

pub async fn rollback_config(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<ConfigQuery>,
    Json(payload): Json<ConfigRollbackRequest>,
) -> Result<(StatusCode, Json<ContractConfigResponse>), ApiError> {
    // Fetch the target version
    let target_config = sqlx::query_as::<_, ContractConfig>(
        r#"
        SELECT id, contract_id, environment, version, config_data, secrets_data, created_at, created_by
        FROM contract_configs
        WHERE contract_id = $1 AND environment = $2 AND version = $3
        "#,
    )
    .bind(contract_id)
    .bind(&query.environment)
    .bind(payload.roll_back_to_version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .ok_or_else(|| ApiError::not_found("ConfigNotFound", "Target version not found for rollback"))?;

    // Create a new version with target_config data
    let current_version: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(version), 0) FROM contract_configs
        WHERE contract_id = $1 AND environment = $2
        "#,
    )
    .bind(contract_id)
    .bind(&query.environment)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let next_version = current_version + 1;

    let new_config = sqlx::query_as::<_, ContractConfig>(
        r#"
        INSERT INTO contract_configs (contract_id, environment, version, config_data, secrets_data, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, contract_id, environment, version, config_data, secrets_data, created_at, created_by
        "#,
    )
    .bind(contract_id)
    .bind(&query.environment)
    .bind(next_version)
    .bind(&target_config.config_data)
    .bind(&target_config.secrets_data) // Keep the already encrypted secrets
    .bind(&payload.created_by)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(new_config.into())))
}
