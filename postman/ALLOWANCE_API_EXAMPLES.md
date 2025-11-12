# Transport Allowance API - Postman Examples

Base URL: `http://127.0.0.1:3000`

**Note:** All allowance endpoints require manager authentication.

---

## Setup: Update Truck Max Limits

### 1. Update Truck Max Allowance Limit

**PATCH** `/trucks/1/max-limit`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

```json
{
  "max_allowance_limit": 4000.00
}
```

### 2. Update Another Truck

**PATCH** `/trucks/2/max-limit`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

```json
{
  "max_allowance_limit": 3500.00
}
```

---

## Transport Allowance Management

### 1. Create Daily Allowance

**POST** `/allowances`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

```json
{
  "allowance_date": "2025-11-12",
  "total_allowance": 10000.00,
  "notes": "Daily transport allowance from CreamyLand"
}
```

**Response:**
```json
{
  "id": 1,
  "allowance_date": "2025-11-12",
  "total_allowance": 10000.0,
  "allocated_amount": 0.0,
  "remaining_amount": 10000.0,
  "status": "pending",
  "notes": "Daily transport allowance from CreamyLand",
  "created_by_username": "manager",
  "truck_allocations": [],
  "created_at": "2025-11-12T10:00:00Z",
  "updated_at": "2025-11-12T10:00:00Z"
}
```

---

### 2. e

**POST** `/allowances/1/allocate`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

```json
{
  "allocations": [
    {
      "truck_id": 1,
      "amount": 3500.00,
      "distance_covered": 45.5,
      "notes": "Route: Downtown - Suburbs"
    },
    {
      "truck_id": 2,
      "amount": 2800.00,
      "distance_covered": 35.2,
      "notes": "Route: City Center"
    },
    {
      "truck_id": 3,
      "amount": 3000.00,
      "distance_covered": 40.0,
      "notes": "Route: Industrial Area"
    }
  ]
}
```

**Response:**
```json
{
  "id": 1,
  "allowance_date": "2025-11-12",
  "total_allowance": 10000.0,
  "allocated_amount": 9300.0,
  "remaining_amount": 700.0,
  "status": "allocated",
  "notes": "Daily transport allowance from CreamyLand",
  "created_by_username": "manager",
  "truck_allocations": [
    {
      "id": 1,
      "truck_id": 1,
      "truck_number": "TRUCK-1",
      "driver_username": "driver1",
      "max_limit": 4000.0,
      "amount": 3500.0,
      "distance_covered": 45.5,
      "notes": "Route: Downtown - Suburbs",
      "created_at": "2025-11-12T10:05:00Z"
    },
    {
      "id": 2,
      "truck_id": 2,
      "truck_number": "TRUCK-2",
      "driver_username": "Ravi",
      "max_limit": 3500.0,
      "amount": 2800.0,
      "distance_covered": 35.2,
      "notes": "Route: City Center",
      "created_at": "2025-11-12T10:05:00Z"
    },
    {
      "id": 3,
      "truck_id": 3,
      "truck_number": "TRUCK-3",
      "driver_username": "Suresh",
      "max_limit": 4000.0,
      "amount": 3000.0,
      "distance_covered": 40.0,
      "notes": "Route: Industrial Area",
      "created_at": "2025-11-12T10:05:00Z"
    }
  ],
  "created_at": "2025-11-12T10:00:00Z",
  "updated_at": "2025-11-12T10:05:00Z"
}
```

---

### 3. Update Single Truck Allocation

**PATCH** `/allowances/1/trucks/1`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

```json
{
  "amount": 3700.00,
  "distance_covered": 48.0,
  "notes": "Route: Downtown - Suburbs (Extended)"
}
```

**Response:** Returns updated full allowance response with all allocations.

---

### 4. Get Allowance Details

**GET** `/allowances/1`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

No body needed. Returns full allowance with all truck allocations.

---

### 5. List All Allowances

**GET** `/allowances`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

No body needed.

**Response:**
```json
[
  {
    "id": 1,
    "allowance_date": "2025-11-12",
    "total_allowance": 10000.0,
    "allocated_amount": 9300.0,
    "remaining_amount": 700.0,
    "status": "allocated",
    "truck_count": 3,
    "created_by_username": "manager"
  }
]
```

