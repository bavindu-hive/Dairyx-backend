#!/bin/bash

echo "=== DairyX Sales API Testing ==="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Base URL
BASE_URL="http://127.0.0.1:3000"

echo -e "${BLUE}Step 1: Login as driver1${NC}"
LOGIN_RESPONSE=$(curl -s -X POST $BASE_URL/users/login \
  -H "Content-Type: application/json" \
  -d '{"username":"driver1","password":"password"}')

TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo -e "${RED}❌ Login failed${NC}"
  echo $LOGIN_RESPONSE | jq .
  exit 1
fi

echo -e "${GREEN}✓ Login successful${NC}"
echo "Token: ${TOKEN:0:20}..."
echo ""

echo -e "${BLUE}Step 2: Create a sale (with default wholesale prices)${NC}"
SALE_RESPONSE=$(curl -s -X POST $BASE_URL/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "amount_paid": 500.0,
    "items": [
      {
        "product_id": 3,
        "quantity": 5
      },
      {
        "product_id": 2,
        "quantity": 10
      }
    ]
  }')

SALE_ID=$(echo $SALE_RESPONSE | jq -r '.id')

if [ "$SALE_ID" == "null" ] || [ -z "$SALE_ID" ]; then
  echo -e "${RED}❌ Sale creation failed${NC}"
  echo $SALE_RESPONSE | jq .
  exit 1
fi

echo -e "${GREEN}✓ Sale created successfully${NC}"
echo "Sale ID: $SALE_ID"
echo ""
echo "Response:"
echo $SALE_RESPONSE | jq '.'
echo ""

echo -e "${BLUE}Step 3: Get sale details${NC}"
GET_RESPONSE=$(curl -s $BASE_URL/sales/$SALE_ID)
echo $GET_RESPONSE | jq '.'
echo ""

echo -e "${BLUE}Step 4: List all sales${NC}"
LIST_RESPONSE=$(curl -s "$BASE_URL/sales")
echo $LIST_RESPONSE | jq '.'
echo ""

echo -e "${BLUE}Step 5: Update payment (add 200 more)${NC}"
PAYMENT_RESPONSE=$(curl -s -X PATCH $BASE_URL/sales/$SALE_ID/payment \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "additional_payment": 200.0
  }')

echo $PAYMENT_RESPONSE | jq '.'
echo ""

echo -e "${BLUE}Step 6: Filter sales by payment status${NC}"
PENDING_SALES=$(curl -s "$BASE_URL/sales?payment_status=pending")
echo "Pending sales:"
echo $PENDING_SALES | jq '.'
echo ""

echo -e "${GREEN}=== All tests completed! ===${NC}"
