# DairyX Backend

Axum + SQLx backend for DairyX Distributors. Provides REST APIs (currently Products CRUD) under the base path `/DairyX`.

## Prerequisites

- Rust (stable)
- PostgreSQL 13+
- sqlx-cli (optional but recommended)

## Quick start

1) Configure environment

Create a `.env` file in the project root:

```
# Example
DATABASE_URL=postgres://postgres:postgres@localhost:5432/dairyx
RUST_LOG=info
```

2) Run database migrations

If you have sqlx-cli installed:

```
sqlx migrate run
```

Alternatively, run from the app on first start (the app expects the schema to exist; prefer using sqlx-cli).

3) Start the server

```
cargo run
```

You should see:

```
Server running on 127.0.0.1:3000
```

Base path: `http://127.0.0.1:3000/DairyX`

## API: Products

Resource is mounted under `/DairyX/products`.

- List products
  - GET `/DairyX/products`
  - 200 OK: `[{ id, name, current_wholesale_price, commission_per_unit, created_at }]`

- Get product by id
  - GET `/DairyX/products/{id}`
  - 200 OK: `{ id, name, current_wholesale_price, commission_per_unit, created_at }`
  - 404 Not Found if missing

- Create product
  - POST `/DairyX/products`
  - Body:
    ```json
    {
      "name": "Milk 1L Packet",
      "current_wholesale_price": 220.0,
      "commission_per_unit": 10.0
    }
    ```
  - 201/200 OK: returns created product
  - 400 Bad Request: if name already exists

- Update product
  - PUT `/DairyX/products/{id}`
  - Body (partial allowed):
    ```json
    {
      "name": "Milk 1L Packet - New",
      "current_wholesale_price": 225.0,
      "commission_per_unit": 11.0
    }
    ```
  - 200 OK: returns updated product
  - 404 Not Found: invalid id
  - 400 Bad Request: duplicate name

- Delete product
  - DELETE `/DairyX/products/{id}`
  - 200 OK on success
  - 404 Not Found if missing

### Example curl

```
# List
curl -s http://127.0.0.1:3000/DairyX/products | jq

# Create
curl -s -X POST http://127.0.0.1:3000/DairyX/products \
  -H 'Content-Type: application/json' \
  -d '{"name":"Milk 1L Packet","current_wholesale_price":220.0,"commission_per_unit":10.0}' | jq

# Get
curl -s http://127.0.0.1:3000/DairyX/products/1 | jq

# Update
curl -s -X PUT http://127.0.0.1:3000/DairyX/products/1 \
  -H 'Content-Type: application/json' \
  -d '{"current_wholesale_price":225.0}' | jq

# Delete
curl -s -X DELETE http://127.0.0.1:3000/DairyX/products/1 | jq
```

## Database & migrations

- The schema is defined in `migrations/` with SQL migrations.
- Important tables:
  - `products` (NUMERIC(10,2) for prices/commissions)
- The app uses SQLx with Postgres and expects `DATABASE_URL`.
- Note on numeric types: the app casts NUMERIC to FLOAT8 in queries to map to Rust `f64`.
  - For strict decimal handling, prefer `rust_decimal` and enable SQLx `decimal` feature.

## Project structure

```
src/
  database/         # DB pool creation and DB-related helpers
  dtos/             # Request/response DTOs (serde)
  error.rs          # App error type and IntoResponse mapping
  handlers/         # Business logic for endpoints (e.g., product CRUD)
  middleware/       # (placeholder) middlewares like auth, cors
  models/           # DB row models (sqlx::FromRow)
  routes/           # Route definitions and router composition
  state/            # AppState (shared state like PgPool)
```

- `src/main.rs`
  - Application bootstrap, loads `.env`, builds router under `/DairyX`, starts axum server on 127.0.0.1:3000.
- `src/routes/`
  - `mod.rs`: creates and composes routers
  - `products.rs`: mounts product routes
- `src/handlers/`
  - `product.rs`: product handler functions (list/get/create/update/delete)
- `src/models/`
  - `product.rs`: Product model used by SQLx queries (`FromRow`)
- `src/dtos/`
  - `product.rs`: DTOs for create/update and response mapping
- `src/database/`
  - `mod.rs`: `create_pool` function (SQLx `PgPool`)
- `src/state/`
  - `app_state.rs`: AppState containing `PgPool`
- `migrations/`
  - SQL files defining the database schema and views

## Configuration

- `.env` example:
  - `DATABASE_URL=postgres://user:pass@localhost:5432/dairyx`
  - `RUST_LOG=info,sqlx=debug` (optional for SQL logs)

## Development tips

- Re-run migrations during schema changes:
  - `sqlx migrate run`
- Watch mode for faster dev loop:
  - `cargo watch -x run` (install with `cargo install cargo-watch`)
- Enable SQL logs for debugging:
  - `RUST_LOG=info,sqlx=debug cargo run`

## Roadmap

- Auth with JWT (jsonwebtoken already added)
- More resources: deliveries, batches, sales
- OpenAPI spec and docs
- Tests (unit + integration)
