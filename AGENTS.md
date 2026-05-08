# AGENTS.md

## Purpose

This repository is a reference implementation for building a small tool rental application with FACTSTR.

The goal is to show the same business behavior in multiple language implementations while keeping the architecture easy to inspect, test, and compare.

This repository is not a framework, library, starter kit, or generic clean architecture demo.

The code should make these ideas visible:

- append-only facts
- command-context consistency
- functional core / imperative shell
- self-contained feature slices
- feature-owned query models
- durable streams for replay and projection catch-up where needed
- direct, explicit application structure without generic architecture layers

## Source of truth

Before changing code, inspect the nearest source of truth.

Use this order:

1. this `AGENTS.md`
2. the README for the implementation being changed, for example `rust/README.md`
3. the relevant local skill under `.agents/skills`
4. the actual source and tests
5. the FACTSTR package documentation for the exact version used
6. the FACTSTR GitHub repository only when the task explicitly requires unreleased FACTSTR behavior or current source comparison

Do not guess FACTSTR APIs, crate names, package names, or method names from memory.

Do not rely on older examples when the current repository, package version, or local skill says something different.

## Repository shape

The repository is organized by implementation language.

Expected high-level shape:

```text
.
├── docs/
├── rust/
├── typescript-node/
└── .agents/
````

Each language implementation should demonstrate the same business behavior where practical.

Language-specific code may differ in idiomatic details, but observable business behavior should remain aligned across implementations.

Do not let one implementation quietly become the hidden source of different product behavior.

Do not introduce shared runtime code between language implementations unless a task explicitly asks for it.

## FACTSTR usage

Use FACTSTR as the append-only fact store.

Application code should use FACTSTR through its public APIs.

Use published FACTSTR packages by default.

For reference implementations, pin exact released versions rather than broad floating ranges.

Use git dependencies only when the task explicitly requires unreleased FACTSTR behavior. If a git dependency is required, pin it to a concrete commit SHA and report that SHA.

Do not manually create FACTSTR tables, indexes, durable cursor tables, or append-batch tables in application code.

FACTSTR owns its store mechanics.

Application code may provide:

* store choice
* database/server configuration
* bootstrap/connect choice
* fact payloads
* queries
* projection handlers

Application code must not own:

* FACTSTR schema DDL
* FACTSTR sequence allocation
* FACTSTR durable cursor storage
* FACTSTR append-batch history
* FACTSTR internal store tables or indexes

Keep FACTSTR usage visible at the application or feature boundary.

Do not hide FACTSTR behind CRUD repositories, generic services, managers, helpers, or adapters.

## PostgreSQL bootstrap rules

Use the normal PostgreSQL connect path when the target database already exists.

Use FACTSTR PostgreSQL bootstrap when a local, demo, test, or reference implementation should start from an existing PostgreSQL server and let FACTSTR create the target database if missing.

The boundary is:

```text
connect(database_url)
= target database already exists
= FACTSTR initializes or validates its own schema inside that database
```

```text
bootstrap(server_url, database_name)
= PostgreSQL server already exists
= FACTSTR may create the target database
= FACTSTR then uses the normal connect path
= FACTSTR initializes or validates its own schema inside that database
```

FACTSTR does not provision, install, run, or host PostgreSQL.

PostgreSQL server provisioning remains external.

Bootstrap requires credentials with permission to inspect `pg_database` and create the target database.

Bootstrap database names must use the FACTSTR-supported simple identifier form:

```text
[A-Za-z_][A-Za-z0-9_]*
```

Do not use quoted exotic PostgreSQL database names in the bootstrap path.

Do not replace FACTSTR bootstrap with manual `sqlx` database creation logic unless the task explicitly asks to compare or test such behavior.

## Rust implementation rules

Use the Rust-specific local skills when working in `rust/`.

For application bootstrap work, use:

```text
.agents/skills/rust/project-bootstrap/SKILL.md
```

For feature-slice work, use:

```text
.agents/skills/rust/feature-slice/SKILL.md
```

The current Rust implementation should remain a small Axum API with explicit wiring.

Keep `main.rs` focused on application startup and wiring.

Keep configuration, logging, routing, health, store initialization, and feature code in clearly named files or folders.

Prefer names that describe owned behavior.

Avoid generic module or folder names such as:

* `core`
* `domain`
* `shared`
* `common`
* `utils`
* `helpers`
* `services`
* `managers`
* `repositories`
* `models`
* `entities`
* `adapters`

Do not create a generic application service layer around FACTSTR.

Do not create a repository abstraction over `PostgresStore`.

Do not centralize feature behavior in global modules.

## TypeScript / Node implementation rules

Use `@factstr/factstr-node` for TypeScript/Node FACTSTR usage.

Keep the TypeScript/Node implementation aligned with the Rust implementation in observable business behavior.

Use the Node package’s public store APIs directly.

The PostgreSQL boundaries must match Rust:

```ts
new FactstrPostgresStore(databaseUrl)
```

means the target database already exists.

```ts
FactstrPostgresStore.bootstrap({
  serverUrl,
  databaseName,
})
```

means FACTSTR may create the target database from an existing PostgreSQL server connection.

Do not reimplement FACTSTR database creation in TypeScript.

Do not duplicate FACTSTR schema initialization in TypeScript.

Do not create a TypeScript repository abstraction around FACTSTR.

Do not introduce framework-specific routing, dependency injection, or service structures unless the task explicitly asks for that implementation style.

## Feature slice rules

New business behavior belongs in feature slices.

A command feature owns the behavior required to handle one command.

A query feature owns the behavior required to answer one query or maintain one read model.

Feature slices should be self-contained.

A command feature may:

* parse or receive a request
* load relevant facts
* build the command context
* validate local input
* decide what facts to append
* append facts with FACTSTR
* return a response

A query feature may:

* query FACTSTR directly
* read a feature-owned projection
* maintain a projection from streams or durable streams
* return a response

Do not put feature decisions in shared services.

Do not create a central domain model.

Do not make features depend on each other by importing internal feature code.

Events are the shared application facts. Feature behavior is local.

## Event definition rules

Application event definitions may be shared inside one language implementation.

In Rust, the expected shared event area is:

```text
rust/src/events/
```

Use shared event definitions for:

* event type constants
* event payload structs
* simple fact construction helpers when useful
* serialization/deserialization boundaries

Do not put these into shared event definitions:

* command decision logic
* validation workflows
* append logic
* query handlers
* projection update logic
* HTTP handlers
* feature orchestration

Event definitions describe facts. They do not own feature behavior.

## Storage and behavior rules

Writes append facts.

Facts are immutable.

Normal feature behavior must not update or delete facts.

Use conditional append when a command depends on a context version.

Do not model write side as mutable CRUD tables first.

Do not use database tables as the primary feature state when the feature is meant to be fact-driven.

Read APIs should use one of these explicit paths:

1. direct FACTSTR query
2. feature-owned in-memory projection
3. feature-owned persisted projection
4. durable subscription-backed projection

Read models are not the source of truth.

Projections are owned by the feature that reads them.

## Query and projection rules

Use FACTSTR queries to select relevant facts.

Use `EventQuery` and `EventFilter` where applicable.

Do not query or stream all facts and then apply broad ad-hoc filtering in application code when FACTSTR can express the relevant scope.

Use future-only streams when a feature only needs live updates.

Use durable streams when a feature needs replay/catch-up across restart.

Use `stream_all` or `stream_all_durable` only when the feature truly owns all facts in scope.

Stream delivery semantics matter:

* notifications happen only after successful persistence
* failed conditional append emits no delivery
* one committed append batch is delivered as one committed batch
* filtered streams deliver only matching committed facts
* delivery follows the committed sequence order
* handler failure does not roll back append success
* durable replay starts after the stored cursor
* durable replay/catch-up must avoid duplicates and gaps according to FACTSTR semantics

Keep unrelated facts out of a projection with query/filter definitions, not with broad subscriptions and ad-hoc filtering after delivery.

## Testing and verification

Tests are part of the reference implementation.

When changing Rust code, run the implementation’s expected checks, usually:

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

Run these from the relevant implementation directory unless the README says otherwise.

When changing TypeScript/Node code, run the package’s actual scripts. Do not invent scripts that are not present.

When touching PostgreSQL bootstrap, store initialization, or persistence behavior, verify against a live PostgreSQL server when possible.

For PostgreSQL bootstrap behavior, tests should prove the relevant claims:

* target database does not exist before bootstrap when the test claims creation
* FACTSTR bootstrap creates the target database
* normal connect still works against an existing database
* append works after store initialization
* query works after store initialization
* conditional append works when used
* durable stream setup works when used
* invalid bootstrap database names fail clearly
* test cleanup drops databases created by the test

If a live PostgreSQL server is unavailable, state that explicitly.

Do not report database bootstrap behavior as verified when only compilation ran.

## Documentation expectations

This repository is a reference implementation, so documentation is part of the work.

Update docs when:

* a new feature is added
* a new business behavior is introduced
* a new FACTSTR usage pattern is introduced
* a language implementation gains or lacks behavior compared with another implementation
* bootstrap, configuration, or runtime setup changes

Keep documentation precise.

Do not rewrite broad documentation during narrow implementation tasks.

Prefer small updates in the relevant README or docs page.

Say:

* FACTSTR stores immutable facts
* FACTSTR initializes the schema it owns
* FACTSTR bootstrap can create the target PostgreSQL database when explicitly requested
* PostgreSQL server provisioning remains external
* credentials need the required PostgreSQL permissions
* projections are feature-owned read models

Do not say:

* FACTSTR provisions PostgreSQL
* FACTSTR runs PostgreSQL
* bootstrap is only schema initialization
* projections are the source of truth
* the application must create FACTSTR tables manually

## What to avoid

Do not introduce:

* CRUD-first write models
* generic repository patterns around FACTSTR
* generic service layers
* manager objects
* helper buckets
* utility buckets
* central domain models
* shared business logic folders
* speculative abstractions for future languages
* cross-feature dependencies through internal feature modules
* manual FACTSTR schema or DDL in application code
* broad refactors without direct behavior gain
* hidden behavior differences between language implementations

Do not move code into generic folders just to make the structure look familiar.

Do not add abstractions only to make stores swappable. FACTSTR already provides store implementations behind its own contract.

## Required response format for implementation work

When reporting implementation work, include:

1. behavior added or changed
2. files changed
3. FACTSTR crates or npm packages used
4. exact FACTSTR versions used
5. whether released packages or git dependencies were used
6. if git dependencies were used, the pinned commit SHA
7. which store implementation was used
8. whether PostgreSQL used `connect` or `bootstrap`
9. how facts are appended
10. whether reads are direct queries, projections, or durable subscriptions
11. verification commands run
12. live checks skipped, if any, and why
13. remaining intentional limitations

## Definition of done

A change is done when:

* the intended behavior is clear
* the code follows the relevant local skill
* FACTSTR usage is explicit and uses the correct released API
* feature behavior stays inside the owning feature
* event definitions remain facts, not workflows
* no generic architecture drift was introduced
* no manual FACTSTR schema handling was added
* Rust and TypeScript behavior remains aligned where both implementations exist
* tests prove the behavior that changed
* docs explain new runtime or feature behavior precisely
* skipped live checks are stated honestly

