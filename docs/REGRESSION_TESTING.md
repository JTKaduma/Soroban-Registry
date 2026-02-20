# Contract Regression Testing

Comprehensive automated regression testing system for Soroban smart contracts.

## Overview

The regression testing system automatically runs tests on each contract version to catch regressions before they impact users. It establishes performance baselines, compares new versions against those baselines, and alerts when regressions are detected.

## Features

### 1. Baseline Management

- Establish performance baselines for major contract versions
- Track execution time, memory usage, CPU instructions, and storage I/O
- Store output snapshots for functional regression detection
- Support multiple baselines per contract (one per major version)

### 2. Automated Test Execution

- Auto-run tests on each deployment
- Support for test suites with multiple test functions
- Configurable test parameters per function
- Integration with blue-green deployment workflow

### 3. Regression Detection

- Performance degradation detection with configurable thresholds:
  - Minor: >10% slower than baseline
  - Major: >25% slower than baseline
  - Critical: >50% slower than baseline
- Output comparison using SHA-256 hashing
- Functional regression detection (output mismatch)

### 4. Alerting System

- Real-time alerts when regressions are detected
- Severity-based classification (minor, major, critical)
- Alert acknowledgment and resolution workflow
- Support for multiple notification channels (email, Slack, webhook)

### 5. Statistics & Monitoring

- Detection accuracy tracking (target: ≥95%)
- False positive rate monitoring (target: <2%)
- Performance trend analysis
- Historical test run data

## API Endpoints

### Baseline Management

#### Establish Baseline

```http
POST /api/contracts/:id/regression/baseline
Content-Type: application/json

{
  "version": "1.0.0",
  "test_suite_name": "core_tests",
  "function_name": "transfer",
  "output": {
    "result": "success",
    "value": 100
  },
  "established_by": "admin@example.com"
}
```

#### Get Baselines

```http
GET /api/contracts/:id/regression/baselines
```

### Test Execution

#### Run Single Test

```http
POST /api/contracts/:id/regression/test
Content-Type: application/json

{
  "version": "1.1.0",
  "test_suite_name": "core_tests",
  "function_name": "transfer",
  "triggered_by": "deployment",
  "deployment_id": "uuid-here"
}
```

#### Run Test Suite

```http
POST /api/contracts/:id/regression/suite
Content-Type: application/json

{
  "version": "1.1.0",
  "suite_name": "core_tests",
  "triggered_by": "deployment",
  "deployment_id": "uuid-here"
}
```

#### Get Test Runs

```http
GET /api/contracts/:id/regression/runs
```

### Test Suite Management

#### Create Test Suite

```http
POST /api/contracts/:id/regression/suites
Content-Type: application/json

{
  "name": "integration_tests",
  "description": "Full integration test suite",
  "test_functions": [
    {
      "function": "initialize",
      "params": {}
    },
    {
      "function": "transfer",
      "params": {
        "from": "GABC...",
        "to": "GDEF...",
        "amount": 100
      }
    }
  ],
  "performance_thresholds": {
    "minor": 10.0,
    "major": 25.0,
    "critical": 50.0
  },
  "auto_run_on_deploy": true,
  "created_by": "admin@example.com"
}
```

#### Get Test Suites

```http
GET /api/contracts/:id/regression/suites
```

### Alert Management

#### Get Alerts

```http
GET /api/contracts/:id/regression/alerts
```

#### Acknowledge Alert

```http
POST /api/contracts/:id/regression/alerts/:alert_id/acknowledge
Content-Type: application/json

{
  "acknowledged_by": "admin@example.com"
}
```

#### Resolve Alert

```http
POST /api/contracts/:id/regression/alerts/:alert_id/resolve
Content-Type: application/json

{
  "resolution_notes": "False positive - expected behavior change"
}
```

### Statistics

#### Get Statistics

```http
GET /api/contracts/:id/regression/statistics?days=30
```

Response:

```json
{
  "contract_id": "uuid",
  "period_start": "2024-01-01T00:00:00Z",
  "period_end": "2024-01-31T23:59:59Z",
  "total_runs": 150,
  "passed_runs": 145,
  "failed_runs": 5,
  "regressions_detected": 3,
  "false_positives": 0,
  "true_positives": 3,
  "detection_accuracy_percent": 100.0,
  "false_positive_rate_percent": 0.0,
  "avg_execution_time_ms": 12.5,
  "avg_degradation_percent": 2.3
}
```

## Database Schema

### Tables

- `regression_test_baselines` - Performance and output baselines
- `regression_test_runs` - Individual test execution records
- `regression_test_suites` - Test suite definitions
- `regression_alerts` - Regression detection alerts
- `regression_test_statistics` - Aggregated statistics

### Enums

- `test_status`: pending, running, passed, failed, skipped
- `regression_severity`: none, minor, major, critical

## Background Services

### Regression Monitor

- Runs every 60 seconds
- Checks for new deployments in 'testing' status
- Automatically runs test suites with `auto_run_on_deploy = true`
- Logs results and creates alerts for regressions

### Statistics Calculator

- Runs every hour
- Calculates regression statistics for all active contracts
- Updates detection accuracy and false positive rates
- Maintains 30-day rolling statistics

## Workflow Integration

### Blue-Green Deployment Integration

1. Deploy to green environment
2. Regression tests automatically triggered
3. Tests run against established baselines
4. If regressions detected:
   - Alerts created with severity level
   - Deployment remains in 'testing' status
   - Manual review required before switch
5. If all tests pass:
   - Deployment can be switched to active
   - Statistics updated

### Manual Testing Workflow

1. Create test suite for contract
2. Establish baseline on stable version
3. Deploy new version
4. Run test suite manually or wait for auto-run
5. Review results and alerts
6. Acknowledge/resolve alerts as needed

## Configuration

### Performance Thresholds

Default thresholds (can be customized per test suite):

- Minor regression: 10% performance degradation
- Major regression: 25% performance degradation
- Critical regression: 50% performance degradation

### Test Execution

- Warmup iterations: 10% of total (min 5, max 20)
- Measurement iterations: 30-50 (configurable)
- Timeout: 60 seconds per test

## Acceptance Criteria

✅ Regression tests run automatically on deployment
✅ Baselines established and tracked per major version
✅ Regressions detected with 95%+ accuracy
✅ Alerts delivered in real-time
✅ False positive rate <2%

## Monitoring

Key metrics exposed via Prometheus:

- `soroban_regression_tests_total` - Total test runs
- `soroban_regression_tests_failed` - Failed test runs
- `soroban_regressions_detected` - Regressions detected
- `soroban_regression_detection_accuracy` - Detection accuracy %
- `soroban_regression_false_positive_rate` - False positive rate %

## Best Practices

1. **Establish baselines early** - Create baselines for stable versions before making changes
2. **Use comprehensive test suites** - Cover all critical functions
3. **Set appropriate thresholds** - Adjust based on contract complexity
4. **Review alerts promptly** - Investigate regressions quickly
5. **Mark false positives** - Help improve detection accuracy
6. **Monitor statistics** - Track trends over time

## Troubleshooting

### Tests not running automatically

- Check that test suite has `auto_run_on_deploy = true`
- Verify deployment is in 'testing' status
- Check background service logs

### High false positive rate

- Review and adjust performance thresholds
- Mark false positives in alert resolution
- Consider environmental factors (load, network)

### Missing baselines

- Establish baselines for major versions
- Ensure baseline establishment succeeds
- Check baseline activation status

## Future Enhancements

- [ ] Snapshot testing for complex outputs
- [ ] Performance regression prediction
- [ ] Automated rollback on critical regressions
- [ ] Integration with CI/CD pipelines
- [ ] Custom notification channels
- [ ] Test result visualization dashboard
