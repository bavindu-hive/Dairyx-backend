# Truck Loads API Examples

Base URL: `http://localhost:3000/DairyX`

## Authentication
Most endpoints require a Bearer token. Get it from `/users/login`:
```bash
curl -X POST http://localhost:3000/DairyX/users/login \
  -H "Content-Type: application/json" \
  -d '{"username": "manager1", "password": "password123"}'
```

---

## 1. Create Truck Load (Load a truck with products)

**Endpoint:** `POST /truck-loads`  
**Auth Required:** ✅ Yes (Bearer Token)  
**Who can access:** Managers only

**Two Ways to Load:**
1. **Manual Mode** - Specify exact batches (full control)
2. **Auto FIFO Mode** - Specify products, system picks batches automatically (easiest!)

### Request Body (Manual Mode - Specify Batches):
```json
{
  "truck_id": 1,
  "load_date": "2025-11-12",
  "loaded_by": 4,
  "notes": "Morning delivery route",
  "items": [
    {
      "batch_id": 4,
      "quantity_loaded": 15
    },
    {
      "batch_id": 7,
      "quantity_loaded": 25
    }
  ]
}
```

### Request Body (Auto FIFO Mode - Specify Products) ⭐ **RECOMMENDED**:
```json
{
  "truck_id": 1,
  "load_date": "2025-11-12",
  "loaded_by": 4,
  "notes": "Morning delivery route",
  "items": [
    {
      "product_id": 2,
      "quantity_loaded": 15
    },
    {
      "product_id": 4,
      "quantity_loaded": 25
    }
  ]
}
```

**How Auto FIFO Works:**
- System automatically selects batches with **nearest expiry dates** first
- Prevents expired stock by using oldest products first
- Can span multiple batches if needed (e.g., request 50, get 30 from batch A + 20 from batch B)
- Much simpler - no need to know which batches have stock!

### Request Body (Mixed Mode):
You can even mix both styles in one request!
```json
{
  "truck_id": 1,
  "load_date": "2025-11-12",
  "loaded_by": 4,
  "items": [
    {
      "product_id": 2,
      "quantity_loaded": 15
    },
    {
      "batch_id": 7,
      "quantity_loaded": 10
    }
  ]
}
```

### cURL Example (Auto FIFO Mode):
```bash
curl -X POST http://localhost:3000/DairyX/truck-loads \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -d '{
    "truck_id": 1,
    "load_date": "2025-11-12",
    "loaded_by": 4,
    "notes": "Morning delivery route",
    "items": [
      {
        "product_id": 2,
        "quantity_loaded": 15
      },
      {
        "product_id": 4,
        "quantity_loaded": 25
      }
    ]
  }'
```

### Success Response (201 Created):
```json
{
  "id": 1,
  "truck_id": 1,
  "truck_number": "TR-001",
  "driver_username": "driver1",
  "load_date": "2025-11-12",
  "loaded_by": 4,
  "loaded_by_username": "manager1",
  "status": "loaded",
  "notes": "Morning delivery route",
  "created_at": "2025-11-12T08:30:00Z",
  "items": [
    {
      "id": 1,
      "batch_id": 4,
      "batch_number": "MILK-241110-B",
      "product_id": 2,
      "product_name": "Milk 1L",
      "expiry_date": "2025-11-17",
      "quantity_loaded": 15,
      "quantity_sold": 0,
      "quantity_returned": 0,
      "quantity_lost_damaged": 0
    },
    {
      "id": 2,
      "batch_id": 7,
      "batch_number": "Cheese-241110-A",
      "product_id": 4,
      "product_name": "Cheese 200g",
      "expiry_date": "2025-11-20",
      "quantity_loaded": 25,
      "quantity_sold": 0,
      "quantity_returned": 0,
      "quantity_lost_damaged": 0
    }
  ],
  "summary": {
    "total_loaded": 40,
    "total_sold": 0,
    "total_returned": 0,
    "total_lost_damaged": 0,
    "product_lines": 2
  }
}
```

### Error Responses:
- **400 Bad Request** - Invalid data:
  - Item missing both `batch_id` and `product_id`
  - Item has both `batch_id` and `product_id` (choose one)
  - Insufficient batch quantity (manual mode)
  - Insufficient stock for product (auto FIFO mode)
  - No available batches for product
- **401 Unauthorized** - Missing or invalid token
- **403 Forbidden** - User is not a manager
- **404 Not Found** - Truck, batch, or product not found

---

## 2. Get Truck Load Details

**Endpoint:** `GET /truck-loads/{id}`  
**Auth Required:** ❌ No (Public endpoint)

