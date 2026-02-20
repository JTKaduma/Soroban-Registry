use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// EXISTING REGISTRY TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Represents a smart contract in the registry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Contract {
    pub id: Uuid,
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: Uuid,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Network where the contract is deployed
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "network_type", rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
}

/// Contract version information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractVersion {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Verification status and details
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Verification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub status: VerificationStatus,
    pub source_code: Option<String>,
    pub build_params: Option<serde_json::Value>,
    pub compiler_version: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Verification status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "verification_status", rename_all = "lowercase")]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed,
}

/// Publisher/developer information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Publisher {
    pub id: Uuid,
    pub stellar_address: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub github_url: Option<String>,
    pub website: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Contract interaction statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractStats {
    pub contract_id: Uuid,
    pub total_deployments: i64,
    pub total_interactions: i64,
    pub unique_users: i64,
    pub last_interaction: Option<DateTime<Utc>>,
}

/// Request to publish a new contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub network: Network,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub publisher_address: String,
}

/// Request to verify a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub contract_id: String,
    pub source_code: String,
    pub build_params: serde_json::Value,
    pub compiler_version: String,
}

/// Search/filter parameters for contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSearchParams {
    pub query: Option<String>,
    pub network: Option<Network>,
    pub verified_only: Option<bool>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub page: Option<i64>,
    #[serde(alias = "page_size")]
    pub limit: Option<i64>,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    #[serde(rename = "contracts")]
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    #[serde(rename = "pages")]
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, page: i64, limit: i64) -> Self {
        let total_pages = if limit > 0 {
            (total as f64 / limit as f64).ceil() as i64
        } else {
            0
        };
        Self {
            items,
            total,
            page,
            total_pages,
    }
}
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "migration_status", rename_all = "snake_case")]
pub enum MigrationStatus {
    Pending,
    Success,
    Failed,
    RolledBack,
}

/// Represents a contract state migration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Migration {
    pub id: Uuid,
    pub contract_id: String,
    pub status: MigrationStatus,
    pub wasm_hash: String,
    pub log_output: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationRequest {
    pub contract_id: String,
    pub wasm_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "deployment_environment", rename_all = "lowercase")]
pub enum DeploymentEnvironment {
    Blue,
    Green,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "deployment_status", rename_all = "lowercase")]
pub enum DeploymentStatus {
    Active,
    Inactive,
    Testing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractDeployment {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: DeploymentEnvironment,
    pub status: DeploymentStatus,
    pub wasm_hash: String,
    pub deployed_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub health_checks_passed: i32,
    pub health_checks_failed: i32,
    pub last_health_check_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentSwitch {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub from_environment: DeploymentEnvironment,
    pub to_environment: DeploymentEnvironment,
    pub switched_at: DateTime<Utc>,
    pub switched_by: Option<String>,
    pub rollback: bool,
}

// ────────────────────────────────────────────────────────────────────────────
// Analytics models
// ────────────────────────────────────────────────────────────────────────────

/// Types of analytics events tracked by the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "analytics_event_type", rename_all = "snake_case")]
pub enum AnalyticsEventType {
    ContractPublished,
    ContractVerified,
    ContractDeployed,
    VersionCreated,
}

impl std::fmt::Display for AnalyticsEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractPublished => write!(f, "contract_published"),
            Self::ContractVerified => write!(f, "contract_verified"),
            Self::ContractDeployed => write!(f, "contract_deployed"),
            Self::VersionCreated => write!(f, "version_created"),
        }
    }
}

/// A raw analytics event recorded when a contract lifecycle action occurs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub event_type: AnalyticsEventType,
    pub contract_id: Uuid,
    pub user_address: Option<String>,
    pub network: Option<Network>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Pre-computed daily aggregate for a single contract
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DailyAggregate {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub date: chrono::NaiveDate,
    pub deployment_count: i32,
    pub unique_deployers: i32,
    pub verification_count: i32,
    pub publish_count: i32,
    pub version_count: i32,
    pub total_events: i32,
    pub unique_users: i32,
    pub network_breakdown: serde_json::Value,
    pub top_users: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ────────────────────────────────────────────────────────────────────────────
// Analytics API response DTOs
// ────────────────────────────────────────────────────────────────────────────

/// Top-level response for GET /api/contracts/:id/analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAnalyticsResponse {
    pub contract_id: Uuid,
    pub deployments: DeploymentStats,
    pub interactors: InteractorStats,
    pub timeline: Vec<TimelineEntry>,
}

/// Deployment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStats {
    pub count: i64,
    pub unique_users: i64,
    pub by_network: serde_json::Value,
}

/// Interactor / unique-user statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractorStats {
    pub unique_count: i64,
    pub top_users: Vec<TopUser>,
}

/// A user ranked by interaction count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopUser {
    pub address: String,
    pub count: i64,
}

/// One data-point in the 30-day timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployGreenRequest {
    pub contract_id: String,
    pub wasm_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchDeploymentRequest {
    pub contract_id: String,
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRequest {
    pub contract_id: String,
    pub environment: DeploymentEnvironment,
    pub passed: bool,
}

// ════════════════════════════════════════════════════════════════════════════
// Audit Log & Version History types
// ════════════════════════════════════════════════════════════════════════════

/// The type of mutation that triggered an audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "audit_action_type", rename_all = "snake_case")]
pub enum AuditActionType {
    ContractPublished,
    MetadataUpdated,
    VerificationChanged,
    PublisherChanged,
    VersionCreated,
    Rollback,
}

impl std::fmt::Display for AuditActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ContractPublished  => "contract_published",
            Self::MetadataUpdated    => "metadata_updated",
            Self::VerificationChanged => "verification_changed",
            Self::PublisherChanged   => "publisher_changed",
            Self::VersionCreated     => "version_created",
            Self::Rollback           => "rollback",
        };
        write!(f, "{}", s)
    }
}

/// One immutable row in `contract_audit_log`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractAuditLog {
    pub id:          Uuid,
    pub contract_id: Uuid,
    pub action_type: AuditActionType,
    pub old_value:   Option<serde_json::Value>,
    pub new_value:   Option<serde_json::Value>,
    pub changed_by:  String,
    pub timestamp:   DateTime<Utc>,
}

/// Full contract state captured at each audited change in `contract_snapshots`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractSnapshot {
    pub id:             Uuid,
    pub contract_id:    Uuid,
    pub version_number: i32,
    pub snapshot_data:  serde_json::Value,
    pub audit_log_id:   Uuid,
    pub created_at:     DateTime<Utc>,
}

/// A single field-level change between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub from:  serde_json::Value,
    pub to:    serde_json::Value,
}

/// Response for GET /api/contracts/:id/versions/:v1/diff/:v2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub contract_id:  Uuid,
    pub from_version: i32,
    pub to_version:   i32,
    /// Fields present in v2 but not v1
    pub added:        Vec<FieldChange>,
    /// Fields present in v1 but not v2
    pub removed:      Vec<FieldChange>,
    /// Fields present in both but with different values
    pub modified:     Vec<FieldChange>,
}

/// Request body for POST /api/contracts/:id/rollback/:snapshot_id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRequest {
    /// Stellar address (or admin service ID) authorising the rollback
    pub changed_by: String,
}

/// Paginated response for audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogPage {
    pub items:       Vec<ContractAuditLog>,
    pub total:       i64,
    pub page:        i64,
    pub total_pages: i64,
}
