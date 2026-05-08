---
name: factstr-usage
description: Use published FACTSTR crates and the Node/TypeScript package correctly for append-only write flows, PostgreSQL bootstrap, and feature-owned projections.
---

# FACTSTR Usage Skill

Use this skill when implementing or changing code that uses FACTSTR in this repository.

FACTSTR is the append-only fact store. Application code should use FACTSTR through its public store APIs, keep facts immutable, and keep feature-owned read models separate from fact storage.

## Source of truth

Do not guess the FACTSTR API from memory.

Before implementing or changing FACTSTR usage, inspect the currently intended release line for this repository.

Use these sources in this order:

1. This repository’s manifests and lock files
   - `Cargo.toml`
   - `Cargo.lock`
   - `package.json`
   - package lock files if present

2. The installed or declared FACTSTR versions
   - Rust crates: `factstr`, `factstr-memory`, `factstr-sqlite`, `factstr-postgres`
   - Node package: `@factstr/factstr-node`

3. FACTSTR documentation for the matching released version
   - README
   - getting started docs
   - stores docs
   - streams docs

4. The FACTSTR GitHub repository only when:
   - the task explicitly uses unreleased FACTSTR changes
   - the task asks to compare against current FACTSTR source
   - the current package docs are insufficient and the version is known

Do not infer API names, crate names, method names, stream behavior, or bootstrap behavior from older examples.

## Version and dependency rules

Use published FACTSTR packages by default.

For Rust reference implementations, prefer exact released versions:

