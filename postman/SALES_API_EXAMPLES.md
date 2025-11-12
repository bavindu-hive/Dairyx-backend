# Sales API Examples

## Prerequisites
- Server running on `http://127.0.0.1:3000`
- Database has the following test data:
  - Users: driver1 (id:2), manager (id:1)
  - Trucks: TRUCK-1 (id:1, driver: driver1)
  - Shops: Downtown Grocery (id:1)
  - Truck Load: id=3 (truck_id=1, loaded today with Butter and Yogurt)

## 1. Login to Get Token

### Login as Driver
```bash
curl -X POST http://127.0.0.1:3000/users/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "driver1",
    "password": "password"
  }'
```

**Response:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "user": {
    "id": 2,
    "username": "driver1",
    "role": "driver"
  }
}
```

**Save the token:**
```bash
export DRIVER_TOKEN="<paste_token_here>"
```

---

## 2. Create a Sale (Driver Only)

### Example 1: Create sale with default wholesale prices
```bash
curl -X POST http://127.0.0.1:3000/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "amount_paid": 500.0,
    "items": [
      {
        "product_id": 1,
        "quantity": 10
      },
      {
        "product_id": 2,
        "quantity": 15
      }
    ]
  }'
```

**Response:**
```json
{
  "id": 1,
  "shop_id": 1,
  "shop_name": "Downtown Grocery",
  "truck_id": 1,
  "truck_number": "TRUCK-1",
  "driver_id": 2,
  "driver_username": "driver1",
  "truck_load_id": 3,
  "total_amount": 1350.0,
  "amount_paid": 500.0,
  "payment_status": "pending",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:30:00Z",
  "items": [
    {
      "id": 1,
      "product_id": 1,
      "product_name": "Butter 500g",
      "batch_id": 5,
      "batch_number": "BATCH-2025-11-10-001",
      "quantity": 10,
      "unit_price": 75.0,
      "commission_earned": 50.0,
      "line_total": 750.0
    },
    {
      "id": 2,
      "product_id": 2,
      "product_name": "Yogurt Cup",
      "batch_id": 6,
      "batch_number": "BATCH-2025-11-10-002",
      "quantity": 15,
      "unit_price": 30.0,
      "commission_earned": 75.0,
      "line_total": 450.0
    }
  ],
  "summary": {
    "total_items": 25,
    "total_commission": 125.0,
    "balance_due": 850.0
  }
}
```

### Example 2: Create sale with custom prices (negotiated)
```bash
curl -X POST http://127.0.0.1:3000/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "amount_paid": 0,
    "items": [
      {
        "product_id": 1,
        "quantity": 5,
        "unit_price": 80.0
      },
      {
        "product_id": 2,
        "quantity": 10,
        "unit_price": 35.0
      }
    ]
  }'
```

**Response:**
```json
{
  "id": 2,
  "shop_id": 1,
  "shop_name": "Downtown Grocery",
  "truck_id": 1,
  "truck_number": "TRUCK-1",
  "driver_id": 2,
  "driver_username": "driver1",
  "truck_load_id": 3,
  "total_amount": 750.0,
  "amount_paid": 0.0,
  "payment_status": "pending",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:35:00Z",
  "items": [
    {
      "id": 3,
      "product_id": 1,
      "product_name": "Butter 500g",
      "batch_id": 5,
      "batch_number": "BATCH-2025-11-10-001",
      "quantity": 5,
      "unit_price": 80.0,
      "commission_earned": 25.0,
      "line_total": 400.0
    },
    {
      "id": 4,
      "product_id": 2,
      "product_name": "Yogurt Cup",
      "batch_id": 6,
      "batch_number": "BATCH-2025-11-10-002",
      "quantity": 10,
      "unit_price": 35.0,
      "commission_earned": 50.0,
      "line_total": 350.0
    }
  ],
  "summary": {
    "total_items": 15,
    "total_commission": 75.0,
    "balance_due": 750.0
  }
}
```

### Example 3: Fully paid sale
```bash
curl -X POST http://127.0.0.1:3000/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "amount_paid": 600.0,
    "items": [
      {
        "product_id": 2,
        "quantity": 20
      }
    ]
  }'
