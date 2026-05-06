# Domain

## Tool Rental System

This example is about a small tool and equipment rental business.

The business rents physical tools to professional users such as electricians, plumbers, HVAC technicians, carpenters, contractors, and field-service teams.

The example focuses on one operational problem:

**Which tools are available right now, and which tools are currently rented out?**

The system keeps a factual history of tool registration, checkout, and return. The current inventory is derived from that history.

## Scope

This example covers:

* registering tools
* checking tools out
* returning tools
* showing the current tool inventory

This example does not cover:

* billing
* pricing
* reservations
* customer accounts
* authentication
* maintenance
* retirement
* advanced reporting
* a user interface

## Tool Status

A tool can be in one of two current states.

### Available

The tool is registered and not currently rented out.

### Rented

The tool is currently checked out.

## Core Operations

### Register Tool

A new tool is added to the fleet.

Typical information:

* tool id
* serial number
* name
* category

Resulting fact:

* `tool-registered`

### Check Out Tool

A tool is checked out to a customer, worker, or job.

This is only allowed when the tool is currently available.

Resulting fact:

* `tool-checked-out`

A tool that is already rented cannot be checked out again.

### Return Tool

A rented tool is returned.

This is only allowed when the tool is currently rented.

Resulting fact:

* `tool-returned`

The return can include a short condition note.

## Current Tool Inventory

The current inventory is a read model built from facts.

It shows every known tool with its current status:

* available
* rented

The inventory is not the source of truth. It is derived from the fact history.

## Facts

Facts are immutable records of what happened.

Initial facts for this example:

* `tool-registered`
* `tool-checked-out`
* `tool-returned`

A committed FACTSTR event record contains:

* `sequence_number`
* `occurred_at`
* `event_type`
* `payload`

In Rust, the current record shape is:

```rust
pub struct EventRecord {
    pub sequence_number: u64,
    pub occurred_at: OffsetDateTime,
    pub event_type: String,
    pub payload: Value,
}
```

New facts are appended with an event type and a payload. FACTSTR assigns the sequence number and occurrence time when the fact is committed.

There is no event-level `context` field.

The relevant command context is defined by reading the facts needed for a command decision.

## Command Context

Most commands operate on one tool.

For example, checking out a tool requires knowing the current facts for that tool.

The command flow is:

1. read the facts relevant to the tool
2. decide whether the command is allowed
3. append the new fact only if the relevant facts have not changed

That is the role of command context consistency.

## Example Rule

A tool can be checked out only when its current status is `Available`.

If two users try to check out the same tool at the same time, only one checkout should succeed.

The second command should detect that the command context changed and must not append another `tool-checked-out` fact.

## Why This Example Is Useful

This domain is small, but it has a real consistency rule.

The example can show:

* facts as the source of truth
* current inventory as a projection
* command context consistency through checkout
* duplicate checkout prevention
* a simple read model that stays understandable across languages

It is intentionally not a full rental product.
