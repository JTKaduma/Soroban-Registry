# Regression Testing Quick Start Guide

Get started with contract regression testing in 5 minutes.

## Prerequisites

- API server running on `http://localhost:3001`
- Contract deployed to the registry
- Contract ID (UUID format)

## Quick Setup

### 1. Create a Test Suite

```bash
CONTRACT_ID="your-contract-uuid-here"

curl -X POST "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/suites" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "quick_test",
    "description": "Quick regression test suite",
    "test_functions": [
      {
        "function": "main_function",
        "params": {}
      }
    ],
    "auto_run_on_deploy": true
  }'
```

### 2. Establish a Baseline

```bash
curl -X POST "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/baseline" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.0.0",
    "test_suite_name": "quick_test",
    "function_name": "main_function",
    "output": {
      "result": "success",
      "value": 42
    }
  }'
```

### 3. Run a Test

```bash
curl -X POST "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/test" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.0.1",
    "test_suite_name": "quick_test",
    "function_name": "main_function",
    "triggered_by": "manual"
  }'
```

### 4. Check Results

```bash
# Get test runs
curl "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/runs"

# Check for alerts
curl "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/alerts"

# View statistics
curl "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/statistics?days=7"
```

## Automated Testing

Once you've created a test suite with `auto_run_on_deploy: true`, tests will automatically run when you deploy:

1. Deploy contract to green environment
2. Wait ~60 seconds for regression monitor
3. Tests run automatically
4. Check alerts if regressions detected

## Common Commands

### List all test suites

```bash
curl "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/suites"
```

### Get all baselines

```bash
curl "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/baselines"
```

### Run full test suite

```bash
curl -X POST "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/suite" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.0.1",
    "suite_name": "quick_test",
    "triggered_by": "manual"
  }'
```

### Acknowledge an alert

```bash
ALERT_ID="alert-uuid-here"

curl -X POST "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/alerts/$ALERT_ID/acknowledge" \
  -H "Content-Type: application/json" \
  -d '{
    "acknowledged_by": "your-name"
  }'
```

### Resolve an alert

```bash
curl -X POST "http://localhost:3001/api/contracts/$CONTRACT_ID/regression/alerts/$ALERT_ID/resolve" \
  -H "Content-Type: application/json" \
  -d '{
    "resolution_notes": "Fixed in version 1.0.2"
  }'
```

## Understanding Results

### Test Status

- `passed` - Test completed successfully, no regression
- `failed` - Test failed or regression detected
- `running` - Test currently executing
- `pending` - Test queued but not started

### Regression Severity

- `none` - No regression detected
- `minor` - 10-25% performance degradation
- `major` - 25-50% performance degradation or output mismatch
- `critical` - >50% performance degradation

### Key Metrics

- **Detection Accuracy** - Should be ≥95%
- **False Positive Rate** - Should be <2%
- **Performance Degradation** - Percentage slower than baseline

## Troubleshooting

### Tests not running automatically?

1. Check test suite has `auto_run_on_deploy: true`
2. Verify deployment is in 'testing' status
3. Wait 60 seconds for monitor cycle
4. Check API logs for errors

### High false positive rate?

1. Adjust performance thresholds in test suite
2. Mark false positives when resolving alerts
3. Consider environmental factors

### No baseline found?

1. Ensure baseline was created successfully
2. Check baseline is active: `is_active = true`
3. Verify version, suite name, and function name match

## Next Steps

- Read full documentation: `docs/REGRESSION_TESTING.md`
- Run demo script: `./scripts/test_regression_system.sh $CONTRACT_ID`
- Set up custom thresholds for your contracts
- Integrate with your deployment pipeline

## Support

For detailed information, see:

- Full documentation: `docs/REGRESSION_TESTING.md`
- Implementation details: `REGRESSION_TESTING_IMPLEMENTATION.md`
- API reference in documentation