```

**Response:**
```json
{
  "id": 3,
  "shop_id": 1,
  "shop_name": "Downtown Grocery",
  "truck_id": 1,
  "truck_number": "TRUCK-1",
  "driver_id": 2,
  "driver_username": "driver1",
  "truck_load_id": 3,
  "total_amount": 600.0,
  "amount_paid": 600.0,
  "payment_status": "paid",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:40:00Z",
  "items": [
    {
      "id": 5,
      "product_id": 2,
      "product_name": "Yogurt Cup",
      "batch_id": 6,
      "batch_number": "BATCH-2025-11-10-002",
      "quantity": 20,
      "unit_price": 30.0,
      "commission_earned": 100.0,
      "line_total": 600.0
    }
  ],
  "summary": {
    "total_items": 20,
    "total_commission": 100.0,
    "balance_due": 0.0
  }
}
```

---

## 3. Get Sale Details (Anyone)

```bash
curl http://127.0.0.1:3000/sales/1
```

**Response:**
```json
{
  "id": 1,
  "shop_id": 1,
  "shop_name": "Downtown Grocery",
  "truck_id": 1,
  "truck_number": "TRUCK-1",
  "driver_id": 2,
  "driver_username": "driver1",
  "truck_load_id": 3,
  "total_amount": 1350.0,
  "amount_paid": 500.0,
  "payment_status": "pending",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:30:00Z",
  "items": [
    {
      "id": 1,
      "product_id": 1,
      "product_name": "Butter 500g",
      "batch_id": 5,
      "batch_number": "BATCH-2025-11-10-001",
      "quantity": 10,
      "unit_price": 75.0,
      "commission_earned": 50.0,
      "line_total": 750.0
    },
    {
      "id": 2,
      "product_id": 2,
      "product_name": "Yogurt Cup",
      "batch_id": 6,
      "batch_number": "BATCH-2025-11-10-002",
      "quantity": 15,
      "unit_price": 30.0,
      "commission_earned": 75.0,
      "line_total": 450.0
    }
  ],
  "summary": {
    "total_items": 25,
    "total_commission": 125.0,
    "balance_due": 850.0
  }
}
```

---

## 4. List All Sales (Anyone)

### List all sales
```bash
curl http://127.0.0.1:3000/sales
```

**Response:**
```json
[
  {
    "id": 3,
    "shop_name": "Downtown Grocery",
    "truck_number": "TRUCK-1",
    "driver_username": "driver1",
    "total_amount": 600.0,
    "amount_paid": 600.0,
    "payment_status": "paid",
    "sale_date": "2025-11-11",
    "total_items": 20
  },
  {
    "id": 2,
    "shop_name": "Downtown Grocery",
    "truck_number": "TRUCK-1",
    "driver_username": "driver1",
    "total_amount": 750.0,
    "amount_paid": 0.0,
    "payment_status": "pending",
    "sale_date": "2025-11-11",
    "total_items": 15
  },
  {
    "id": 1,
    "shop_name": "Downtown Grocery",
    "truck_number": "TRUCK-1",
    "driver_username": "driver1",
    "total_amount": 1350.0,
    "amount_paid": 500.0,
    "payment_status": "pending",
    "sale_date": "2025-11-11",
    "total_items": 25
  }
]
```

### Filter by driver
```bash
curl "http://127.0.0.1:3000/sales?driver_id=2"
```

### Filter by shop
```bash
curl "http://127.0.0.1:3000/sales?shop_id=1"
```

### Filter by date
```bash
curl "http://127.0.0.1:3000/sales?sale_date=2025-11-11"
```

### Filter by payment status
```bash
curl "http://127.0.0.1:3000/sales?payment_status=pending"
```

### Multiple filters
```bash
curl "http://127.0.0.1:3000/sales?driver_id=2&payment_status=pending&sale_date=2025-11-11"
```

---

## 5. Update Payment (Driver Only)

### Add partial payment
```bash
curl -X PATCH http://127.0.0.1:3000/sales/1/payment \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "additional_payment": 400.0
  }'
