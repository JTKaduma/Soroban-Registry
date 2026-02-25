# Backend Performance Tuning & Optimization Guidelines

As the Soroban Registry grows, maintaining optimal performance is critical. This guide outlines the strategies, tools, and baselines required to keep our backend responsive and scalable.

## Performance Baselines & Targets
When implementing new features or optimizing existing ones, aim for the following targets:
* **API Response Times:** `< 100ms` for 95th percentile (p95) of standard queries.
* **Cache Hit Ratios:** `> 80%` for frequently accessed read-heavy endpoints.
* **Database Connection Wait Times:** `< 10ms` during peak load.

## 1. Database Query Optimization
Inefficient queries are the most common source of backend bottlenecks.

### Using `EXPLAIN ANALYZE`
Always profile complex PostgreSQL queries before merging. `EXPLAIN ANALYZE` executes the query and provides actual run times and execution plans.

```sql
-- Before: Sequential scan (Slow)
EXPLAIN ANALYZE SELECT * FROM contracts WHERE author_id = 'xyz';

-- After Optimization: Add an index to author_id
CREATE INDEX idx_contracts_author ON contracts(author_id);
-- Now uses Index Scan (Fast)
```

## 2. Caching Strategies
Caching reduces database load and network latency. We utilize a multi-tier caching strategy:
* **In-Memory Cache (Local):** Use for static, rarely changing configuration data.
* **Redis (Distributed):** Use for frequently accessed, computationally expensive data.
* **HTTP Caching:** Utilize `Cache-Control` headers for idempotent API responses.

## 3. Connection Pooling
Do not open a new database connection per request. Configure our connection pooler to maintain a healthy pool of active connections.

## 4. Identifying Slow Queries
* **Postgres `pg_stat_statements`:** Use this to find the top 10 slowest queries.
* **Application Logs:** Any query taking longer than 500ms should log a warning.

## 5. Load Testing
Before major releases, simulate traffic spikes to validate our baselines using tools like **k6** or **Vegeta**.