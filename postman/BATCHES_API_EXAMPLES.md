# Batches API Examples - DairyX Backend

## Overview
Batches in DairyX are created automatically when deliveries are received from CreamyLand. Each batch represents a specific quantity of a product with a unique batch number and expiry date. Batches are tracked through the entire system lifecycle: delivery → truck loading → sales → reconciliation.

### Key Features:
- ✅ **Auto-created** during delivery creation
- ✅ **FIFO selection** (First-In-First-Out by expiry date)
- ✅ **Stock movement tracking** via `stock_movements` table
- ✅ **Expiry date management**
- ✅ **Remaining quantity tracking**
- ✅ **Unique batch numbers** per product

---

## 📊 Batch Lifecycle

```
1. Delivery Created
   ↓
2. Batches Created (with batch_number, expiry_date, quantity)
   ↓ [stock_movement: delivery_in logged automatically]
   ↓
3. Truck Load Created (FIFO batch selection)
   ↓ [stock_movement: truck_load_out will be logged]
   ↓
4. Sales Made (from truck inventory)
   ↓ [stock_movement: sale_out will be logged]
   ↓
5. Truck Returns (end of day)
   ↓ [stock_movement: truck_return_in logged during reconciliation]
   ↓
6. Daily Reconciliation (stock returned to batches)
   ↓
7. Next Day Cycle Begins
```

---

## 🔍 Viewing Batches

### 1. View All Batches (Via Database Query)
Since batches don't have a dedicated API endpoint, you can query them directly through SQL or view them through related endpoints.

**Direct SQL Query**:
```sql
-- View all batches with product details
SELECT 
    b.id,
    b.batch_number,
    p.id as product_id,
    p.name as product_name,
    b.quantity as initial_quantity,
    b.remaining_quantity,
    b.expiry_date,
    b.created_at
FROM batches b
JOIN products p ON b.product_id = p.id
ORDER BY b.expiry_date ASC, b.created_at ASC;
```

**Response Example**:
```
 id | batch_number | product_id | product_name   | initial_quantity | remaining_quantity | expiry_date | created_at
----|--------------|------------|----------------|------------------|--------------------|--------------|-----------
 1  | BATCH001     | 1          | Milk 1L Packet | 100              | 75                 | 2025-11-20  | 2025-11-12
 2  | BATCH002     | 1          | Milk 1L Packet | 50               | 50                 | 2025-11-22  | 2025-11-12
 3  | BATCH003     | 2          | Yogurt Cup     | 200              | 150                | 2025-11-18  | 2025-11-12
```

---

### 2. View Available Batches (Ready for Loading)

**SQL Query**:
```sql
-- View only batches with remaining stock (available for truck loading)
SELECT 
    b.id,
    b.batch_number,
    p.name as product_name,
    b.remaining_quantity,
    b.expiry_date,
    CASE 
        WHEN b.expiry_date < CURRENT_DATE THEN 'EXPIRED'
        WHEN b.expiry_date <= CURRENT_DATE + INTERVAL '3 days' THEN 'EXPIRING SOON'
        ELSE 'GOOD'
    END as status
FROM batches b
JOIN products p ON b.product_id = p.id
WHERE b.remaining_quantity > 0
ORDER BY b.expiry_date ASC, b.created_at ASC;
```

---

### 3. View Batch Stock Balance (With Integrity Check)

**SQL Query - Using View**:
```sql
-- View batch balance with stock movement verification
SELECT * FROM batch_stock_balance
ORDER BY batch_id;
```

**Response Fields**:
- `batch_id` - Batch ID
- `product_id` - Product ID
- `product_name` - Product name
- `initial_quantity` - Starting quantity
- `remaining_quantity` - Current remaining quantity
- `calculated_balance` - Calculated from stock_movements
- `balance_matches` - TRUE if remaining_quantity matches calculated balance

---

### 4. View Batch via Delivery Endpoint

**Request**: `GET /deliveries/{delivery_id}`

**Response** (includes batches):
```json
{
  "id": 1,
  "delivery_date": "2025-11-12",
  "received_by": 1,
  "delivery_note_number": "DN-20251112-001",
  "items": [
    {
      "id": 1,
      "product_id": 1,
      "quantity": 150,
      "unit_price": 100.50,
      "batches": [
        {
          "id": 1,
          "batch_number": "BATCH001",
          "quantity": 100,
          "remaining_quantity": 75,
          "expiry_date": "2025-11-20"
        },
        {
          "id": 2,
          "batch_number": "BATCH002",
          "quantity": 50,
          "remaining_quantity": 50,
          "expiry_date": "2025-11-22"
        }
      ]
    }
  ]
}
```

