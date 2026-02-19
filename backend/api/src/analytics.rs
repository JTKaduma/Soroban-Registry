use shared::{AnalyticsEventType, Network};
use sqlx::PgPool;
use uuid::Uuid;

/// Record an analytics event.
///
/// This is intentionally fire-and-forget: callers should log errors but
/// never let a failed analytics insert break the main request flow.
pub async fn record_event(
    pool: &PgPool,
    event_type: AnalyticsEventType,
    contract_id: Uuid,
    user_address: Option<&str>,
    network: Option<&Network>,
    metadata: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO analytics_events (event_type, contract_id, user_address, network, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&event_type)
    .bind(contract_id)
    .bind(user_address)
    .bind(network)
    .bind(metadata.unwrap_or(serde_json::json!({})))
    .execute(pool)
    .await?;

    tracing::debug!(
        event = %event_type,
        contract = %contract_id,
        "analytics event recorded"
    );

    Ok(())
}
