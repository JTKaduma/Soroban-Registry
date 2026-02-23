use chrono::Utc;
use sqlx::PgPool;
use std::time::Duration;

pub fn spawn_maintenance_scheduler(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = check_scheduled_maintenance(&pool).await {
                tracing::error!("Maintenance scheduler error: {}", e);
            }
        }
    });
}

async fn check_scheduled_maintenance(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    
    let result = sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"
        WITH expired AS (
            SELECT contract_id FROM maintenance_windows 
            WHERE ended_at IS NULL 
            AND scheduled_end_at IS NOT NULL 
            AND scheduled_end_at <= $1
        )
        UPDATE contracts SET is_maintenance = false 
        WHERE id IN (SELECT contract_id FROM expired)
        RETURNING id
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    if !result.is_empty() {
        sqlx::query(
            "UPDATE maintenance_windows SET ended_at = $1 WHERE ended_at IS NULL AND scheduled_end_at <= $1"
        )
        .bind(now)
        .execute(pool)
        .await?;
        
        tracing::info!("Ended {} scheduled maintenance windows", result.len());
    }

    Ok(())
}
