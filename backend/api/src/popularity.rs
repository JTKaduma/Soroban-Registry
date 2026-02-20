// api/src/popularity.rs
// Popularity scoring engine with hourly batch recalculation

use sqlx::PgPool;
use std::time::Duration;

/// Spawn a background task that recalculates popularity scores every hour.
pub fn spawn_popularity_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));

        loop {
            interval.tick().await;
            tracing::info!("popularity: starting hourly score recalculation");

            if let Err(err) = recalculate_scores(&pool, "7d").await {
                tracing::error!(error = ?err, "popularity: recalculation failed");
            }
        }
    });
}

/// Parse a timeframe string ("7d", "30d", "90d") into a PostgreSQL interval expression.
fn timeframe_to_interval(timeframe: &str) -> &'static str {
    match timeframe {
        "30d" => "30 days",
        "90d" => "90 days",
        _ => "7 days", // default
    }
}

/// Parse timeframe into a decay period in days (used for exponential decay).
fn timeframe_to_decay_days(timeframe: &str) -> f64 {
    match timeframe {
        "30d" => 30.0,
        "90d" => 90.0,
        _ => 7.0,
    }
}

/// Recalculate popularity scores for all contracts.
///
/// Formula:
///   score = (deployments * 0.4) + (interactions * 0.3) + (verification * 0.2) + (age_score * 0.1)
///
/// Where:
///   - deployments: time-decayed count of deployments within the timeframe
///   - interactions: time-decayed count of interactions within the timeframe
///   - verification: 100 if verified, 0 otherwise
///   - age_score: 100 * exp(-days_since_created / 365) — newer = higher
///
/// Time decay: each event is weighted by exp(-days_since_event / decay_period)
pub async fn recalculate_scores(pool: &PgPool, timeframe: &str) -> Result<(), sqlx::Error> {
    let interval = timeframe_to_interval(timeframe);
    let decay_days = timeframe_to_decay_days(timeframe);

    let query = format!(
        r#"
        UPDATE contracts c SET
            popularity_score = COALESCE(scores.score, 0.0),
            score_updated_at = NOW()
        FROM (
            SELECT
                c2.id,
                -- Weighted deployments (0.4)
                COALESCE(dep.decayed_count, 0) * 0.4
                -- Weighted interactions (0.3)
                + COALESCE(inter.decayed_count, 0) * 0.3
                -- Verification bonus (0.2)
                + CASE WHEN c2.is_verified THEN 100.0 ELSE 0.0 END * 0.2
                -- Age score (0.1): newer contracts score higher
                + 100.0 * EXP(-EXTRACT(EPOCH FROM (NOW() - c2.created_at)) / 86400.0 / 365.0) * 0.1
                AS score
            FROM contracts c2
            LEFT JOIN LATERAL (
                SELECT SUM(
                    EXP(-EXTRACT(EPOCH FROM (NOW() - cd.deployed_at)) / 86400.0 / {decay_days})
                ) AS decayed_count
                FROM contract_deployments cd
                WHERE cd.contract_id = c2.id
                  AND cd.deployed_at >= NOW() - INTERVAL '{interval}'
            ) dep ON true
            LEFT JOIN LATERAL (
                SELECT SUM(
                    EXP(-EXTRACT(EPOCH FROM (NOW() - ci.created_at)) / 86400.0 / {decay_days})
                ) AS decayed_count
                FROM contract_interactions ci
                WHERE ci.contract_id = c2.id
                  AND ci.created_at >= NOW() - INTERVAL '{interval}'
            ) inter ON true
        ) scores
        WHERE c.id = scores.id
        "#,
        decay_days = decay_days,
        interval = interval,
    );

    let result = sqlx::query(&query).execute(pool).await?;
    tracing::info!(
        rows_updated = result.rows_affected(),
        timeframe = timeframe,
        "popularity: scores recalculated"
    );

    Ok(())
}
