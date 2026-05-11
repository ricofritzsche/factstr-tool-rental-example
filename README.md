# Tool Rental Example: Command-Context Consistency and Fact-First Thinking with FACTSTR

## Purpose

This repository contains reference implementations of a small tool-rental system built with FACTSTR.

The current implemented version is the Rust implementation. The repository is structured so the same domain behavior can be shown in implementation-specific folders while keeping the core business rules aligned.

Implementation-specific setup, run, and test instructions belong in each implementation README.

Rust implementation: [rust/README.md](rust/README.md)

## FACTSTR

This example is built with FACTSTR.

- GitHub: https://github.com/ricofritzsche/factstr
- Website: https://factstr.com

## Domain Scope

This example demonstrates a small operational slice of a tool-rental business:

- register tools
- check tools out
- return tools
- show current inventory

The current inventory is a maintained view derived from recorded facts. The facts remain the source of truth, while the inventory view shows which tools exist, where they currently are, and whether they are available or checked out.

## Implemented Capabilities

The Rust implementation currently includes:

- Register Tool
- Check Out Tool
- Return Tool
- Get Inventory
- a built-in HTML UI for manual browser testing
- inventory change notifications through Server-Sent Events

## Design Ideas Demonstrated

For a longer explanation of the feature-slice approach used here, see:
[How I structure self-contained feature slices](https://medium.com/@rico-fritzsche/how-i-structure-self-contained-feature-slices-a31d17df5628?sk=826aa678f4b8bf45679f6218cf35c8eb)

- Shared facts  
  The implementation shares application fact definitions such as `tool-registered`, `tool-checked-out`, and `tool-returned` while keeping feature behavior local.

- Command feature slices  
  Each command capability is implemented as its own feature slice that owns request handling, context loading, local decisions, and fact append behavior.

- Command-context consistency with FACTSTR  
  Commands read only the facts relevant to their decision and append conditionally so conflicting changes do not silently succeed.

- HTTP as a delivery boundary  
  HTTP handlers translate requests and responses without moving business rules or FACTSTR usage out of the owning feature.

- Feature-owned maintained inventory projection  
  The current inventory is maintained as a feature-owned read model derived from `tool-registered`, `tool-checked-out`, and `tool-returned`.

- Durable projection cursor and projection state sharing the same PostgreSQL durability boundary  
  The Rust Get Inventory projection keeps its persisted state in PostgreSQL alongside FACTSTR’s durable cursor boundary so restart catch-up stays consistent.

- SSE as an inventory invalidation signal  
  The Rust UI listens for `inventory-changed` notifications and refetches the maintained inventory view instead of treating live notifications as the inventory payload.

## Repository Structure

```text
docs/
  DOMAIN.md
  specs/

rust/
  README.md
  src/
  static/
```

## Out Of Scope

This repository intentionally does not cover:

- billing
- pricing
- reservations
- customer accounts
- authentication
- maintenance
- retirement
- advanced reporting

The example stays focused on a small, inspectable rental workflow and its maintained inventory view.

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
