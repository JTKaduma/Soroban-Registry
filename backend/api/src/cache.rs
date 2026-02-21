use async_trait::async_trait;
use moka::future::Cache as MokaCache;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache configuration options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvictionPolicy {
    Lru,
    Lfu, // Implemented via Moka (TinyLFU)
}

impl std::str::FromStr for EvictionPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lru" => Ok(EvictionPolicy::Lru),
            "lfu" => Ok(EvictionPolicy::Lfu),
            _ => Err(format!("Unknown eviction policy: {}", s)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub enabled: bool,
    pub policy: EvictionPolicy,
    pub global_ttl: Duration,
    pub max_capacity: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: EvictionPolicy::Lfu,
            global_ttl: Duration::from_secs(60),
            max_capacity: 10_000,
        }
    }
}

impl CacheConfig {
    /// Load configuration from environment variables with fallback to defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(enabled_str) = std::env::var("CACHE_ENABLED") {
            config.enabled = enabled_str.to_lowercase() == "true";
        }

        if let Ok(ttl_str) = std::env::var("CACHE_TTL_SECONDS") {
            if let Ok(secs) = ttl_str.parse::<u64>() {
                config.global_ttl = Duration::from_secs(secs);
            }
        }

        if let Ok(policy_str) = std::env::var("CACHE_POLICY") {
            if let Ok(policy) = policy_str.parse::<EvictionPolicy>() {
                config.policy = policy;
            }
        }

        if let Ok(capacity_str) = std::env::var("CACHE_MAX_CAPACITY") {
            if let Ok(capacity) = capacity_str.parse::<u64>() {
                config.max_capacity = capacity;
            }
        }

        tracing::info!(
            "Cache config loaded: enabled={}, policy={:?}, ttl={:?}, capacity={}",
            config.enabled,
            config.policy,
            config.global_ttl,
            config.max_capacity
        );

        config
    }
}

/// Metrics for cache performance - with symmetric instrumentation
#[derive(Debug, Default)]
pub struct CacheMetrics {
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,

    // Cached hit latency (µs) - recorded when cache hit occurs
    pub cached_hit_latency_sum_micros: AtomicUsize,
    pub cached_hit_count: AtomicUsize,

    // Cache miss latency (µs) - recorded for miss path only (lookup + fetch)
    pub cache_miss_latency_sum_micros: AtomicUsize,
    pub cache_miss_count: AtomicUsize,

    // Uncached baseline latency (µs) - recorded when cache=off to establish baseline
    pub uncached_latency_sum_micros: AtomicUsize,
    pub uncached_count: AtomicUsize,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64 * 100.0
        }
    }

    pub fn avg_cached_hit_latency(&self) -> f64 {
        let sum = self.cached_hit_latency_sum_micros.load(Ordering::Relaxed);
        let count = self.cached_hit_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        }
    }

    pub fn avg_cache_miss_latency(&self) -> f64 {
        let sum = self.cache_miss_latency_sum_micros.load(Ordering::Relaxed);
        let count = self.cache_miss_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        }
    }

    pub fn avg_uncached_latency(&self) -> f64 {
        let sum = self.uncached_latency_sum_micros.load(Ordering::Relaxed);
        let count = self.uncached_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        }
    }

    /// Improvement factor = uncached latency / cached hit latency
    /// Only valid when both have measurements
    pub fn improvement_factor(&self) -> f64 {
        let cached_hit = self.avg_cached_hit_latency();
        let uncached = self.avg_uncached_latency();

        // Must have both measurements to compute improvement
        if cached_hit == 0.0 || uncached == 0.0 {
            0.0
        } else {
            uncached / cached_hit
        }
    }
}

/// Cache read result with latency information
#[derive(Debug, Clone)]
pub struct CacheReadResult {
    pub value: Option<String>,
    /// Whether this was a cache hit (true) or miss (false)
    pub was_hit: bool,
    /// Latency of the cache lookup operation in microseconds
    pub lookup_latency_micros: usize,
}

/// Cache interface
#[async_trait]
pub trait ContractStateCache: Send + Sync {
    /// Get from cache. Returns (value, was_hit, lookup_latency_micros)
    async fn get(&self, contract_id: &str, key: &str) -> CacheReadResult;

    /// Put into cache with optional per-key TTL override
    async fn put(
        &self,
        contract_id: &str,
        key: &str,
        value: String,
        ttl_override: Option<Duration>,
    );

    /// Invalidate a cache entry
    async fn invalidate(&self, contract_id: &str, key: &str);

    fn metrics(&self) -> &CacheMetrics;
}

/// Moka-based implementation (TinyLFU) with per-key TTL support
pub struct MokaLfuCache {
    cache: MokaCache<String, (String, Option<Instant>)>,
    metrics: CacheMetrics,
    ttl: Duration,
}

impl MokaLfuCache {
    pub fn new(capacity: u64, ttl: Duration) -> Self {
        Self {
            cache: MokaCache::builder()
                .max_capacity(capacity)
                .time_to_live(ttl)
                .build(),
            metrics: CacheMetrics::default(),
            ttl,
        }
    }
}

