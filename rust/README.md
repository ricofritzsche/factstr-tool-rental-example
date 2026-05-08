# Rust API Bootstrap

This Rust app is the initial HTTP API bootstrap for the FACTSTR tool rental example.

It currently provides:

* explicit environment-based configuration
* startup logging
* PostgreSQL database creation/opening
* FACTSTR PostgreSQL store startup
* `GET /health`

The tool rental domain features are not implemented yet.

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