### cURL Example:
```bash
curl -X GET http://localhost:3000/DairyX/truck-loads/1
```

### Success Response (200 OK):
```json
{
  "id": 1,
  "truck_id": 1,
  "truck_number": "TR-001",
  "driver_username": "driver1",
  "load_date": "2025-11-12",
  "loaded_by": 4,
  "loaded_by_username": "manager1",
  "status": "loaded",
  "notes": "Morning delivery route",
  "created_at": "2025-11-12T08:30:00Z",
  "items": [
    {
      "id": 1,
      "batch_id": 4,
      "batch_number": "MILK-241110-B",
      "product_id": 2,
      "product_name": "Milk 1L",
      "expiry_date": "2025-11-17",
      "quantity_loaded": 15,
      "quantity_sold": 12,
      "quantity_returned": 2,
      "quantity_lost_damaged": 1
    },
    {
      "id": 2,
      "batch_id": 7,
      "batch_number": "Cheese-241110-A",
      "product_id": 4,
      "product_name": "Cheese 200g",
      "expiry_date": "2025-11-20",
      "quantity_loaded": 25,
      "quantity_sold": 20,
      "quantity_returned": 3,
      "quantity_lost_damaged": 2
    }
  ],
  "summary": {
    "total_loaded": 40,
    "total_sold": 32,
    "total_returned": 5,
    "total_lost_damaged": 3,
    "product_lines": 2
  }
}
```

### Error Responses:
- **404 Not Found** - Truck load does not exist

---

## 3. List All Truck Loads

**Endpoint:** `GET /truck-loads`  
**Auth Required:** ❌ No (Public endpoint)  
**Query Parameters:**
- `truck_id` (optional) - Filter by truck ID
- `status` (optional) - Filter by status: "loaded", "reconciled"
- `start_date` (optional) - Format: YYYY-MM-DD
- `end_date` (optional) - Format: YYYY-MM-DD

### cURL Example (All loads):
```bash
curl -X GET http://localhost:3000/DairyX/truck-loads
```

### cURL Example (With filters):
```bash
# Filter by truck
curl -X GET "http://localhost:3000/DairyX/truck-loads?truck_id=1"

# Filter by status
curl -X GET "http://localhost:3000/DairyX/truck-loads?status=loaded"

# Filter by date range
curl -X GET "http://localhost:3000/DairyX/truck-loads?start_date=2025-11-01&end_date=2025-11-30"

# Combine filters
curl -X GET "http://localhost:3000/DairyX/truck-loads?truck_id=1&status=loaded&start_date=2025-11-12"
```

### Success Response (200 OK):
```json
[
  {
    "id": 1,
    "truck_id": 1,
    "truck_number": "TR-001",
    "driver_username": "driver1",
    "load_date": "2025-11-12",
    "status": "loaded",
    "total_loaded": 40,
    "total_sold": 32,
    "total_returned": 5,
    "total_lost_damaged": 3
  },
  {
    "id": 2,
    "truck_id": 2,
    "truck_number": "TR-002",
    "driver_username": "driver2",
    "load_date": "2025-11-12",
    "status": "reconciled",
    "total_loaded": 60,
    "total_sold": 55,
    "total_returned": 3,
    "total_lost_damaged": 2
  }
]
```

---

## 4. Reconcile Truck Load (Record returns)

**Endpoint:** `PUT /truck-loads/{id}/reconcile`  
**Auth Required:** ✅ Yes (Bearer Token)  
**Who can access:** Managers only

This endpoint records the quantities returned from a truck after the day's sales.

### Request Body:
```json
{
  "returns": [
    {
      "batch_id": 4,
      "quantity_returned": 2
    },
    {
      "batch_id": 7,
      "quantity_returned": 3
    }
  ]
}
```

### cURL Example:
```bash
curl -X PUT http://localhost:3000/DairyX/truck-loads/1/reconcile \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -d '{
    "returns": [
      {
        "batch_id": 4,
        "quantity_returned": 2
      },
      {
        "batch_id": 7,
        "quantity_returned": 3
      }
    ]
  }'
```

