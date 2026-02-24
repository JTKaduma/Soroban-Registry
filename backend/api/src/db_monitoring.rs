use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use crate::cache::CacheLayer;
use crate::metrics;

pub fn spawn_db_monitoring_task(pool: PgPool, cache: Arc<CacheLayer>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let max_connections = pool.options().get_max_connections();

        loop {
            interval.tick().await;

            // Database Pool Metrics
            let total_connections = pool.size() as u32;
            let idle_connections = pool.num_idle() as u32;
            let active_connections = total_connections.saturating_sub(idle_connections);

            metrics::DB_CONNECTIONS_ACTIVE.set(active_connections as i64);
            metrics::DB_CONNECTIONS_IDLE.set(idle_connections as i64);
            metrics::DB_POOL_SIZE.set(total_connections as i64);

            let utilization = if max_connections > 0 {
                active_connections as f64 / max_connections as f64
            } else {
                0.0
            };

            metrics::DB_POOL_UTILIZATION.with_label_values(&["default"]).set(utilization);


            if utilization >= 0.8 {
                tracing::warn!(
                    utilization = %format!("{:.1}%", utilization * 100.0),
                    active = active_connections,
                    idle = idle_connections,
                    max = max_connections,
                    "High database pool utilization detected"
                );
            }

            // Moka Cache Metrics
            let abi_entries = cache.abi_cache.entry_count();
            let abi_size = cache.abi_cache.weighted_size();
            let ver_entries = cache.verification_cache.entry_count();
            let ver_size = cache.verification_cache.weighted_size();

            metrics::CACHE_ENTRIES.set(abi_entries.saturating_add(ver_entries) as i64);
            metrics::CACHE_SIZE_BYTES.set(abi_size.saturating_add(ver_size) as i64);

            tracing::debug!(
                db_active = active_connections,
                db_idle = idle_connections,
                cache_entries = abi_entries + ver_entries,
                "Resource monitoring update"
            );
        }
    });
}

/// Helper to acquire a connection with latency tracking and slow acquisition logging
pub async fn acquire_with_metrics(pool: &PgPool) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, sqlx::Error> {
    let start = std::time::Instant::now();
    let res = pool.acquire().await;
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;

    metrics::DB_CONNECTION_WAIT_MS.with_label_values(&["default"]).observe(duration_ms);

    if res.is_err() {
        metrics::DB_POOL_TIMEOUTS.inc();
    }

    if duration_ms > 100.0 {
        tracing::warn!(
            duration_ms = duration_ms,
            "Slow database connection acquisition"
        );
    }

    res
}
