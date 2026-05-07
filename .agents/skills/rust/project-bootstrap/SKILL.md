---
name: rust-project-bootstrap
description: Create or review the first working skeleton of a new Rust service, CLI, or application.
---

## Bootstrap Structure For Rust Applications

`main.rs` must stay a wiring entry point.

For a real service, CLI, or application bootstrap, do not put configuration loading, logging setup, route construction, store opening, and lifecycle handling all into `main.rs`.

A small set of concern-owned files is allowed from the first runnable version when each file is used immediately and owns real startup behavior.

Good bootstrap files:

* `config.rs` for environment/config loading and validation
* `logging.rs` for `tracing` / `tracing-subscriber` setup
* `routes.rs` or `http_routes.rs` for constructing the HTTP router
* `health.rs` for the health endpoint when the app is a service
* `store.rs` or `open_factstr_store.rs` for opening the concrete store used by the app
* `command_line.rs` for CLI argument parsing when the app is a CLI

Do not create empty folders or placeholder modules.

Do not create generic technical buckets such as:

* `services`
* `managers`
* `repositories`
* `helpers`
* `utils`
* `common`
* `shared`
* `core`
* `domain`

The first service bootstrap may look like this:

```text
src/
  main.rs
  config.rs
  logging.rs
  routes.rs
  health.rs
  store.rs
```

The first CLI bootstrap may look like this:

```text
src/
  main.rs
  config.rs
  logging.rs
  command_line.rs
  store.rs
```

Use `src/lib.rs` only when the project needs a library crate surface for tests, examples, or multiple binaries. Do not add `src/lib.rs` only to move code away from `main.rs`.

`main.rs` should normally do only this:

1. load and validate configuration
2. initialize logging
3. open required resources
4. build the router or command flow
5. run the application
6. return a clear error on startup failure

The implementation details of those steps belong in the concern-owned files above.
