# Rust API Bootstrap

This Rust app is the initial HTTP API bootstrap for the FACTSTR tool rental example.

It currently provides:

* explicit environment-based configuration
* startup logging
* PostgreSQL database creation/opening
* FACTSTR PostgreSQL store startup
* `GET /health`

The Rust implementation currently includes Register Tool, Check Out Tool, Return Tool, and Get Inventory.

## Configuration

Required environment variables:

```text
FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL
FACTSTR_TOOL_RENTAL_DATABASE_NAME
FACTSTR_TOOL_RENTAL_BIND_ADDRESS
```

Suggested values are shown in `.env.example`.

You can copy `.env.example` to `.env` for local development. The app loads `.env` automatically when present.

## Run

```bash
cargo run
```

## Health Check

```bash
curl http://127.0.0.1:3000/health
```

`/health` returns JSON and validates FACTSTR/PostgreSQL store connectivity before returning HTTP 200.

## Endpoints

### Register Tool

`POST /tools`

Returns `201 Created` with `tool_id` and `serial_number` on success.
Returns `409 Conflict` when the serial number is already registered.

```bash
curl -X POST http://127.0.0.1:3000/tools \
  -H 'content-type: application/json' \
  -d '{
    "serial_number": "SN-1001",
    "name": "Rotary Hammer",
    "category": "drilling",
    "manufacturer": "Bosch",
    "model": "GBH 2-26",
    "home_location": "warehouse-a",
    "initial_condition": "ready"
  }'
```

### Check Out Tool

`POST /tools/{tool_id}/checkout`

Returns `201 Created` with `tool_id`, `checked_out_to`, `checked_out_at`, and `due_back_at` on success.
Returns `404 Not Found` when the tool is unknown.
Returns `409 Conflict` when the tool is already checked out.

```bash
curl -X POST http://127.0.0.1:3000/tools/<tool_id>/checkout \
  -H 'content-type: application/json' \
  -d '{
    "checked_out_to": "Team Alpha",
    "checked_out_at": "2026-05-08T09:00:00Z",
    "due_back_at": "2026-05-09T09:00:00Z",
    "use_location": "job-site-7",
    "condition_at_checkout": "ready"
  }'
```

### Return Tool

`POST /tools/{tool_id}/return`

Returns `201 Created` with `tool_id`, `returned_at`, and `returned_to_location` on success.
Returns `404 Not Found` when the tool is unknown.
Returns `409 Conflict` when the tool is not currently checked out.

```bash
curl -X POST http://127.0.0.1:3000/tools/<tool_id>/return \
  -H 'content-type: application/json' \
  -d '{
    "returned_at": "2026-05-10T09:00:00Z",
    "returned_to_location": "warehouse-a",
    "condition_at_return": "ready"
  }'
```

### Get Inventory

`GET /tools`

Returns `200 OK` with the maintained current inventory view.
The view is updated from FACTSTR durable streams.
Empty inventory returns `{ "items": [] }`.
FACTSTR stores facts and durable stream cursors in PostgreSQL, and the Get Inventory slice stores its projection state in PostgreSQL under `projections.inventory_items`.
This keeps the durable cursor and projection state in the same durability boundary across deployment restarts.

```bash
curl http://127.0.0.1:3000/tools
```

## Tests

```bash
cargo test
```

PostgreSQL-backed lifecycle tests:

```bash
FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test
```

`cargo test` also loads `.env` automatically when present.
The configured PostgreSQL user must be allowed to create and drop test databases.
No external `psql` client is required.

## Database Creation

On startup, the app uses the FACTSTR PostgreSQL bootstrap to create the application database if it does not already exist, then opens the store against that database.

Integration tests create their own unique databases and drop them after the test run.

## Shared Facts

The Rust implementation now defines the initial shared application facts under `src/events/`:

* `tool-registered`
* `tool-checked-out`
* `tool-returned`

These definitions describe fact shapes only. Command decisions and query behavior belong in feature slices.

The Register Tool feature now exists in the Rust implementation and records `tool-registered`.
It is available through `POST /tools`, returns `201 Created` with `tool_id` and `serial_number`, and returns `409 Conflict` when the serial number is already registered.
The Check Out Tool feature is available through `POST /tools/{tool_id}/checkout`, returns `201 Created` with `tool_id`, `checked_out_to`, `checked_out_at`, and `due_back_at`, returns `404 Not Found` for unknown tools, and returns `409 Conflict` when a tool is already checked out.
The Return Tool feature is available through `POST /tools/{tool_id}/return`, returns `201 Created` with `tool_id`, `returned_at`, and `returned_to_location`, returns `404 Not Found` for unknown tools, and returns `409 Conflict` when the tool is not currently checked out.
The Get Inventory feature is available through `GET /tools`, returns the maintained current inventory view, is updated from FACTSTR durable streams, stores projection state in `projections.inventory_items`, and returns an empty list when no tools have been registered.