#[async_trait]
impl ContractStateCache for MokaLfuCache {
    async fn get(&self, contract_id: &str, key: &str) -> CacheReadResult {
        let cache_key = format!("{}:{}", contract_id, key);
        let start = Instant::now();

        let result = self.cache.get(&cache_key).await;
        let lookup_latency = start.elapsed().as_micros() as usize;

        match result {
            Some((value, expiry_opt)) => {
                // Check if per-key TTL has expired
                if let Some(expiry) = expiry_opt {
                    if Instant::now() >= expiry {
                        // Expired entry
                        self.cache.invalidate(&cache_key).await;
                        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
                        return CacheReadResult {
                            value: None,
                            was_hit: false,
                            lookup_latency_micros: lookup_latency,
                        };
                    }
                }

                // Valid cache hit
                self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                self.metrics
                    .cached_hit_latency_sum_micros
                    .fetch_add(lookup_latency, Ordering::Relaxed);
                self.metrics
                    .cached_hit_count
                    .fetch_add(1, Ordering::Relaxed);

                CacheReadResult {
                    value: Some(value),
                    was_hit: true,
                    lookup_latency_micros: lookup_latency,
                }
            }
            None => {
                // Cache miss
                self.metrics.misses.fetch_add(1, Ordering::Relaxed);
                CacheReadResult {
                    value: None,
                    was_hit: false,
                    lookup_latency_micros: lookup_latency,
                }
            }
        }
    }

    async fn put(
        &self,
        contract_id: &str,
        key: &str,
        value: String,
        ttl_override: Option<Duration>,
    ) {
        let cache_key = format!("{}:{}", contract_id, key);

        // Support per-key TTL by storing expiry time with value
        let expiry = ttl_override.map(|ttl| Instant::now() + ttl);
        self.cache.insert(cache_key, (value, expiry)).await;
    }

    async fn invalidate(&self, contract_id: &str, key: &str) {
        let cache_key = format!("{}:{}", contract_id, key);
        self.cache.invalidate(&cache_key).await;
    }

    fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }
}

/// LRU-based implementation using `lru` crate + RwLock
struct LruEntry {
    value: String,
    expiry: Instant,
}

pub struct LruCacheImpl {
    cache: RwLock<lru::LruCache<String, LruEntry>>,
    metrics: CacheMetrics,
    default_ttl: Duration,
}

impl LruCacheImpl {
    pub fn new(capacity: u64, ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity as usize).unwrap(),
            )),
            metrics: CacheMetrics::default(),
            default_ttl: ttl,
        }
    }
}

#[async_trait]
impl ContractStateCache for LruCacheImpl {
    async fn get(&self, contract_id: &str, key: &str) -> CacheReadResult {
        let cache_key = format!("{}:{}", contract_id, key);
        let start = Instant::now();
        let mut cache = self.cache.write().await;

        // Check existence and expiry
        if let Some(entry) = cache.get(&cache_key) {
            if entry.expiry > Instant::now() {
                // Valid hit
                let lookup_latency = start.elapsed().as_micros() as usize;
                self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                self.metrics
                    .cached_hit_latency_sum_micros
                    .fetch_add(lookup_latency, Ordering::Relaxed);
                self.metrics
                    .cached_hit_count
                    .fetch_add(1, Ordering::Relaxed);

                return CacheReadResult {
                    value: Some(entry.value.clone()),
                    was_hit: true,
                    lookup_latency_micros: lookup_latency,
                };
            } else {
                // Expired - remove it
                cache.pop(&cache_key);
            }
        }

        // Miss (not found or expired)
        let lookup_latency = start.elapsed().as_micros() as usize;
        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
        CacheReadResult {
            value: None,
            was_hit: false,
            lookup_latency_micros: lookup_latency,
        }
    }

    async fn put(
        &self,
        contract_id: &str,
        key: &str,
        value: String,
        ttl_override: Option<Duration>,
    ) {
        let cache_key = format!("{}:{}", contract_id, key);
        let ttl = ttl_override.unwrap_or(self.default_ttl);
        let expiry = Instant::now() + ttl;
        let mut cache = self.cache.write().await;
        cache.put(cache_key, LruEntry { value, expiry });
    }

    async fn invalidate(&self, contract_id: &str, key: &str) {
        let cache_key = format!("{}:{}", contract_id, key);
        let mut cache = self.cache.write().await;
        cache.pop(&cache_key);
    }

    fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }
}

/// Wrapper for the cache layer with symmetric latency tracking
pub struct CacheLayer {
    backend: Box<dyn ContractStateCache + Send + Sync>,
    config: CacheConfig,
}

