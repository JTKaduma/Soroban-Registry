//! API Handlers for Contract Type Safety Validation
//!
//! Provides endpoints for validating contract function calls
//! and generating type-safe bindings.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;
use crate::type_safety::{
    bindings::{generate_bindings, BindingLanguage},
    parser::parse_json_spec,
    validator::{CallValidator, FunctionInfo, ValidateCallRequest, ValidationResult},
};

/// Request body for validate-call endpoint
#[derive(Debug, Deserialize)]
pub struct ValidateCallBody {
    /// Method name to validate
    pub method_name: String,
    /// Parameters as string values
    pub params: Vec<String>,
    /// Enable strict mode (no implicit conversions)
    #[serde(default)]
    pub strict: bool,
}

/// Response for validate-call endpoint
#[derive(Debug, Serialize)]
pub struct ValidateCallResponse {
    pub valid: bool,
    pub function_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ValidationErrorDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ValidationWarningDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_params: Option<Vec<ParsedParamDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_return: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationErrorDto {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationWarningDto {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParsedParamDto {
    pub name: String,
    pub expected_type: String,
    pub value: serde_json::Value,
}

/// Query params for binding generation
#[derive(Debug, Deserialize)]
pub struct GenerateBindingsQuery {
    /// Language for bindings: "typescript" or "rust"
    pub language: String,
}

/// Response for functions list endpoint
#[derive(Debug, Serialize)]
pub struct ContractFunctionsResponse {
    pub contract_id: String,
    pub contract_name: String,
    pub functions: Vec<FunctionInfoDto>,
}

#[derive(Debug, Serialize)]
pub struct FunctionInfoDto {
    pub name: String,
    pub visibility: String,
    pub params: Vec<ParamInfoDto>,
    pub return_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub is_mutable: bool,
}

#[derive(Debug, Serialize)]
pub struct ParamInfoDto {
    pub name: String,
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// API Error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            StatusCode::NOT_FOUND,
            Json(Self {
                error: "NOT_FOUND".to_string(),
                message: message.into(),
            }),
        )
    }

    pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            StatusCode::BAD_REQUEST,
            Json(Self {
                error: "BAD_REQUEST".to_string(),
                message: message.into(),
            }),
        )
    }

    pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Self {
                error: "INTERNAL_ERROR".to_string(),
                message: message.into(),
            }),
        )
    }
}

/// POST /api/contracts/:id/validate-call
///
/// Validate a contract function call for type safety
pub async fn validate_call(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Json(body): Json<ValidateCallBody>,
) -> Result<Json<ValidateCallResponse>, (StatusCode, Json<ApiError>)> {
    // 1. Fetch contract ABI from database
    let abi_json = fetch_contract_abi(&state, &contract_id)
        .await
        .map_err(|e| ApiError::not_found(e))?;

    // 2. Parse ABI
    let abi = parse_json_spec(&abi_json, &contract_id)
        .map_err(|e| ApiError::bad_request(format!("Failed to parse ABI: {}", e)))?;

    // 3. Create validator
    let validator = if body.strict {
        CallValidator::new(abi).strict()
    } else {
        CallValidator::new(abi)
    };

    // 4. Validate the call
    let result = validator.validate_call(&body.method_name, &body.params);

    // 5. Convert to response
    Ok(Json(validation_result_to_response(result)))
}

