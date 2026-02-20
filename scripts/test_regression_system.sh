#!/bin/bash
# Test script for regression testing system
# Demonstrates the full workflow: baseline creation, test execution, and alert handling

set -e

API_URL="${API_URL:-http://localhost:3001}"
CONTRACT_ID="${1:-}"

if [ -z "$CONTRACT_ID" ]; then
    echo "Usage: $0 <contract_id>"
    echo "Example: $0 550e8400-e29b-41d4-a716-446655440000"
    exit 1
fi

echo "=========================================="
echo "Regression Testing System Demo"
echo "=========================================="
echo "Contract ID: $CONTRACT_ID"
echo "API URL: $API_URL"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Step 1: Create a test suite
echo -e "${YELLOW}Step 1: Creating test suite...${NC}"
SUITE_RESPONSE=$(curl -s -X POST "$API_URL/api/contracts/$CONTRACT_ID/regression/suites" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "core_functionality",
    "description": "Core contract functionality tests",
    "test_functions": [
      {
        "function": "initialize",
        "params": {}
      },
      {
        "function": "transfer",
        "params": {
          "from": "GABC123",
          "to": "GDEF456",
          "amount": 100
        }
      },
      {
        "function": "balance",
        "params": {
          "address": "GABC123"
        }
      }
    ],
    "performance_thresholds": {
      "minor": 10.0,
      "major": 25.0,
      "critical": 50.0
    },
    "auto_run_on_deploy": true,
    "created_by": "test_script"
  }')

echo "$SUITE_RESPONSE" | jq '.'
echo -e "${GREEN}✓ Test suite created${NC}\n"

# Step 2: Establish baseline for version 1.0.0
echo -e "${YELLOW}Step 2: Establishing baseline for version 1.0.0...${NC}"
BASELINE_RESPONSE=$(curl -s -X POST "$API_URL/api/contracts/$CONTRACT_ID/regression/baseline" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.0.0",
    "test_suite_name": "core_functionality",
    "function_name": "transfer",
    "output": {
      "result": "success",
      "from": "GABC123",
      "to": "GDEF456",
      "amount": 100,
      "new_balance_from": 900,
      "new_balance_to": 100
    },
    "established_by": "test_script"
  }')

echo "$BASELINE_RESPONSE" | jq '.'
BASELINE_ID=$(echo "$BASELINE_RESPONSE" | jq -r '.id')
echo -e "${GREEN}✓ Baseline established (ID: $BASELINE_ID)${NC}\n"

# Step 3: Get all baselines
echo -e "${YELLOW}Step 3: Retrieving all baselines...${NC}"
curl -s "$API_URL/api/contracts/$CONTRACT_ID/regression/baselines" | jq '.'
echo -e "${GREEN}✓ Baselines retrieved${NC}\n"

# Step 4: Run a single regression test
echo -e "${YELLOW}Step 4: Running single regression test...${NC}"
TEST_RESPONSE=$(curl -s -X POST "$API_URL/api/contracts/$CONTRACT_ID/regression/test" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.0.1",
    "test_suite_name": "core_functionality",
    "function_name": "transfer",
    "triggered_by": "manual"
  }')

echo "$TEST_RESPONSE" | jq '.'
TEST_ID=$(echo "$TEST_RESPONSE" | jq -r '.id')
REGRESSION_DETECTED=$(echo "$TEST_RESPONSE" | jq -r '.regression_detected')

if [ "$REGRESSION_DETECTED" = "true" ]; then
    echo -e "${RED}✗ Regression detected!${NC}\n"
else
    echo -e "${GREEN}✓ Test passed - no regression${NC}\n"
fi

# Step 5: Run full test suite
echo -e "${YELLOW}Step 5: Running full test suite...${NC}"
SUITE_RUN_RESPONSE=$(curl -s -X POST "$API_URL/api/contracts/$CONTRACT_ID/regression/suite" \
  -H "Content-Type: application/json" \
  -d '{
    "version": "1.0.1",
    "suite_name": "core_functionality",
    "triggered_by": "manual"
  }')

echo "$SUITE_RUN_RESPONSE" | jq '.'
TOTAL_RUNS=$(echo "$SUITE_RUN_RESPONSE" | jq -r '.total_runs')
PASSED=$(echo "$SUITE_RUN_RESPONSE" | jq -r '.passed')
FAILED=$(echo "$SUITE_RUN_RESPONSE" | jq -r '.failed')
REGRESSIONS=$(echo "$SUITE_RUN_RESPONSE" | jq -r '.regressions_detected')

echo -e "${GREEN}✓ Suite completed: $PASSED/$TOTAL_RUNS passed, $REGRESSIONS regressions detected${NC}\n"

# Step 6: Get test run history
echo -e "${YELLOW}Step 6: Retrieving test run history...${NC}"
curl -s "$API_URL/api/contracts/$CONTRACT_ID/regression/runs" | jq '.[:3]'
echo -e "${GREEN}✓ Test history retrieved${NC}\n"

# Step 7: Check for alerts
echo -e "${YELLOW}Step 7: Checking for regression alerts...${NC}"
ALERTS_RESPONSE=$(curl -s "$API_URL/api/contracts/$CONTRACT_ID/regression/alerts")
echo "$ALERTS_RESPONSE" | jq '.'
ALERT_COUNT=$(echo "$ALERTS_RESPONSE" | jq '. | length')

if [ "$ALERT_COUNT" -gt 0 ]; then
    echo -e "${RED}⚠ Found $ALERT_COUNT unresolved alerts${NC}\n"
    
    # Step 8: Acknowledge first alert
    FIRST_ALERT_ID=$(echo "$ALERTS_RESPONSE" | jq -r '.[0].id')
    if [ "$FIRST_ALERT_ID" != "null" ]; then
        echo -e "${YELLOW}Step 8: Acknowledging alert $FIRST_ALERT_ID...${NC}"
        curl -s -X POST "$API_URL/api/contracts/$CONTRACT_ID/regression/alerts/$FIRST_ALERT_ID/acknowledge" \
          -H "Content-Type: application/json" \
          -d '{
            "acknowledged_by": "test_script"
          }' | jq '.'
        echo -e "${GREEN}✓ Alert acknowledged${NC}\n"
    fi
else
    echo -e "${GREEN}✓ No alerts found${NC}\n"
fi

# Step 9: Get statistics
echo -e "${YELLOW}Step 9: Retrieving regression statistics (last 30 days)...${NC}"
STATS_RESPONSE=$(curl -s "$API_URL/api/contracts/$CONTRACT_ID/regression/statistics?days=30")
echo "$STATS_RESPONSE" | jq '.'

ACCURACY=$(echo "$STATS_RESPONSE" | jq -r '.detection_accuracy_percent // 0')
FPR=$(echo "$STATS_RESPONSE" | jq -r '.false_positive_rate_percent // 0')

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo "Detection Accuracy: $ACCURACY%"
echo "False Positive Rate: $FPR%"

# Check acceptance criteria
if (( $(echo "$ACCURACY >= 95.0" | bc -l) )); then
    echo -e "${GREEN}✓ Accuracy meets acceptance criteria (≥95%)${NC}"
else
    echo -e "${RED}✗ Accuracy below acceptance criteria (<95%)${NC}"
fi

if (( $(echo "$FPR < 2.0" | bc -l) )); then
    echo -e "${GREEN}✓ False positive rate meets acceptance criteria (<2%)${NC}"
else
    echo -e "${RED}✗ False positive rate above acceptance criteria (≥2%)${NC}"
fi

echo ""
echo -e "${GREEN}Regression testing system demo completed!${NC}"