```toml
factstr = "0.4.1"
factstr-postgres = "0.4.1"
````

Use the actual current release line for this repository. Do not hardcode `0.4.1` if the project has moved to a newer released FACTSTR line.

For TypeScript/Node reference implementations, prefer the matching npm package version:

```json
{
  "dependencies": {
    "@factstr/factstr-node": "0.4.1"
  }
}
```

Use git dependencies only when the task explicitly requires unreleased FACTSTR changes.

If git dependencies are required:

* pin to a concrete commit SHA
* do not use a floating branch
* report the pinned SHA
* explain why a released package is not used

Keep Rust and Node/TypeScript examples on the same FACTSTR release line unless the task explicitly requires otherwise.

Do not mix:

* released `factstr` with git `factstr-postgres`
* released Node package with unreleased Rust semantics
* broad version ranges in reference implementations that are meant to prove one exact behavior

## Intended use in this repository

Use FACTSTR as the write-side store for immutable facts.

Use:

* `factstr` for the shared Rust contract
* `factstr-memory` for in-memory development and tests when persistence is not needed
* `factstr-sqlite` for embedded persistence
* `factstr-postgres` for PostgreSQL-backed runtime usage
* `@factstr/factstr-node` for TypeScript/Node usage

Use the concrete store types directly:

* `MemoryStore`
* `SqliteStore`
* `PostgresStore`
* `FactstrMemoryStore`
* `FactstrSqliteStore`
* `FactstrPostgresStore`

Do not wrap FACTSTR behind a CRUD repository abstraction.

Do not model features as mutable CRUD tables first.

Do not hide FACTSTR behind generic service, manager, repository, helper, adapter, or provider layers.

Keep FACTSTR usage visible at the feature or application boundary.

Application code should care about:

* which store is used
* which facts are appended
* which facts are queried
* which projection is updated
* which bootstrap/connect path is required

Application code should not care about:

* FACTSTR table names
* FACTSTR indexes
* FACTSTR durable cursor tables
* FACTSTR append-batch history tables
* internal store schema details

## Rust PostgreSQL store usage

Use `PostgresStore::connect(database_url)` when the target PostgreSQL database already exists.

This path means:

* PostgreSQL server exists
* target database exists
* FACTSTR initializes or validates the schema it owns inside that database
* application code does not create FACTSTR tables or indexes manually

Use `PostgresStore::bootstrap(options)` when a local, demo, test, or reference implementation should start from an existing PostgreSQL server and let FACTSTR create the target database if missing.

The bootstrap options are:

```rust
PostgresBootstrapOptions {
    server_url,
    database_name,
}
```

The bootstrap path means:

* PostgreSQL server exists
* credentials can inspect `pg_database`
* credentials can create the target database
* FACTSTR creates the target database if missing
* FACTSTR derives the target database URL from `server_url + database_name`
* FACTSTR then uses the normal `connect` path
* FACTSTR initializes or validates its schema inside the target database

FACTSTR does not provision, install, run, or host PostgreSQL.

The PostgreSQL server remains external.

Bootstrap database names must use the FACTSTR-supported simple identifier form:

```text
[A-Za-z_][A-Za-z0-9_]*
```

Do not use quoted exotic PostgreSQL database names in the bootstrap path.

Do not manually run FACTSTR DDL from application code.

Do not create FACTSTR tables, indexes, durable stream cursor tables, or append batch tables in the application.

## TypeScript / Node PostgreSQL store usage

Use `@factstr/factstr-node` for TypeScript/Node examples.

Use the existing database path when the target database already exists:

```ts
const store = new FactstrPostgresStore(databaseUrl);
```

Use the explicit bootstrap path when the target database should be created by FACTSTR:

```ts
const store = FactstrPostgresStore.bootstrap({
  serverUrl,
  databaseName,
});
```

The TypeScript bootstrap options are:

```ts
type FactstrPostgresBootstrapOptions = {
  serverUrl: string;
  databaseName: string;
};
```

The Node/TypeScript boundary must mirror the Rust boundary:

* constructor path requires an existing target database
* bootstrap path requires an existing PostgreSQL server
* bootstrap may create the target database
* FACTSTR does not provision or run PostgreSQL
* credentials must be able to inspect `pg_database` and create the target database
* `databaseName` must match `[A-Za-z_][A-Za-z0-9_]*`

Do not reimplement database creation logic in TypeScript.

Do not duplicate FACTSTR schema initialization in TypeScript.

Rust remains the source of truth for store behavior and validation.

## Storage rules

FACTSTR stores facts.

Facts are immutable.

Writes append new facts.

Facts represent things that happened.

Application code must not update or delete facts as part of normal feature behavior.

Use conditional append when a command depends on a context version.

Do not replace FACTSTR with mutable CRUD tables for the write side.

Do not use database tables as the primary feature state when the feature is meant to be fact-driven.

Read APIs should use one of these explicit paths:

1. Direct FACTSTR query
2. Feature-owned in-memory projection
3. Feature-owned persisted projection
4. Durable subscription-backed projection

Keep read models separate from fact storage.

A projection is a query model owned by a feature. It is not the source of truth.

## Query and append rules

Use `EventQuery` and `EventFilter` to select facts.

Keep selection explicit.

Use event type filters when the feature depends on specific fact types.

Use payload predicates only when needed and supported by the current FACTSTR version.

Do not query everything and then perform broad ad-hoc filtering in application code when FACTSTR query filters can express the relevant scope.

Use `append(...)` for unconditional facts.

Use `append_if(...)` when the command depends on the facts relevant to a decision context.

Do not model consistency around aggregates unless the feature explicitly chooses that as a local fact payload concept.

FACTSTR consistency is context-based. IDs may appear in facts, but they do not define the global model.

## Projections and subscriptions

Treat projections as feature-owned query models updated from committed batches.

Use:

```rust
stream_to(&EventQuery, handle)
```

when a feature needs future-only projection updates.

Use:

```rust
stream_to_durable(&DurableStream, &EventQuery, handle)
```

when a feature needs replay/catch-up across restart.

Use:

```rust
stream_all(...)
stream_all_durable(...)
```

only when the feature truly owns all facts in scope.

The stream contract matters:

* notifications happen only after successful persistence
* failed conditional append emits no delivery
* one committed append batch is delivered as one committed batch
* filtered streams deliver only matching committed facts
* delivery follows committed sequence order
* handler failure does not roll back append success
* durable replay starts after the stored cursor
* durable replay/catch-up must avoid duplicates and gaps according to FACTSTR semantics

Keep unrelated facts out of a projection with `EventQuery` and `EventFilter`, not with broad stream subscriptions plus ad-hoc filtering after delivery.

Unsubscribe explicitly through the returned stream handle when a subscription should stop.

## Feature-slice rules

Use FACTSTR at clear feature boundaries.

A command feature may:

* load relevant facts
* build a command context
* decide what facts to append
* call `append(...)` or `append_if(...)`

A query feature may:

* query facts directly
* read a feature-owned projection
* maintain a projection from streams or durable streams

Do not place all FACTSTR access into one generic central store service.

Do not introduce shared domain models around FACTSTR records.

Do not create a generic “event repository”.

Do not create abstractions only to make stores swappable. FACTSTR already provides store implementations behind its own contract.

## Store choice rules

Use `factstr-memory` when:

* the feature needs no persistence
* the test should be fast and isolated
* the code only proves contract-level behavior

Use `factstr-sqlite` when:

* embedded persistence is needed
* no external database should be required
* the example should run locally with a file-backed store

Use `factstr-postgres` when:

* the runtime is PostgreSQL-backed
* the reference implementation should prove PostgreSQL behavior
* durable stream cursor persistence across process restart matters in a PostgreSQL setup

For PostgreSQL tests, prefer unique test databases or isolated schemas according to the repository’s existing test support.

Clean up test databases when the test owns them.

Do not require developers to manually create FACTSTR tables before tests.

## What to verify before implementation

Before implementing FACTSTR usage, verify:

* current FACTSTR release version used by this repository
* crate names and package names
* public store type names
* whether Rust, TypeScript, or both are in scope
* whether PostgreSQL should use `connect` or `bootstrap`
* whether the target database already exists
* whether the PostgreSQL credentials can create databases when bootstrap is used
* whether the feature should read directly, use an in-memory projection, use a persisted projection, or use a durable subscription
* whether the required stream or durable-stream capability exists in the current version
* whether Node/TypeScript has parity with the Rust capability being used

## What to verify after implementation

After changing FACTSTR usage, verify the actual behavior.

For Rust:

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

Use the repository’s exact commands when they differ.

For TypeScript/Node:

```bash
npm run build
npm test
```

Use the package’s actual scripts. Do not invent scripts that are not present.

For PostgreSQL bootstrap, use a live PostgreSQL server when possible.

Verify:

* target database does not exist before bootstrap when the test claims creation
* bootstrap creates the target database
* normal `connect` still works against an existing database
* append works after store initialization
* query works after store initialization
* `append_if` works when the feature uses conditional append
* durable stream setup works when the feature uses durable streams
* invalid bootstrap database names fail clearly

If the live PostgreSQL server is unavailable, state that explicitly. Do not report live PostgreSQL behavior as verified when only compilation ran.

## Documentation expectations

When documenting FACTSTR usage, be precise.

Say:

* FACTSTR initializes the schema it owns
* FACTSTR bootstrap can create the target PostgreSQL database when explicitly requested
* PostgreSQL server provisioning remains external
* credentials need the required PostgreSQL permissions
* facts are append-only
* projections are feature-owned read models

Do not say:

* FACTSTR provisions PostgreSQL
* FACTSTR runs PostgreSQL
* bootstrap is only schema initialization
* projections are the source of truth
* the application must create FACTSTR tables manually

## Response expectations

When reporting completed FACTSTR usage work, include:

* which FACTSTR crates or npm packages were used
* exact versions used
* whether released packages or git dependencies were used
* if git dependencies were used, the pinned commit SHA
* which store implementation was used
* whether PostgreSQL used `connect` or `bootstrap`
* how FACTSTR was initialized
* how facts are appended
* whether reads are direct queries, projections, or durable subscriptions
* what was verified
* which live checks were skipped, if any, and why


