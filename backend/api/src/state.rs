use crate::cache::{CacheConfig, CacheLayer};
use prometheus::Registry;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub started_at: Instant,
    pub cache: Arc<CacheLayer>,
    pub registry: Registry,
}

impl AppState {
    pub fn new(db: PgPool, registry: Registry) -> Self {
        let config = CacheConfig::from_env();
        Self {
            db,
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(config)),
            registry,
        }
    }
}
