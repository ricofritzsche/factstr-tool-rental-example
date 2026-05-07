---
name: feature-slice-rust
description: Create or modify a self-contained Rust feature slice with local ownership, visible IO, clear pure/shell separation, and mirrored tests under /tests. Avoid generic technical roles, cross-feature reuse, and one-file collapse.
---


# Feature Slice

## Shared Event Definitions

Application event definitions live in:

```text
src/events/
```

This is the only shared application-semantic folder allowed by default.

It may contain:

* event type constants
* event payload types
* event construction functions
* small event classification functions when they are purely about event shape

It must not contain:

* command decisions
* feature workflows
* validation rules
* context loading
* append logic
* projection logic
* HTTP handlers
* cross-feature behavior

Events are shared because facts are the application’s shared source of truth.

Feature behavior stays inside the feature that owns it.

Do not create:

* `src/domain/`
* `src/shared/`
* `src/common/`
* `src/core/`
* `src/models/`
* `src/entities/`

If something is not an event definition, it does not belong in `src/events/`.

## Naming Inside A Feature

The feature folder names the behavior.

Inside the feature, prefer stable local role names so similar features are easy to compare.

Good command feature shape:

```text
src/features/check_out_tool/
  mod.rs
  process_request.rs
  load_context.rs
  build_context.rs
  generate_consequences.rs
  append_consequences.rs
```

Good query feature shape:

```text
src/features/get_current_inventory/
  mod.rs
  process_request.rs
  load_context.rs
  build_context.rs
```

The internal file names describe their role inside the feature. The feature folder provides the domain context.

### File roles

`process_request.rs`

Owns the local feature flow.

It may contain small request/response definitions when they are only used by this feature.

It wires the steps together:

1. receive input
2. load context
3. build context
4. generate consequences or read result
5. append/persist when needed
6. return output

It should stay readable. If it grows too much, split only by real ownership.

`load_context.rs`

Shell code.

Owns reads needed before a decision or query result can be built.

Examples:

* load facts from FACTSTR
* load a projection row
* call an external dependency if the feature truly needs it

`build_context.rs`

Pure code when possible.

Turns loaded facts or rows into the explicit context needed by the decision.

It should not perform IO.

`generate_consequences.rs`

Pure code.

Owns the actual decision.

Input:

* request
* built context

Output:

* consequences to append or persist
* rejection / conflict / validation result

It should not perform IO.

`append_consequences.rs`

Shell code.

Owns appending or persisting the consequences produced by the pure decision.

For FACTSTR-backed commands, this is where `append` or `append_if` belongs.

## Request And Response Shapes

Do not create `request.rs` and `response.rs` by default.

Small request and response types may live in:

* `process_request.rs`
* `http_handler.rs`

Extract separate request/response files only when the boundary shape becomes large enough to deserve its own file.

## Functional Core / Imperative Shell

Use the split because it clarifies behavior, not because every feature must have many files.

Pure files:

* `build_context.rs`
* `generate_consequences.rs`

Shell files:

* `process_request.rs`
* `load_context.rs`
* `append_consequences.rs`
* `http_handler.rs` when an HTTP boundary exists

Pure code must be testable without FACTSTR, databases, HTTP, files, or external systems.

Shell code makes IO visible.

## What To Avoid

Do not create generic technical roles such as:

* `service.rs`
* `manager.rs`
* `repository.rs`
* `helper.rs`
* `utils.rs`
* `common.rs`
* `shared.rs`
* `core.rs`
* `domain.rs`

Do not create HTTP-verb files such as:

* `post_tool.rs`
* `get_inventory.rs`
* `put_tool.rs`
* `delete_tool.rs`

Do not import another feature’s internal files by default.

Shared events may be imported from `src/events/`.

Shared infrastructure may be imported when it is truly infrastructure, such as:

* config
* logging
* database pool
* FACTSTR store opening
* HTTP router setup

Feature behavior remains local.