```

**Response:**
```json
{
  "id": 1,
  "shop_id": 1,
  "shop_name": "Downtown Grocery",
  "truck_id": 1,
  "truck_number": "TRUCK-1",
  "driver_id": 2,
  "driver_username": "driver1",
  "truck_load_id": 3,
  "total_amount": 1350.0,
  "amount_paid": 900.0,
  "payment_status": "pending",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:30:00Z",
  "items": [...],
  "summary": {
    "total_items": 25,
    "total_commission": 125.0,
    "balance_due": 450.0
  }
}
```

### Add final payment (changes status to paid)
```bash
curl -X PATCH http://127.0.0.1:3000/sales/1/payment \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "additional_payment": 450.0
  }'
```

**Response:**
```json
{
  "id": 1,
  "shop_id": 1,
  "shop_name": "Downtown Grocery",
  "truck_id": 1,
  "truck_number": "TRUCK-1",
  "driver_id": 2,
  "driver_username": "driver1",
  "truck_load_id": 3,
  "total_amount": 1350.0,
  "amount_paid": 1350.0,
  "payment_status": "paid",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:30:00Z",
  "items": [...],
  "summary": {
    "total_items": 25,
    "total_commission": 125.0,
    "balance_due": 0.0
  }
}
```

---

## Error Examples

### 1. Insufficient quantity in truck load
```bash
curl -X POST http://127.0.0.1:3000/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "items": [
      {
        "product_id": 1,
        "quantity": 1000
      }
    ]
  }'
```

**Response (400):**
```json
{
  "error": "Insufficient quantity for product 'Butter 500g' in truck load. Need 1000, but not enough available."
}
```

### 2. Driver trying to sell from another driver's truck
```bash
# Login as driver2 first, then try to create sale with truck_load_id=3 (belongs to driver1)
curl -X POST http://127.0.0.1:3000/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER2_TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "items": [...]
  }'
```

**Response (403):**
```json
{
  "error": "You can only create sales for your own truck"
}
```

### 3. Payment exceeds total amount
```bash
curl -X PATCH http://127.0.0.1:3000/sales/2/payment \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "additional_payment": 10000.0
  }'
```

**Response (400):**
```json
{
  "error": "Total payment (10000.00) would exceed sale amount (750.00)"
}
```

### 4. Negative unit price
```bash
curl -X POST http://127.0.0.1:3000/sales \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DRIVER_TOKEN" \
  -d '{
    "shop_id": 1,
    "truck_load_id": 3,
    "sale_date": "2025-11-11",
    "items": [
      {
        "product_id": 1,
        "quantity": 5,
        "unit_price": -10.0
      }
    ]
  }'
```

**Response (400):**
```json
{
  "error": "Unit price cannot be negative"
}
```

---

## Key Points

1. **Commission is always fixed**: Calculated as `quantity × commission_per_unit` from the products table, regardless of the sale price.

2. **FIFO batch selection**: The system automatically selects batches with the earliest expiry date.

3. **Flexible pricing**: 
   - Omit `unit_price` to use the current wholesale price
   - Provide `unit_price` to override with a negotiated price

4. **Payment tracking**:
   - Payment status automatically changes from "pending" to "paid" when fully paid
   - Can make multiple partial payments

5. **Driver restrictions**:
   - Drivers can only create sales from their assigned trucks
   - Drivers can only update payments for their own sales

6. **Automatic updates**:
   - Truck load items `quantity_sold` is automatically updated via database trigger
   - Batch `remaining_quantity` tracking is maintained
