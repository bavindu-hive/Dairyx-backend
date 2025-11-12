#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:3000/DairyX"

# Get authentication token
echo -e "${BLUE}=== Getting Authentication Token ===${NC}"
TOKEN=$(curl -s -X POST ${BASE_URL}/users/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq -r '.token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo -e "${RED}❌ Failed to get authentication token${NC}"
  exit 1
fi

echo -e "${GREEN}✓ Token obtained${NC}"
echo

# Test 1: Get batch movements
echo -e "${BLUE}=== Test 1: GET /stock-movements/batches/5 ===${NC}"
echo "Getting complete movement history for batch 5 (Butter-241110-A)..."
curl -s -X GET ${BASE_URL}/stock-movements/batches/5 \
  -H "Authorization: Bearer $TOKEN" | jq '.'
echo
echo

# Test 2: Get daily movements
echo -e "${BLUE}=== Test 2: GET /stock-movements/daily/2025-11-10 ===${NC}"
echo "Getting daily stock movement summary for 2025-11-10..."
curl -s -X GET ${BASE_URL}/stock-movements/daily/2025-11-10 \
  -H "Authorization: Bearer $TOKEN" | jq '.'
echo
echo

# Test 3: Get product movements
echo -e "${BLUE}=== Test 3: GET /stock-movements/products/2 ===${NC}"
echo "Getting all movements for product 2 (Butter)..."
curl -s -X GET "${BASE_URL}/stock-movements/products/2?start_date=2025-11-01&end_date=2025-11-12" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
echo
echo

# Test 4: Create stock adjustment
echo -e "${BLUE}=== Test 4: POST /stock-movements/adjust ===${NC}"
echo "Creating a stock adjustment (adding 5 units to batch 5)..."
curl -s -X POST ${BASE_URL}/stock-movements/adjust \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "batch_id": 5,
    "product_id": 2,
    "quantity": 5,
    "reason": "Stock correction",
    "notes": "Found 5 extra units during inventory check"
  }' | jq '.'
echo
echo

# Test 5: Verify adjustment was logged
echo -e "${BLUE}=== Test 5: Verify adjustment in batch movements ===${NC}"
echo "Getting updated movement history for batch 5..."
curl -s -X GET ${BASE_URL}/stock-movements/batches/5 \
  -H "Authorization: Bearer $TOKEN" | jq '.movements | last'
echo
echo

# Test 6: Start reconciliation
echo -e "${BLUE}=== Test 6: POST /reconciliations/start ===${NC}"
echo "Starting reconciliation for 2025-11-10..."
RECON_RESPONSE=$(curl -s -X POST ${BASE_URL}/reconciliations/start \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reconciliation_date": "2025-11-10",
    "notes": "End of day reconciliation test"
  }')
echo "$RECON_RESPONSE" | jq '.'
echo
echo

# Test 7: Get reconciliation
RECON_DATE=$(echo "$RECON_RESPONSE" | jq -r '.reconciliation_date')
echo -e "${BLUE}=== Test 7: GET /reconciliations/$RECON_DATE ===${NC}"
echo "Getting reconciliation details..."
curl -s -X GET ${BASE_URL}/reconciliations/$RECON_DATE \
  -H "Authorization: Bearer $TOKEN" | jq '.'
echo
echo

# Test 8: Verify truck return (if trucks exist)
TRUCK_COUNT=$(echo "$RECON_RESPONSE" | jq -r '.trucks_out')
if [ "$TRUCK_COUNT" -gt 0 ]; then
  TRUCK_ID=$(echo "$RECON_RESPONSE" | jq -r '.truck_items[0].truck_id')
  echo -e "${BLUE}=== Test 8: POST /reconciliations/$RECON_DATE/trucks/$TRUCK_ID/verify ===${NC}"
  echo "Verifying truck $TRUCK_ID return..."
  curl -s -X POST ${BASE_URL}/reconciliations/$RECON_DATE/trucks/$TRUCK_ID/verify \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "items_returned": [],
      "items_discarded": [],
      "discrepancy_notes": null
    }' | jq '.'
  echo
  echo
else
  echo -e "${BLUE}=== Test 8: Skipped (no trucks) ===${NC}"
  echo
fi

# Test 9: List reconciliations
echo -e "${BLUE}=== Test 9: GET /reconciliations ===${NC}"
echo "Listing all reconciliations..."
curl -s -X GET ${BASE_URL}/reconciliations \
  -H "Authorization: Bearer $TOKEN" | jq '.'
echo
echo

echo -e "${GREEN}=== All Tests Completed ===${NC}"
