# Alerting Rules and Runbooks

## Overview

This document defines alert rules, thresholds, runbooks for incident response, and on-call procedures for the Soroban Registry platform.

## Alert Severity Levels

| Severity | Description | Response Time | Escalation |
|----------|-------------|---------------|------------|
| **Critical** | Service down or severely degraded | Immediate (< 5 min) | Page on-call engineer |
| **Warning** | Degraded performance or potential issue | 30 minutes | Slack notification, review during business hours |
| **Info** | Informational, no action needed | N/A | Log only |

## Alert Configuration

Alerts are defined in `observability/prometheus/alerts.yml` and evaluated by Prometheus every 15 seconds.

### AlertManager Routing

```yaml
route:
  group_by: [alertname, severity]
  group_wait: 30s          # Wait before sending first alert
  group_interval: 5m       # Wait before sending new alerts for same group
  repeat_interval: 4h      # Wait before repeating resolved alert

  routes:
    - match:
        severity: critical
      receiver: pagerduty-critical
      continue: true
    - match:
        severity: critical
      receiver: slack-critical
```

**Notification Channels:**
- **Critical**: PagerDuty + Slack `#soroban-incidents`
- **Warning**: Slack `#soroban-alerts`

## Critical Alerts

### 1. HighP99Latency

**Alert Rule:**

```yaml
- alert: HighP99Latency
  expr: histogram_quantile(0.99, sum(rate(soroban_verification_latency_seconds_bucket[5m])) by (le)) > 0.5
  for: 5m
  labels:
    severity: critical
    team: backend
  annotations:
    summary: "Verification P99 latency exceeds 500ms"
    description: "P99 verification latency is {{ $value | humanizeDuration }} (threshold 500ms)."
```

**Impact:** Users experience slow contract verification, affecting publishing workflow.

**Runbook:**

1. **Check current latency:**
   ```bash
   curl -s http://prometheus:9090/api/v1/query?query='histogram_quantile(0.99, sum(rate(soroban_verification_latency_seconds_bucket[5m])) by (le))' | jq
   ```

2. **Identify slow operations:**
   - Check Jaeger for verification traces > 500ms
   - Look for common patterns (large contracts, specific publishers)

3. **Check dependencies:**
   ```bash
   # Database connection pool
   curl -s http://api:3001/metrics | grep soroban_db_connections_active

   # RPC latency
   curl -s http://api:3001/metrics | grep soroban_indexer_rpc_latency
   ```

4. **Common causes and fixes:**

   | Cause | Symptoms | Fix |
   |-------|----------|-----|
   | Database slow queries | High `soroban_db_query_duration_seconds` | Add indexes, optimize queries |
   | RPC endpoint slow | High `soroban_indexer_rpc_latency_seconds` | Switch RPC endpoint, increase timeout |
   | Verification queue backup | High `soroban_verification_queue_depth` | Scale verifier workers |
   | Large contract source | Logs show "source size exceeded" | Implement streaming verification |

5. **Immediate mitigation:**
   ```bash
   # Scale verifier service
   docker-compose up -d --scale verifier=3

   # Or restart with higher worker count
   docker-compose restart verifier
   ```

6. **Monitor recovery:**
   ```promql
   histogram_quantile(0.99, sum(rate(soroban_verification_latency_seconds_bucket[5m])) by (le))
   ```

**Escalation:** If latency doesn't improve within 15 minutes, page senior SRE.

---

### 2. HighErrorRate

**Alert Rule:**

```yaml
- alert: HighErrorRate
  expr: (sum(rate(soroban_http_requests_total{status=~"5.."}[5m])) / sum(rate(soroban_http_requests_total[5m]))) * 100 > 5
  for: 5m
  labels:
    severity: critical
    team: backend
  annotations:
    summary: "HTTP 5xx error rate exceeds 5%"
    description: "Current error rate is {{ $value | printf \"%.2f\" }}%."
```

**Impact:** Service degraded or unavailable for users.

**Runbook:**

1. **Check error rate by endpoint:**
   ```promql
   sum by (path) (rate(soroban_http_requests_total{status=~"5.."}[5m]))
   ```

2. **Examine recent logs:**
   ```bash
   # Docker logs
   docker-compose logs --tail=100 api | grep ERROR

   # Or Loki query
   # {service="soroban-api"} |= "ERROR" | json
   ```