### Success Response (200 OK):
```json
{
  "id": 1,
  "truck_id": 1,
  "truck_number": "TR-001",
  "driver_username": "driver1",
  "load_date": "2025-11-12",
  "loaded_by": 4,
  "loaded_by_username": "manager1",
  "status": "reconciled",
  "notes": "Morning delivery route",
  "created_at": "2025-11-12T08:30:00Z",
  "items": [
    {
      "id": 1,
      "batch_id": 4,
      "batch_number": "MILK-241110-B",
      "product_id": 2,
      "product_name": "Milk 1L",
      "expiry_date": "2025-11-17",
      "quantity_loaded": 15,
      "quantity_sold": 12,
      "quantity_returned": 2,
      "quantity_lost_damaged": 1
    },
    {
      "id": 2,
      "batch_id": 7,
      "batch_number": "Cheese-241110-A",
      "product_id": 4,
      "product_name": "Cheese 200g",
      "expiry_date": "2025-11-20",
      "quantity_loaded": 25,
      "quantity_sold": 20,
      "quantity_returned": 3,
      "quantity_lost_damaged": 2
    }
  ],
  "summary": {
    "total_loaded": 40,
    "total_sold": 32,
    "total_returned": 5,
    "total_lost_damaged": 3,
    "product_lines": 2
  }
}
```

### Notes:
- `quantity_sold` is calculated from sales records
- `quantity_lost_damaged` is automatically calculated: `loaded - sold - returned`
- Status changes to "reconciled" after this operation
- Batches are updated with returned quantities

### Error Responses:
- **400 Bad Request** - Invalid return quantities or already reconciled
- **401 Unauthorized** - Missing or invalid token
- **403 Forbidden** - User is not a manager
- **404 Not Found** - Truck load not found

---

## 5. Delete Truck Load

**Endpoint:** `DELETE /truck-loads/{id}`  
**Auth Required:** ✅ Yes (Bearer Token)  
**Who can access:** Managers only

⚠️ **Warning:** Can only delete truck loads with no sales recorded. Once sales are made, deletion is not allowed.

### cURL Example:
```bash
curl -X DELETE http://localhost:3000/DairyX/truck-loads/1 \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

### Success Response (204 No Content):
No response body. HTTP status code 204 indicates successful deletion.

### Error Responses:
- **400 Bad Request** - Truck load has sales and cannot be deleted
- **401 Unauthorized** - Missing or invalid token
- **403 Forbidden** - User is not a manager
- **404 Not Found** - Truck load not found

---

## Common Field Descriptions

### Truck Load Status:
- `loaded` - Truck is loaded and out for delivery
- `reconciled` - Truck has returned and returns have been recorded

### Calculated Fields:
- `quantity_sold` - Total quantity sold (from sales records)
- `quantity_returned` - Quantity returned to warehouse
- `quantity_lost_damaged` - Automatically calculated: `loaded - sold - returned`

### Summary Fields:
- `total_loaded` - Sum of all items loaded
- `total_sold` - Sum of all items sold
- `total_returned` - Sum of all items returned
- `total_lost_damaged` - Sum of all lost/damaged items
- `product_lines` - Number of different products loaded

---

## Typical Workflow

1. **Morning - Load Truck:**
   ```
   POST /truck-loads
   ```
   - Manager loads products onto truck
   - Status: "loaded"
   - Batch quantities are reduced

2. **During Day - Sales:**
   - Driver makes sales (using sales endpoints)
   - Sales are recorded against truck load items

3. **Evening - Reconcile Returns:**
   ```
   PUT /truck-loads/{id}/reconcile
   ```
   - Manager records returned quantities
   - Status changes to "reconciled"
   - Batch quantities are increased with returns
   - Lost/damaged quantities calculated automatically

4. **Reporting:**
   ```
   GET /truck-loads?start_date=2025-11-12&end_date=2025-11-12
   ```
   - View all loads for a specific date
   - Analyze performance and losses

---

## Testing Tips

1. **Create a truck load** first using POST
2. **Get the truck load ID** from the response
3. **View details** using GET /truck-loads/{id}
4. **Make some sales** (optional, using sales endpoints)
5. **Reconcile** using PUT /truck-loads/{id}/reconcile
6. **List all loads** to see the summary

### Example Test Sequence:
```bash
# 1. Login
TOKEN=$(curl -X POST http://localhost:3000/DairyX/users/login \
  -H "Content-Type: application/json" \
  -d '{"username": "manager1", "password": "password123"}' \
  | jq -r '.token')

# 2. Create truck load
LOAD_ID=$(curl -X POST http://localhost:3000/DairyX/truck-loads \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "truck_id": 1,
    "load_date": "2025-11-12",
    "loaded_by": 4,
    "items": [{"batch_id": 4, "quantity_loaded": 15}]
  }' | jq -r '.id')

# 3. View details
curl -X GET http://localhost:3000/DairyX/truck-loads/$LOAD_ID

# 4. Reconcile returns
curl -X PUT http://localhost:3000/DairyX/truck-loads/$LOAD_ID/reconcile \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "returns": [{"batch_id": 4, "quantity_returned": 2}]
  }'

# 5. List all loads
curl -X GET http://localhost:3000/DairyX/truck-loads
```