---

### 6. Filter Allowances

#### By Status
**GET** `/allowances?status=pending`

#### By Date Range
**GET** `/allowances?start_date=2025-11-01&end_date=2025-11-30`

#### Multiple Filters
**GET** `/allowances?status=allocated&start_date=2025-11-12`

---

### 7. Finalize Allowance (Lock Distribution)

**POST** `/allowances/1/finalize`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

No body needed. Changes status to "finalized" - prevents further modifications.

**Response:** Returns full allowance with status="finalized"

---

### 8. Delete Allowance (Only if Pending)

**DELETE** `/allowances/1`
**Headers:** `Authorization: Bearer MANAGER_TOKEN_HERE`

No body needed. Only works if status is "pending".

---

## Error Examples

### 1. Duplicate Allowance Date
```json
{
  "error": "Allowance for this date already exists"
}
```

### 2. Exceeds Total Allowance
```json
{
  "error": "Total allocation (11000) would exceed total allowance (10000). Already allocated: 0, Remaining: 10000"
}
```

### 3. Exceeds Truck Max Limit
```json
{
  "error": "Allocation amount (4500) exceeds truck TRUCK-2's max limit (3500)"
}
```

### 4. Duplicate Truck Allocation
```json
{
  "error": "Truck TRUCK-1 already has an allocation for this date"
}
```

### 5. Cannot Modify Finalized
```json
{
  "error": "Cannot allocate to finalized allowance"
}
```

### 6. Inactive Truck
```json
{
  "error": "Truck TRUCK-1 is not active"
}
```

---

## Complete Workflow Example

### Day 1: November 12, 2025

**Step 1:** Manager sets max limits (one-time setup)
```bash
PATCH /trucks/1/max-limit { "max_allowance_limit": 4000.00 }
PATCH /trucks/2/max-limit { "max_allowance_limit": 3500.00 }
PATCH /trucks/3/max-limit { "max_allowance_limit": 4000.00 }
```

**Step 2:** Create today's allowance pool
```bash
POST /allowances
{
  "allowance_date": "2025-11-12",
  "total_allowance": 10000.00,
  "notes": "Daily allowance from CreamyLand"
}
```

**Step 3:** Allocate to trucks
```bash
POST /allowances/1/allocate
{
  "allocations": [
    { "truck_id": 1, "amount": 3500.00, "distance_covered": 45.5 },
    { "truck_id": 2, "amount": 2800.00, "distance_covered": 35.2 },
    { "truck_id": 3, "amount": 3000.00, "distance_covered": 40.0 }
  ]
}
```

**Step 4:** Adjust if needed
```bash
PATCH /allowances/1/trucks/1
{
  "amount": 3700.00,
  "distance_covered": 48.0
}
```

**Step 5:** Finalize distribution (lock it)
```bash
POST /allowances/1/finalize
```

---

## Summary

- **Total Allowance:** Rs. 10,000
- **Allocated:** Rs. 9,300
- **Remaining:** Rs. 700
- **Trucks:** 3
- **Status:** Finalized ✓

---

## Database Reference

### Current Setup:
- **Trucks:**
  - TRUCK-1 (driver: driver1) - Max limit: ₹4,000
  - TRUCK-2 (driver: Ravi) - Max limit: ₹3,500
  - TRUCK-3 (driver: Suresh) - Max limit: ₹4,000

### Validation Rules:
- ✓ One allowance per date
- ✓ Sum of allocations ≤ total allowance
- ✓ Each allocation ≤ truck's max limit
- ✓ Cannot allocate same truck twice
- ✓ Cannot modify finalized allowance
- ✓ Trigger auto-updates allocated_amount

---

## Testing Checklist

- [ ] Create allowance for today
- [ ] Allocate to multiple trucks
- [ ] Try to exceed max limit (should fail)
- [ ] Try to exceed total allowance (should fail)
- [ ] Update single allocation
- [ ] View allowance details
- [ ] List all allowances
- [ ] Filter by status
- [ ] Finalize allowance
- [ ] Try to modify finalized (should fail)
- [ ] Delete pending allowance