3. **Common causes:**

   | Error Pattern | Likely Cause | Fix |
   |---------------|--------------|-----|
   | `DatabaseError` in logs | DB connection exhaustion | Increase pool size, check slow queries |
   | `RPC timeout` | Stellar RPC issues | Switch to backup RPC endpoint |
   | `InternalServerError` with panic | Code bug introduced | Rollback to previous version |
   | `ServiceUnavailable` | Downstream dependency down | Check indexer, verifier services |

4. **Check service health:**
   ```bash
   curl http://api:3001/health
   ```

5. **Immediate mitigation:**
   ```bash
   # Check if specific service is failing
   docker-compose ps

   # Restart unhealthy services
   docker-compose restart api

   # Rollback if recent deployment
   git log --oneline -5
   docker-compose down && git checkout <previous-commit> && docker-compose up -d
   ```

6. **Database connection issues:**
   ```bash
   # Check connection pool
   docker-compose exec api curl -s http://localhost:3001/metrics | grep db_connections

   # Temporary fix: increase pool size
   docker-compose exec -e DATABASE_POOL_SIZE=50 api sh -c 'kill -HUP 1'
   ```

**Escalation:** If error rate > 10% or affecting all endpoints, page engineering lead immediately.

---

### 3. SLOBurnRateHigh

**Alert Rule:**

```yaml
- alert: SLOBurnRateHigh
  expr: soroban_slo_burn_rate{slo="availability"} > 1
  for: 1h
  labels:
    severity: critical
    team: backend
  annotations:
    summary: "SLO burn rate >1 for availability"
    description: "Current burn rate is {{ $value | printf \"%.2f\" }}. Error budget being consumed faster than sustainable."
```

**Impact:** Error budget will be exhausted before end of month, risking SLO breach.

**Runbook:**

1. **Check current SLO status:**
   ```promql
   # Availability SLI (target: 99.9%)
   soroban_sli_availability

   # Error budget remaining
   1 - soroban_slo_burn_rate{slo="availability"}
   ```

2. **Identify contributing factors:**
   - Check for ongoing incidents
   - Review recent deployments
   - Check for elevated error rates or latency

3. **Calculate time to budget exhaustion:**
   ```python
   burn_rate = 2.5  # From metric
   hours_to_exhaustion = (1 / burn_rate) * 720  # 720 hours in 30 days
   print(f"Budget exhausted in {hours_to_exhaustion:.1f} hours")
   ```

4. **Actions:**
   - **Burn rate 1-2x**: Monitor closely, investigate root cause
   - **Burn rate 2-5x**: Prioritize fixes, defer new features
   - **Burn rate >5x**: Emergency response, halt all non-critical changes

5. **Mitigation strategies:**
   - Roll back recent risky changes
   - Implement circuit breakers for failing dependencies
   - Add caching to reduce load
   - Increase capacity/resources

**Escalation:** If burn rate > 10x, initiate incident response process.

---

## Warning Alerts

### 4. DatabaseConnectionExhaustion

**Alert Rule:**

```yaml
- alert: DatabaseConnectionExhaustion
  expr: soroban_db_connections_active / soroban_db_pool_size > 0.8
  for: 3m
  labels:
    severity: warning
    team: infra
  annotations:
    summary: "DB connection pool >80% utilized"
    description: "{{ $value | printf \"%.0f\" }}% of pool connections in use."
```

**Impact:** Risk of connection exhaustion leading to errors.

**Runbook:**

1. **Check current utilization:**
   ```bash
   curl -s http://api:3001/metrics | grep -E '(db_connections_active|db_pool_size)'
   ```

2. **Identify connection leaks:**
   ```bash
   # Check database for idle connections
   docker-compose exec database psql -U soroban -c "
     SELECT pid, usename, application_name, state, query_start, state_change
     FROM pg_stat_activity
     WHERE state != 'idle'
     ORDER BY query_start;
   "
   ```

3. **Check for slow queries holding connections:**
   ```sql
   SELECT pid, now() - query_start AS duration, query
   FROM pg_stat_activity
   WHERE state = 'active' AND now() - query_start > interval '30 seconds'
   ORDER BY duration DESC;
   ```