/// GET /api/contracts/:id/functions
///
/// List all functions available on a contract
pub async fn list_contract_functions(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> Result<Json<ContractFunctionsResponse>, (StatusCode, Json<ApiError>)> {
    // Fetch and parse ABI
    let abi_json = fetch_contract_abi(&state, &contract_id)
        .await
        .map_err(|e| ApiError::not_found(e))?;

    let abi = parse_json_spec(&abi_json, &contract_id)
        .map_err(|e| ApiError::bad_request(format!("Failed to parse ABI: {}", e)))?;

    let validator = CallValidator::new(abi.clone());
    let functions = validator.list_functions();

    Ok(Json(ContractFunctionsResponse {
        contract_id,
        contract_name: abi.name,
        functions: functions.into_iter().map(function_info_to_dto).collect(),
    }))
}

/// GET /api/contracts/:id/functions/:method
///
/// Get information about a specific function
pub async fn get_function_info(
    State(state): State<AppState>,
    Path((contract_id, method_name)): Path<(String, String)>,
) -> Result<Json<FunctionInfoDto>, (StatusCode, Json<ApiError>)> {
    let abi_json = fetch_contract_abi(&state, &contract_id)
        .await
        .map_err(|e| ApiError::not_found(e))?;

    let abi = parse_json_spec(&abi_json, &contract_id)
        .map_err(|e| ApiError::bad_request(format!("Failed to parse ABI: {}", e)))?;

    let validator = CallValidator::new(abi);
    let info = validator
        .get_function_info(&method_name)
        .ok_or_else(|| ApiError::not_found(format!("Function '{}' not found", method_name)))?;

    Ok(Json(function_info_to_dto(info)))
}

/// GET /api/contracts/:id/bindings?language=typescript
///
/// Generate type-safe bindings for a contract
pub async fn generate_contract_bindings(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(query): Query<GenerateBindingsQuery>,
) -> Result<(StatusCode, String), (StatusCode, Json<ApiError>)> {
    // Parse language
    let language: BindingLanguage = query
        .language
        .parse()
        .map_err(|e: String| ApiError::bad_request(e))?;

    // Fetch and parse ABI
    let abi_json = fetch_contract_abi(&state, &contract_id)
        .await
        .map_err(|e| ApiError::not_found(e))?;

    let abi = parse_json_spec(&abi_json, &contract_id)
        .map_err(|e| ApiError::bad_request(format!("Failed to parse ABI: {}", e)))?;

    // Generate bindings
    let bindings = generate_bindings(&abi, language);

    // Return with appropriate content type
    let content_type = match language {
        BindingLanguage::TypeScript => "application/typescript",
        BindingLanguage::Rust => "text/x-rust",
    };

    Ok((StatusCode::OK, bindings))
}

/// Helper: Fetch contract ABI from database
async fn fetch_contract_abi(state: &AppState, contract_id: &str) -> Result<String, String> {
    // Try to parse as UUID first, then fall back to contract_id string lookup
    let query = if let Ok(uuid) = Uuid::parse_str(contract_id) {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT abi FROM contracts WHERE id = $1"
        )
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
    } else {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT abi FROM contracts WHERE contract_id = $1"
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
    };

    match query {
        Ok(Some(Some(abi))) => Ok(abi),
        Ok(Some(None)) => Err(format!("Contract '{}' has no ABI", contract_id)),
        Ok(None) => Err(format!("Contract '{}' not found", contract_id)),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

/// Convert ValidationResult to API response
fn validation_result_to_response(result: ValidationResult) -> ValidateCallResponse {
    ValidateCallResponse {
        valid: result.valid,
        function_name: result.function_name,
        errors: result
            .errors
            .into_iter()
            .map(|e| ValidationErrorDto {
                code: format!("{:?}", e.code),
                message: e.message,
                field: e.field,
                expected: e.expected,
                actual: e.actual,
            })
            .collect(),
        warnings: result
            .warnings
            .into_iter()
            .map(|w| ValidationWarningDto {
                code: format!("{:?}", w.code),
                message: w.message,
                field: w.field,
            })
            .collect(),
        parsed_params: result.parsed_params.map(|params| {
            params
                .into_iter()
                .map(|p| ParsedParamDto {
                    name: p.name,
                    expected_type: p.expected_type,
                    value: serde_json::to_value(&p.value).unwrap_or(serde_json::Value::Null),
                })
                .collect()
        }),
        expected_return: result.expected_return,
    }
}

/// Convert FunctionInfo to DTO
fn function_info_to_dto(info: FunctionInfo) -> FunctionInfoDto {
    FunctionInfoDto {
        name: info.name,
        visibility: format!("{:?}", info.visibility).to_lowercase(),
        params: info
            .params
            .into_iter()
            .map(|p| ParamInfoDto {
                name: p.name,
                type_name: p.type_name,
                doc: p.doc,
            })
            .collect(),
        return_type: info.return_type,
        doc: info.doc,
        is_mutable: info.is_mutable,
    }
}
