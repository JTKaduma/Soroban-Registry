use sqlx::PgPool;
use std::time::Duration;

/// Spawn the background aggregation task.
///
/// Runs every hour:
///   1. Aggregate raw events into daily summaries (yesterday + today).
///   2. Delete raw events older than 90 days.
pub fn spawn_aggregation_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));

        loop {
            interval.tick().await;
            tracing::info!("aggregation: starting hourly run");

            if let Err(err) = run_aggregation(&pool).await {
                tracing::error!(error = ?err, "aggregation: run failed");
            }

            if let Err(err) = cleanup_old_events(&pool).await {
                tracing::error!(error = ?err, "aggregation: retention cleanup failed");
            }
        }
    });
}

/// Build daily aggregates from raw `analytics_events`.
///
/// Uses `ON CONFLICT … DO UPDATE` so re-running is idempotent.
async fn run_aggregation(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Aggregate events from the last 2 days (yesterday + partial today)
    // to ensure we always capture the freshest data.
    let rows_affected = sqlx::query(
        r#"
        INSERT INTO analytics_daily_aggregates (
            contract_id, date,
            deployment_count, unique_deployers,
            verification_count, publish_count, version_count,
            total_events, unique_users,
            network_breakdown, top_users
        )
        SELECT
            e.contract_id,
            DATE(e.created_at) AS agg_date,

            -- deployment counts
            COUNT(*) FILTER (WHERE e.event_type = 'contract_deployed') AS deployment_count,
            COUNT(DISTINCT e.user_address) FILTER (WHERE e.event_type = 'contract_deployed') AS unique_deployers,

            -- other event counts
            COUNT(*) FILTER (WHERE e.event_type = 'contract_verified') AS verification_count,
            COUNT(*) FILTER (WHERE e.event_type = 'contract_published') AS publish_count,
            COUNT(*) FILTER (WHERE e.event_type = 'version_created') AS version_count,

            -- totals
            COUNT(*) AS total_events,
            COUNT(DISTINCT e.user_address) AS unique_users,

            -- network breakdown as JSON object
            COALESCE(
                jsonb_object_agg(
                    COALESCE(e.network::text, 'unknown'),
                    sub.net_count
                ) FILTER (WHERE sub.net_count IS NOT NULL),
                '{}'::jsonb
            ) AS network_breakdown,

            -- top users as JSON array (top 10)
            COALESCE(
                (
                    SELECT jsonb_agg(
                        jsonb_build_object('address', tu.user_address, 'count', tu.cnt)
                        ORDER BY tu.cnt DESC
                    )
                    FROM (
                        SELECT e2.user_address, COUNT(*) AS cnt
                        FROM analytics_events e2
                        WHERE e2.contract_id = e.contract_id
                          AND DATE(e2.created_at) = DATE(e.created_at)
                          AND e2.user_address IS NOT NULL
                        GROUP BY e2.user_address
                        ORDER BY cnt DESC
                        LIMIT 10
                    ) tu
                ),
                '[]'::jsonb
            ) AS top_users

        FROM analytics_events e
        LEFT JOIN LATERAL (
            SELECT e.network, COUNT(*) AS net_count
            FROM analytics_events e3
            WHERE e3.contract_id = e.contract_id
              AND DATE(e3.created_at) = DATE(e.created_at)
              AND e3.network IS NOT NULL
            GROUP BY e3.network
        ) sub ON true
        WHERE e.created_at >= CURRENT_DATE - INTERVAL '1 day'
        GROUP BY e.contract_id, DATE(e.created_at)

        ON CONFLICT (contract_id, date) DO UPDATE SET
            deployment_count    = EXCLUDED.deployment_count,
            unique_deployers    = EXCLUDED.unique_deployers,
            verification_count  = EXCLUDED.verification_count,
            publish_count       = EXCLUDED.publish_count,
            version_count       = EXCLUDED.version_count,
            total_events        = EXCLUDED.total_events,
            unique_users        = EXCLUDED.unique_users,
            network_breakdown   = EXCLUDED.network_breakdown,
            top_users           = EXCLUDED.top_users
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    tracing::info!(
        rows = rows_affected,
        "aggregation: daily summaries upserted"
    );
    Ok(())
}

/// Delete raw analytics events older than 90 days.
async fn cleanup_old_events(pool: &PgPool) -> Result<(), sqlx::Error> {
    let deleted =
        sqlx::query("DELETE FROM analytics_events WHERE created_at < NOW() - INTERVAL '90 days'")
            .execute(pool)
            .await?
            .rows_affected();

    if deleted > 0 {
        tracing::info!(deleted, "aggregation: cleaned up old raw events");
    }

    Ok(())
}