---

## 🆕 Creating Batches

Batches are **automatically created** when you create a delivery. You cannot create batches independently.

### Create Delivery (Which Creates Batches)

**Request**: `POST /deliveries`

```json
{
  "delivery_date": "2025-11-12",
  "received_by": 1,
  "delivery_note_number": "DN-20251112-001",
  "items": [
    {
      "product_id": 1,
      "quantity": 150,
      "unit_price": 100.50,
      "batches": [
        {
          "batch_number": "BATCH001",
          "quantity": 100,
          "expiry_date": "2025-11-20"
        },
        {
          "batch_number": "BATCH002",
          "quantity": 50,
          "expiry_date": "2025-11-22"
        }
      ]
    },
    {
      "product_id": 2,
      "quantity": 200,
      "unit_price": 25.00,
      "batches": [
        {
          "batch_number": "YOGURT-2025-11-001",
          "quantity": 200,
          "expiry_date": "2025-11-18"
        }
      ]
    }
  ]
}
```

**What Happens**:
1. ✅ Delivery created
2. ✅ Batches created with:
   - `product_id` from item
   - `batch_number` (must be unique per product)
   - `quantity` and `remaining_quantity` (initially equal)
   - `expiry_date`
3. ✅ Stock movement logged automatically:
   - `movement_type`: `delivery_in`
   - `reference_type`: `delivery`
   - `reference_id`: delivery_id

**Response**: Same as GET delivery (includes batch details)

---

## 📦 Batch Usage Examples

### 1. Loading Batches onto Truck

When creating a truck load, the system automatically selects batches using **FIFO** (First-In-First-Out by expiry date).

**Request**: `POST /truck-loads`

```json
{
  "truck_id": 1,
  "driver_id": 2,
  "load_date": "2025-11-12",
  "items": [
    {
      "product_id": 1,
      "quantity": 50
    }
  ]
}
```

**What Happens**:
1. System finds batches with `product_id = 1` and `remaining_quantity > 0`
2. Orders by `expiry_date ASC` (FIFO - oldest first)
3. Deducts 50 units from oldest batch(es)
4. Updates `batch.remaining_quantity`
5. Creates `truck_load_items` records
6. *(Phase 2)* Logs `stock_movement` with type `truck_load_out`

**Example FIFO Selection**:
```
Available batches:
- BATCH001: 75 units, expiry: 2025-11-20 ← Selected first
- BATCH002: 50 units, expiry: 2025-11-22

Request: 50 units
Result: Takes 50 from BATCH001
BATCH001 remaining: 25 units
```

---

### 2. Selling from Truck (Uses Truck's Batch Inventory)

When creating a sale, batches are selected from the truck's loaded inventory.

**Request**: `POST /sales`

```json
{
  "truck_id": 1,
  "driver_id": 2,
  "shop_id": 1,
  "sale_date": "2025-11-12",
  "items": [
    {
      "product_id": 1,
      "quantity": 20,
      "unit_price": 120.00
    }
  ]
}
```

**What Happens**:
1. System finds batches loaded on this truck (`truck_load_items`)
2. Deducts 20 units from truck inventory
3. Updates `truck_load_items.quantity_sold`
4. Updates `batch.remaining_quantity`
5. *(Phase 2)* Logs `stock_movement` with type `sale_out`

---

### 3. Viewing Batch Stock Movements (Audit Trail)

**SQL Query**:
```sql
-- View all movements for a specific batch
SELECT 
    sm.id,
    sm.movement_type,
    sm.quantity,
    sm.reference_type,
    sm.reference_id,
    sm.movement_date,
    sm.notes,
    u.username as created_by
FROM stock_movements sm
LEFT JOIN users u ON sm.created_by = u.id
WHERE sm.batch_id = 1
ORDER BY sm.created_at ASC;
```

**Response Example**:
```
 id | movement_type   | quantity | reference_type | reference_id | movement_date | notes                           | created_by
----|-----------------|----------|----------------|--------------|---------------|---------------------------------|-----------
 1  | delivery_in     | 100.00   | delivery       | 1            | 2025-11-12    | Initial delivery - BATCH001     | NULL
 2  | truck_load_out  | 50.00    | truck_load     | 1            | 2025-11-12    | Loaded to Truck-01              | manager
 3  | sale_out        | 20.00    | sale           | 5            | 2025-11-12    | Sale to Shop ABC                | driver1
```