impl CacheLayer {
    pub fn new(config: CacheConfig) -> Self {
        let backend: Box<dyn ContractStateCache + Send + Sync> = match config.policy {
            EvictionPolicy::Lfu => {
                Box::new(MokaLfuCache::new(config.max_capacity, config.global_ttl))
            }
            EvictionPolicy::Lru => {
                Box::new(LruCacheImpl::new(config.max_capacity, config.global_ttl))
            }
        };

        Self { backend, config }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get from cache with full instrumentation
    /// Returns (value, was_hit)
    pub async fn get(&self, contract_id: &str, key: &str) -> (Option<String>, bool) {
        if !self.config.enabled {
            return (None, false);
        }

        let result = self.backend.get(contract_id, key).await;

        // Record cache miss latency if this was a miss
        if !result.was_hit {
            self.backend
                .metrics()
                .cache_miss_latency_sum_micros
                .fetch_add(result.lookup_latency_micros, Ordering::Relaxed);
            self.backend
                .metrics()
                .cache_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }

        (result.value, result.was_hit)
    }

    pub async fn put(
        &self,
        contract_id: &str,
        key: &str,
        value: String,
        ttl_override: Option<Duration>,
    ) {
        if !self.config.enabled {
            return;
        }
        self.backend
            .put(contract_id, key, value, ttl_override)
            .await;
    }

    pub async fn invalidate(&self, contract_id: &str, key: &str) {
        if !self.config.enabled {
            return;
        }
        self.backend.invalidate(contract_id, key).await;
    }

    pub fn metrics(&self) -> &CacheMetrics {
        self.backend.metrics()
    }

    /// Record uncached baseline latency (for cache=off requests)
    pub fn record_uncached_latency(&self, duration: Duration) {
        let micros = duration.as_micros() as usize;
        self.backend
            .metrics()
            .uncached_latency_sum_micros
            .fetch_add(micros, Ordering::Relaxed);
        self.backend
            .metrics()
            .uncached_count
            .fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_flow() {
        let config = CacheConfig {
            enabled: true,
            policy: EvictionPolicy::Lfu,
            global_ttl: Duration::from_secs(60),
            max_capacity: 100,
        };
        let cache = CacheLayer::new(config);

        cache.put("c1", "k1", "v1".to_string(), None).await;

        let (val, was_hit) = cache.get("c1", "k1").await;
        assert_eq!(val, Some("v1".to_string()));
        assert!(was_hit);

        // Miss
        let (val2, was_hit2) = cache.get("c1", "k2").await;
        assert!(val2.is_none());
        assert!(!was_hit2);
    }

    #[tokio::test]
    async fn test_invalidation() {
        let config = CacheConfig::default();
        let cache = CacheLayer::new(config);

        cache.put("c1", "k1", "v1".to_string(), None).await;
        cache.invalidate("c1", "k1").await;

        let (val, _) = cache.get("c1", "k1").await;
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn test_ttl_lru() {
        let config = CacheConfig {
            enabled: true,
            policy: EvictionPolicy::Lru,
            global_ttl: Duration::from_millis(50),
            max_capacity: 100,
        };
        let cache = CacheLayer::new(config);

        cache.put("c1", "k1", "v1".to_string(), None).await;

        // Immediate get
        let (val, was_hit) = cache.get("c1", "k1").await;
        assert_eq!(val, Some("v1".to_string()));
        assert!(was_hit);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        let (val2, _) = cache.get("c1", "k1").await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_per_key_ttl_override() {
        let config = CacheConfig {
            enabled: true,
            policy: EvictionPolicy::Lru,
            global_ttl: Duration::from_secs(60),
            max_capacity: 100,
        };
        let cache = CacheLayer::new(config);

        // Put with short override
        cache
            .put(
                "c1",
                "k1",
                "v1".to_string(),
                Some(Duration::from_millis(50)),
            )
            .await;

        let (val, was_hit) = cache.get("c1", "k1").await;
        assert!(was_hit);

        // Wait for override TTL
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired (override, not global)
        let (val2, _) = cache.get("c1", "k1").await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_metrics_symmetric() {
        let config = CacheConfig::default();
        let cache = CacheLayer::new(config);

        cache.put("c1", "k1", "v1".to_string(), None).await;

        cache.get("c1", "k1").await; // Hit
        cache.get("c1", "k2").await; // Miss

        let m = cache.metrics();
        assert_eq!(m.hits.load(Ordering::Relaxed), 1);
        assert_eq!(m.misses.load(Ordering::Relaxed), 1);
        assert_eq!(m.hit_rate(), 50.0);

        // Verify latencies are recorded
        assert!(m.cached_hit_count.load(Ordering::Relaxed) > 0);
        assert!(m.cached_hit_latency_sum_micros.load(Ordering::Relaxed) > 0);
        assert!(m.cache_miss_count.load(Ordering::Relaxed) > 0);
        assert!(m.cache_miss_latency_sum_micros.load(Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn test_disabled() {
        let config = CacheConfig {
            enabled: false,
            ..CacheConfig::default()
        };
        let cache = CacheLayer::new(config);

        cache.put("c1", "k1", "v1".to_string(), None).await;
        let (val, _) = cache.get("c1", "k1").await;
        assert!(val.is_none());
    }
}
