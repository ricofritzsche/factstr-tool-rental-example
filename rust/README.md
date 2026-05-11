# Rust Implementation

## What This Contains

This is the Rust implementation of the FACTSTR tool rental reference app.

For a longer explanation of the feature-slice approach used here, see:
[How I structure self-contained feature slices](https://medium.com/@rico-fritzsche/how-i-structure-self-contained-feature-slices-a31d17df5628?sk=826aa678f4b8bf45679f6218cf35c8eb)

It currently includes:

- PostgreSQL-backed FACTSTR store
- Register Tool
- Check Out Tool
- Return Tool
- Get Inventory
- PostgreSQL-backed inventory projection under `projections.inventory_items`
- async durable stream projection updates
- built-in browser UI at `GET /`
- SSE inventory invalidation at `GET /tools/events`

This implementation uses `factstr = "0.5.2"` and `factstr-postgres = "0.5.2"`.

## Requirements

- Rust toolchain
- PostgreSQL database/server reachable through the configured admin URL

## Configuration

Environment variables:

- `FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL`
  Required. No default.
- `FACTSTR_TOOL_RENTAL_DATABASE_NAME`
  Optional. Default: `factstr_tool_rental`
- `FACTSTR_TOOL_RENTAL_BIND_ADDRESS`
  Optional. Default: `127.0.0.1:3000`

If a `.env` file is present, it is loaded automatically.

## Run

```bash
cd rust
cargo run
```

Then open:

```text
http://127.0.0.1:3000/
```

## Built-In UI

- `GET /` serves the built-in browser UI.
- No frontend build step is required.
- Static assets live under `rust/static/`.
- The UI uses `GET /tools` as the inventory source.
- The UI listens to `GET /tools/events` for cross-tab refresh.

## API Endpoints

### `GET /health`

Checks that the service is running and the store is reachable.

- Success: `200 OK`

### `GET /tools`

Returns the maintained current inventory view.

- Success: `200 OK`

### `POST /tools`

Registers a new tool.

- Success: `201 Created`
- Request body:

```json
{
  "serial_number": "SN-1001",
  "name": "Rotary Hammer",
  "category": "drilling",
  "manufacturer": "Bosch",
  "model": "GBH 2-26",
  "home_location": "warehouse-a",
  "initial_condition": "ready"
}
```

- Important status mappings:
  - `400 Bad Request` for invalid or blank required fields
  - `409 Conflict` for `serial_number_already_registered`
  - `500 Internal Server Error` for `store_error`

### `GET /tools/events`

Returns the inventory invalidation stream.

- Success: `200 OK`
- Response type: Server-Sent Events
- Emits `inventory-changed`

### `POST /tools/{tool_id}/checkout`

Checks out an available tool.

- Success: `201 Created`
- Request body:

```json
{
  "checked_out_to": "Team Alpha",
  "checked_out_at": "2026-05-08T09:00:00Z",
  "due_back_at": "2026-05-09T09:00:00Z",
  "use_location": "job-site-7",
  "condition_at_checkout": "ready"
}
```

- Important status mappings:
  - `400 Bad Request` for invalid input such as missing fields or invalid due-back ordering
  - `404 Not Found` for `tool_not_registered`
  - `409 Conflict` for `tool_already_checked_out`
  - `500 Internal Server Error` for `store_error`

### `POST /tools/{tool_id}/return`

Returns a checked-out tool.

- Success: `201 Created`
- Request body:

```json
{
  "returned_at": "2026-05-10T09:00:00Z",
  "returned_to_location": "warehouse-a",
  "condition_at_return": "ready"
}
```

- Important status mappings:
  - `400 Bad Request` for invalid input such as missing required fields
  - `404 Not Found` for `tool_not_registered`
  - `409 Conflict` for `tool_not_checked_out`
  - `500 Internal Server Error` for `store_error`

## Projection Persistence

- FACTSTR stores facts and durable stream cursors in PostgreSQL.
- Get Inventory stores its projection state in PostgreSQL under `projections.inventory_items`.
- Projection updates are persisted before the durable cursor advances.
- `GET /tools` reads the maintained projection.
- `GET /tools/events` only emits an invalidation signal.

## Source Layout

```text
src/events/
src/features/
src/http/
src/projection_database.rs
src/routes.rs
src/store.rs
static/
```

- `src/events/`: shared fact definitions
- `src/features/`: feature slices
- `src/http/`: HTTP handlers and HTTP response mapping
- `src/projection_database.rs`: projection database infrastructure
- `src/routes.rs`: route composition
- `src/store.rs`: FACTSTR store boundary
- `static/`: browser UI assets

## Tests

```bash
cd rust
cargo test
```

PostgreSQL-backed tests require `FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL`.

The test suite creates and drops unique test databases for PostgreSQL-backed cases.

## License

See the repository-level [license section](../README.md#license).
