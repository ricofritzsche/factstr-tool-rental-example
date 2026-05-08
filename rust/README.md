# Rust API Bootstrap

This Rust app is the initial HTTP API bootstrap for the FACTSTR tool rental example.

It currently provides:

* explicit environment-based configuration
* startup logging
* PostgreSQL database creation/opening
* FACTSTR PostgreSQL store startup
* `GET /health`

The Rust implementation currently includes the Register Tool command feature. Other tool-rental features are not implemented yet.

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
