use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ChallengeQuery {
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub address: String,
    pub nonce: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub address: String,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in_seconds: u64,
}

pub async fn get_challenge(
    State(state): State<AppState>,
    Query(query): Query<ChallengeQuery>,
) -> ApiResult<Json<ChallengeResponse>> {
    if query.address.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidAddress",
            "address is required",
        ));
    }
    let mut mgr = state.auth_mgr.write().unwrap();
    let nonce = mgr.create_challenge(&query.address);
    Ok(Json(ChallengeResponse {
        address: query.address,
        nonce,
        expires_in_seconds: 300,
    }))
}

pub async fn verify_challenge(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<(StatusCode, Json<VerifyResponse>), ApiError> {
    if payload.address.trim().is_empty()
        || payload.public_key.trim().is_empty()
        || payload.signature.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "InvalidPayload",
            "address, public_key and signature are required",
        ));
    }
    let mut mgr = state.auth_mgr.write().unwrap();
    let token = mgr
        .verify_and_issue_jwt(&payload.address, &payload.public_key, &payload.signature)
        .map_err(|_| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "AuthFailed",
                "invalid challenge response",
            )
        })?;
    Ok((
        StatusCode::OK,
        Json(VerifyResponse {
            token,
            token_type: "Bearer",
            expires_in_seconds: 86_400,
        }),
    ))
}
