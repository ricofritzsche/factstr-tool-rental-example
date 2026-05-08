# Domain Feature Spec: Return Tool

## Purpose

A tool rental business needs a way to record that a checked-out tool has come back.

Returning a tool means the business records that the tool is no longer rented out and can become available again.

## Business Capability

Return a checked-out tool.

## Domain Fact

Successful return records this fact:

`tool-returned`

## Domain Rule

A tool can be returned only when it has been registered and is currently checked out.

A tool is currently checked out when the latest relevant tool-status fact means the tool is rented out.

For this example:

- after `tool-registered`, the tool is available
- after `tool-checked-out`, the tool is checked out
- after `tool-returned`, the tool is available again

## Input From The User

The user provides the return information known at return time.

Required:

- `tool_id`
- `returned_at`

Optional:

- `returned_to_location`
- `condition_at_return`

## Recorded Fact Data

When a tool is returned successfully, the recorded `tool-returned` fact contains:

- `tool_id`
- `returned_at`
- `returned_to_location`
- `condition_at_return`

## Default Values

When optional information is not provided or is blank, the return uses these values:

- `returned_to_location`: `unassigned`
- `condition_at_return`: `usable`

These defaults keep the recorded fact complete while keeping the return form small.

## Validation

The return is rejected when one of these required values is missing or blank:

- `tool_id`

The return is rejected when this required timestamp is missing:

- `returned_at`

Text values are stored without surrounding whitespace.

The feature does not validate tool ID format, location values, condition values, business hours, damage workflow, maintenance workflow, or late-return policy in this example.

## Tool Not Registered

Return is rejected when no registered tool exists for the submitted `tool_id`.

From the business perspective, the tool is unknown to the inventory.

## Tool Not Checked Out

Return is rejected when the submitted tool is currently available.

From the business perspective, the tool cannot be returned because it is not currently rented out.

## Successful Result

After a successful return, the caller receives:

- `tool_id`
- `returned_at`
- `returned_to_location`

The result confirms the return without returning a full inventory view.

## Failure Results

Return can fail for these business reasons:

- tool ID is missing or blank
- the return timestamp is missing
- the tool is not registered
- the tool is not currently checked out

Technical failures may also happen, but they are not part of the domain behavior.

## Out Of Scope

This feature does not cover:

- registering a tool
- checking out a tool
- showing current inventory
- editing tool data
- deleting tools
- retiring tools
- reservations
- extending a checkout
- late-return handling
- damage handling
- maintenance workflow
- pricing
- billing
- customers
- user accounts
- authentication
- authorization
- HTTP endpoint design
- database schema design
- projection design