---

## 🔄 Batch Restock Scenarios

### Scenario 1: Same Batch Number, Same Expiry (Top-up)

**First Delivery**:
```json
{
  "batch_number": "BATCH001",
  "quantity": 100,
  "expiry_date": "2025-11-20"
}
```

**Second Delivery** (Same batch, same expiry):
```json
{
  "batch_number": "BATCH001",
  "quantity": 50,
  "expiry_date": "2025-11-20"
}
```

**Result**: 
- ✅ Batch quantity updated: 100 + 50 = 150
- ✅ Remaining quantity updated: existing_remaining + 50
- ✅ New stock movement logged

---

### Scenario 2: Same Batch Number, Different Expiry (Error)

**First Delivery**:
```json
{
  "batch_number": "BATCH001",
  "quantity": 100,
  "expiry_date": "2025-11-20"
}
```

**Second Delivery** (Same batch, different expiry):
```json
{
  "batch_number": "BATCH001",
  "quantity": 50,
  "expiry_date": "2025-11-25"  ← Different!
}
```

**Result**: 
- ❌ **ERROR**: "Batch BATCH001 already exists with different expiry date"
- Use a different batch number for different expiry dates

---

## 📊 Useful Batch Queries

### Query 1: Expiring Soon Alert
```sql
-- Find batches expiring in next 3 days
SELECT 
    b.batch_number,
    p.name as product_name,
    b.remaining_quantity,
    b.expiry_date,
    b.expiry_date - CURRENT_DATE as days_until_expiry
FROM batches b
JOIN products p ON b.product_id = p.id
WHERE b.remaining_quantity > 0
  AND b.expiry_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '3 days'
ORDER BY b.expiry_date ASC;
```

---

### Query 2: Expired Batches
```sql
-- Find expired batches with remaining stock
SELECT 
    b.batch_number,
    p.name as product_name,
    b.remaining_quantity,
    b.expiry_date,
    CURRENT_DATE - b.expiry_date as days_expired
FROM batches b
JOIN products p ON b.product_id = p.id
WHERE b.remaining_quantity > 0
  AND b.expiry_date < CURRENT_DATE
ORDER BY b.expiry_date ASC;
```

---

### Query 3: Batch Usage Summary
```sql
-- See how much of each batch has been used
SELECT 
    b.id,
    b.batch_number,
    p.name as product_name,
    b.quantity as initial_quantity,
    b.remaining_quantity,
    b.quantity - b.remaining_quantity as quantity_used,
    ROUND((b.quantity - b.remaining_quantity)::NUMERIC / b.quantity * 100, 2) as usage_percentage,
    b.expiry_date
FROM batches b
JOIN products p ON b.product_id = p.id
WHERE b.quantity > 0
ORDER BY usage_percentage DESC;
```

---

### Query 4: Stock Movement History for Batch
```sql
-- Complete audit trail for a batch
SELECT 
    sm.id,
    sm.movement_type,
    sm.quantity,
    sm.reference_type,
    sm.reference_id,
    sm.movement_date,
    sm.notes,
    COALESCE(u.username, 'SYSTEM') as created_by,
    -- Running balance
    SUM(
        CASE 
            WHEN sm.movement_type IN ('delivery_in', 'truck_return_in', 'adjustment') 
            THEN sm.quantity 
            ELSE -sm.quantity 
        END
    ) OVER (ORDER BY sm.created_at) as running_balance
FROM stock_movements sm
LEFT JOIN users u ON sm.created_by = u.id
WHERE sm.batch_id = 1
ORDER BY sm.created_at ASC;
```

---

## 🎯 Best Practices

### 1. Batch Naming Convention
```
Format: {PRODUCT_CODE}-{YYYY-MM-DD}-{SEQUENCE}
Examples:
- MILK-2025-11-12-001
- YOGURT-2025-11-12-001
- CURD-2025-11-12-001
```

### 2. Expiry Date Management
- ✅ Set realistic expiry dates (check product shelf life)
- ✅ FIFO automatically handles oldest-first
- ✅ Monitor expiring batches daily
- ✅ Use adjustment movements for expired stock removal