4. **Temporary mitigation:**
   ```bash
   # Increase pool size (requires restart)
   # Edit docker-compose.yml or set env var:
   DATABASE_POOL_SIZE=50

   docker-compose restart api
   ```

5. **Long-term fixes:**
   - Add connection timeouts
   - Implement connection pooling at proxy level (PgBouncer)
   - Optimize slow queries
   - Reduce connection lifetime

**Escalation:** If utilization reaches 95%, treat as critical.

---

### 5. CacheHitRateLow

**Alert Rule:**

```yaml
- alert: CacheHitRateLow
  expr: rate(soroban_cache_hits_total[10m]) / (rate(soroban_cache_hits_total[10m]) + rate(soroban_cache_misses_total[10m])) < 0.5
  for: 10m
  labels:
    severity: warning
    team: backend
  annotations:
    summary: "Cache hit ratio below 50%"
    description: "Cache hit rate is {{ $value | printf \"%.2f\" }}."
```

**Impact:** Increased database load, slower response times.

**Runbook:**

1. **Check cache statistics:**
   ```bash
   curl -s http://api:3001/metrics | grep cache
   ```

2. **Analyze cache effectiveness:**
   ```promql
   # Hit rate by cache
   sum by (cache_name) (rate(soroban_cache_hits_total[10m])) /
     (sum by (cache_name) (rate(soroban_cache_hits_total[10m])) +
      sum by (cache_name) (rate(soroban_cache_misses_total[10m])))
   ```

3. **Common causes:**
   - Cache evictions due to size limits
   - Short TTL values
   - Recent deployment cleared cache
   - Traffic pattern changed (new queries not cached)

4. **Actions:**
   ```bash
   # Check cache memory usage
   curl -s http://api:3001/metrics | grep soroban_cache_size_bytes

   # Check eviction rate
   curl -s http://api:3001/metrics | grep soroban_cache_evictions_total
   ```

5. **Mitigation:**
   ```bash
   # Increase cache size (requires restart)
   CACHE_SIZE_MB=512
   docker-compose restart api

   # Or warm cache manually
   curl -X POST http://api:3001/admin/cache/warm
   ```

**Escalation:** Generally not critical unless combined with high latency or error rate.

---

### 6. VerificationQueueBacklog

**Alert Rule:**

```yaml
- alert: VerificationQueueBacklog
  expr: soroban_verification_queue_depth > 100
  for: 5m
  labels:
    severity: warning
    team: backend
  annotations:
    summary: "Verification queue depth exceeds 100"
    description: "Queue depth is {{ $value }}."
```

**Impact:** Contract verification delays, poor user experience.

**Runbook:**

1. **Check queue depth:**
   ```bash
   curl -s http://api:3001/metrics | grep soroban_verification_queue_depth
   ```

2. **Check verifier worker status:**
   ```bash
   docker-compose ps verifier
   docker-compose logs --tail=50 verifier
   ```

3. **Investigate slow verifications:**
   - Check Jaeger for slow verification traces
   - Look for patterns (large contracts, specific compilers)

4. **Mitigation:**
   ```bash
   # Scale verifier workers
   docker-compose up -d --scale verifier=5

   # Check processing rate
   watch -n 5 'curl -s http://api:3001/metrics | grep verification_queue_depth'
   ```

5. **Drain queue manually (if needed):**
   ```bash
   # Process oldest items first
   curl -X POST http://api:3001/admin/verification/drain?max_age=3600
   ```

**Escalation:** If queue > 500 or growing rapidly, escalate to engineering.

---

### 7. HighMigrationFailureRate

**Alert Rule:**

```yaml
- alert: HighMigrationFailureRate
  expr: rate(soroban_migration_failures_total[10m]) / rate(soroban_migration_total[10m]) > 0.1
  for: 10m
  labels:
    severity: warning
    team: backend
  annotations:
    summary: "Migration failure rate exceeds 10%"
    description: "{{ $value | printf \"%.0f\" }}% of migrations failing."
```

**Impact:** Users unable to migrate contracts to new versions.

**Runbook:**

1. **Check migration logs:**
   ```bash
   docker-compose logs api | grep -i migration | grep -i error
   ```

2. **Identify failing migrations:**
   ```sql
   SELECT contract_id, from_version, to_version, error_message, created_at
   FROM migration_attempts
   WHERE status = 'failed'
   ORDER BY created_at DESC
   LIMIT 20;
   ```

