# Sales API - Postman JSON Examples

Base URL: `http://127.0.0.1:3000`

---

## 1. Login (Get Token First)

**POST** `/users/login`

```json
{
  "username": "driver1",
  "password": "password"
}
```

**Save the token from response and use it in Authorization header as:**
`Bearer YOUR_TOKEN_HERE`

---

## 2. Create Sale - Default Wholesale Prices

**POST** `/sales`
**Headers:** `Authorization: Bearer YOUR_TOKEN_HERE`

```json
{
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
}
```

---

## 3. Create Sale - Custom Prices (Negotiated)

**POST** `/sales`
**Headers:** `Authorization: Bearer YOUR_TOKEN_HERE`

```json
{
  "shop_id": 1,
  "truck_load_id": 3,
  "sale_date": "2025-11-11",
  "amount_paid": 0,
  "items": [
    {
      "product_id": 3,
      "quantity": 3,
      "unit_price": 750.0
    },
    {
      "product_id": 2,
      "quantity": 8,
      "unit_price": 160.0
    }
  ]
}
```

---

## 4. Create Sale - Fully Paid

**POST** `/sales`
**Headers:** `Authorization: Bearer YOUR_TOKEN_HERE`

```json
{
  "shop_id": 1,
  "truck_load_id": 3,
  "sale_date": "2025-11-11",
  "amount_paid": 1500.0,
  "items": [
    {
      "product_id": 2,
      "quantity": 10
    }
  ]
}
```

---

## 5. Get Sale by ID

**GET** `/sales/1`

No body needed.

---

## 6. List All Sales

**GET** `/sales`

No body needed.

---

## 7. Filter Sales by Driver

**GET** `/sales?driver_id=2`

No body needed.

---

## 8. Filter Sales by Payment Status

**GET** `/sales?payment_status=pending`

No body needed.

---

## 9. Filter Sales - Multiple Filters

**GET** `/sales?driver_id=2&payment_status=pending&sale_date=2025-11-11`

No body needed.

---

## 10. Update Payment (Add Partial Payment)

**PATCH** `1`
**Headers:** `Authorization: Bearer YOUR_TOKEN_HERE`

```json
{
  "additional_payment": 300.0
}
```

---

## 11. Update Payment (Pay Remaining Balance)

**PATCH** `/sales/1/payment`
**Headers:** `Authorization: Bearer YOUR_TOKEN_HERE`

```json
{
  "additional_payment": 850.0
}
```

---

## Database Reference

Your current data:
- **Driver:** driver1 (id: 2, password: "password")
- **Truck:** TRUCK-1 (id: 1, assigned to driver1)
- **Shop:** Downtown Grocery (id: 1)
- **Truck Load:** id: 3 (loaded today with products)
- **Products in Truck Load 3:**
  - Butter 500g (id: 3) - 40 units available, price: ₹700, commission: ₹15
  - Yogurt Cup (id: 2) - 50 units available, price: ₹150, commission: ₹5

---

## Testing Steps in Postman

1. **Login:** POST `/users/login` with driver1 credentials → Copy the token
2. **Create Sale:** POST `/sales` with the token in Authorization header
3. **View Sale:** GET `/sales/{id}` where {id} is from step 2 response
4. **List Sales:** GET `/sales` to see all sales
5. **Add Payment:** PATCH `/sales/{id}/payment` to add more payment
6. **Check Status:** GET `/sales/{id}` to verify payment_status changed to "paid"

---

## Expected Responses

### Create Sale Response:
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
  "total_amount": 5000.0,
  "amount_paid": 500.0,
  "payment_status": "pending",
  "sale_date": "2025-11-11",
  "created_at": "2025-11-11T10:30:00Z",
  "items": [
    {
      "id": 1,
      "product_id": 3,
      "product_name": "Butter 500g",
      "batch_id": 5,
      "batch_number": "BATCH-...",
      "quantity": 5,
      "unit_price": 700.0,
      "commission_earned": 75.0,
      "line_total": 3500.0
    },
    {
      "id": 2,
      "product_id": 2,
      "product_name": "Yogurt Cup",
      "batch_id": 4,
      "batch_number": "BATCH-...",
      "quantity": 10,
      "unit_price": 150.0,
      "commission_earned": 50.0,
      "line_total": 1500.0
    }
  ],
  "summary": {
    "total_items": 15,
    "total_commission": 125.0,
    "balance_due": 4500.0
  }
}
```

### List Sales Response:
```json
[
  {
    "id": 1,
    "shop_name": "Downtown Grocery",
    "truck_number": "TRUCK-1",
    "driver_username": "driver1",
    "total_amount": 5000.0,
    "amount_paid": 500.0,
    "payment_status": "pending",
    "sale_date": "2025-11-11",
    "total_items": 15
  }
]
```