### 3. Batch Quantity Validation
- ✅ Sum of batch quantities must equal delivery item quantity
- ✅ Each batch must have quantity > 0
- ✅ Remaining quantity updated automatically via triggers

### 4. Stock Movement Tracking
- ✅ Delivery creates `delivery_in` movement (automatic)
- ✅ Truck load creates `truck_load_out` movement (Phase 2)
- ✅ Sale creates `sale_out` movement (Phase 2)
- ✅ Reconciliation creates `truck_return_in` movement (Phase 2)
- ✅ Manual adjustments logged with notes

---

## 🔐 Permissions

### Batch Creation (via Delivery)
- **Who**: Manager only
- **Endpoint**: `POST /deliveries`
- **Auth**: Bearer token with `role: manager`

### Batch Viewing
- **Who**: Manager and Driver (via their respective endpoints)
- **Endpoints**: 
  - Deliveries: Both can view
  - Truck loads: Driver can view their own
  - Sales: Driver can view their own

### Batch Queries
- **Who**: Database access required
- **Method**: Direct SQL queries or future API endpoints

---

## 🚀 Future Enhancements (Phase 2)

### Planned Features:
1. ✅ **Direct Batch Query Endpoint**: `GET /batches?product_id=1&available=true`
2. ✅ **Batch Stock Movement Endpoint**: `GET /batches/{id}/movements`
3. ✅ **Expiring Batches Alert**: `GET /batches/expiring?days=3`
4. ✅ **Expired Batches Report**: `GET /batches/expired`
5. ✅ **Manual Stock Adjustment**: `POST /stock-movements/adjust` (for damaged/expired items)

---

## 📝 Example Workflow

### Complete Batch Lifecycle Example:

#### Step 1: Receive Delivery
```bash
POST /deliveries
{
  "delivery_date": "2025-11-12",
  "received_by": 1,
  "items": [{
    "product_id": 1,
    "quantity": 100,
    "batches": [{
      "batch_number": "MILK-2025-11-12-001",
      "quantity": 100,
      "expiry_date": "2025-11-20"
    }]
  }]
}
```
**Result**: Batch created with ID 1, 100 units, stock_movement logged

---

#### Step 2: Load Truck
```bash
POST /truck-loads
{
  "truck_id": 1,
  "items": [{
    "product_id": 1,
    "quantity": 50
  }]
}
```
**Result**: 50 units from BATCH001 loaded, remaining: 50 units

---

#### Step 3: Make Sales
```bash
POST /sales
{
  "truck_id": 1,
  "shop_id": 1,
  "items": [{
    "product_id": 1,
    "quantity": 30
  }]
}
```
**Result**: 30 units sold from truck, batch remaining: 50, truck remaining: 20

---

#### Step 4: Check Batch Status
```sql
SELECT * FROM batch_stock_balance WHERE batch_id = 1;
```
**Result**:
```
batch_id: 1
initial_quantity: 100
remaining_quantity: 50
calculated_balance: 50
balance_matches: true ✅
```

---

## ❓ Common Questions

### Q1: Can I create a batch without a delivery?
**A**: No, batches are always created through deliveries. This ensures proper audit trail.

### Q2: What happens if I try to load more than available?
**A**: Error: "Insufficient batch quantity. Available: X, Requested: Y"

### Q3: Can I delete a batch?
**A**: No direct delete. Delete the delivery to remove its batches (only if no sales made).

### Q4: How do I handle expired batches?
**A**: Use manual stock adjustment endpoint (Phase 2) with movement_type: `expired_out`

### Q5: Why is balance_matches false for some batches?
**A**: Batches created before stock_movements migration. New batches will always match.

### Q6: Can batches span multiple deliveries?
**A**: Yes! If same batch_number and expiry_date, quantities will be added together.

---

## 🔗 Related Documentation

- [POSTMAN_EXAMPLES.md](./POSTMAN_EXAMPLES.md) - Delivery creation examples
- [SALES_API_EXAMPLES.md](./SALES_API_EXAMPLES.md) - Sale examples using batches
- [OPTION_A_IMPLEMENTATION_SUMMARY.md](./OPTION_A_IMPLEMENTATION_SUMMARY.md) - Stock movement system
- [REGRESSION_TEST_RESULTS.md](./REGRESSION_TEST_RESULTS.md) - Testing results

---

**Document Version**: 1.0  
**Last Updated**: 2025-11-12  
**System**: DairyX Backend v1.0