3. **Common failure reasons:**
   - Incompatible contract versions
   - Missing dependencies
   - State migration script errors
   - Insufficient permissions

4. **Actions:**
   - Review migration compatibility matrix
   - Check contract state validation
   - Verify migration scripts are up to date

5. **Temporary fix:**
   ```bash
   # Disable auto-migrations if widespread failures
   curl -X POST http://api:3001/admin/migrations/pause

   # Investigate and fix root cause, then resume
   curl -X POST http://api:3001/admin/migrations/resume
   ```

**Escalation:** If failure rate > 50%, disable migrations and page engineering.

---

## On-Call Procedures

### On-Call Rotation

- **Primary**: First responder for all alerts
- **Secondary**: Backup if primary doesn't respond within 10 minutes
- **Escalation**: Engineering lead for critical incidents

### Response Time SLAs

| Severity | Acknowledgment | Initial Response | Resolution Target |
|----------|----------------|------------------|-------------------|
| Critical | 5 minutes | 15 minutes | 1 hour |
| Warning | 30 minutes | 1 hour | Next business day |

### Incident Response Process

1. **Acknowledge alert** in PagerDuty/Slack
2. **Assess severity** and impact
3. **Start incident channel** in Slack: `/incident start <title>`
4. **Investigate** using runbook
5. **Mitigate** immediate impact
6. **Communicate** status updates every 30 minutes
7. **Resolve** root cause
8. **Post-mortem** for critical incidents (within 48 hours)

### Communication Templates

**Initial Response (Critical):**
```
🔴 INCIDENT: <Title>
Status: Investigating
Impact: <User-facing impact>
Started: <Time>
Responder: @<Name>
Updates: Every 30 minutes
```

**Status Update:**
```
UPDATE: <Title>
Progress: <What's been done>
Current theory: <Root cause hypothesis>
Next steps: <Actions being taken>
ETA: <Expected resolution time>
```

**Resolution:**
```
✅ RESOLVED: <Title>
Duration: <Total time>
Root cause: <Brief explanation>
Fix: <What was done>
Follow-up: <Post-mortem ticket link>
```

### Escalation Paths

1. **Level 1**: On-call engineer (primary)
2. **Level 2**: On-call engineer (secondary) - after 10 min no response
3. **Level 3**: Engineering lead - for critical incidents or >1 hour unresolved
4. **Level 4**: Director of Engineering - for major outages or business impact

## Alert Inhibition Rules

Prevent alert spam by suppressing lower-severity alerts when critical alerts fire:

```yaml
inhibit_rules:
  - source_match:
      severity: critical
    target_match:
      severity: warning
    equal: [alertname]
```

**Example:** If `HighErrorRate` (critical) fires, suppress `CacheHitRateLow` (warning) for same service.

## Testing Alerts

Regularly test alert pipelines:

```bash
# Send test alert to AlertManager
curl -XPOST http://alertmanager:9093/api/v1/alerts -d '[
  {
    "labels": {
      "alertname": "TestAlert",
      "severity": "warning",
      "team": "backend"
    },
    "annotations": {
      "summary": "Test alert - please acknowledge",
      "description": "This is a test alert to verify notification channels."
    }
  }
]'
```

**Testing Schedule:** Monthly test of all notification channels.

## Alert Tuning

Regularly review and tune alert thresholds:

1. **Track false positives** (alerts that don't require action)
2. **Track missed incidents** (incidents without alerts)
3. **Adjust thresholds** quarterly based on baseline metrics
4. **Remove noisy alerts** that cause alert fatigue

**Metrics:**
- Alert fatigue rate: `< 10%` false positives
- Coverage rate: `> 95%` incidents have alerts
- Time to acknowledge: `< 5 minutes` for critical

## Related Documentation

- [OBSERVABILITY.md](./OBSERVABILITY.md) - Metrics and monitoring setup
- [ERROR_CODES.md](./ERROR_CODES.md) - Error code reference for debugging
- [INCIDENT_RESPONSE.md](./INCIDENT_RESPONSE.md) - Detailed incident response procedures

## Support

For alert configuration issues:
- GitHub Issues: https://github.com/ALIPHATICHYD/Soroban-Registry/issues
- Tag with: `alerting`, `observability`
