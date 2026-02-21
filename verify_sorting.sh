#!/bin/bash

BASE_URL="http://localhost:3000/api/contracts"

echo "Testing default sorting (created_at DESC)..."
curl -s "${BASE_URL}" | jq '.items[0].name'

echo -e "\nTesting popularity (interactions) DESC..."
curl -s "${BASE_URL}?sort_by=popularity&sort_order=desc" | jq '.items[0].name'

echo -e "\nTesting deployments DESC..."
curl -s "${BASE_URL}?sort_by=deployments&sort_order=desc" | jq '.items[0].name'

echo -e "\nTesting search relevance..."
curl -s "${BASE_URL}?query=soroban&sort_by=relevance" | jq '.items[0].name'

echo -e "\nVerification complete."
