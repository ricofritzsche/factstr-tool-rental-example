# Domain Feature Spec: Check Out Tool

## Purpose

A tool rental business needs a way to hand out a registered tool to a person, team, job, or customer.

Checking out a tool means the business records that the tool has left the available inventory and is currently in use outside its home location.

## Business Capability

Check out an available tool.

## Domain Fact

Successful checkout records this fact:

`tool-checked-out`

## Domain Rule

A tool can be checked out only when it has been registered and is currently available.

A tool is currently available when the latest relevant tool-status fact means the tool is available for checkout.

For this example:

- after `tool-registered`, the tool is available
- after `tool-checked-out`, the tool is not available
- after `tool-returned`, the tool is available again

## Input From The User

The user provides the checkout information known at checkout time.

Required:

- `tool_id`
- `checked_out_to`
- `checked_out_at`
- `due_back_at`

Optional:

- `use_location`
- `condition_at_checkout`

## Recorded Fact Data

When a tool is checked out successfully, the recorded `tool-checked-out` fact contains:

- `tool_id`
- `checked_out_to`
- `checked_out_at`
- `due_back_at`
- `use_location`
- `condition_at_checkout`

## Default Values

When optional information is not provided or is blank, the checkout uses these values:

- `use_location`: `unknown`
- `condition_at_checkout`: `usable`

These defaults keep the recorded fact complete while keeping the checkout form small.

## Validation

The checkout is rejected when one of these required values is missing or blank:

- `tool_id`
- `checked_out_to`

The checkout is rejected when one of these required timestamps is missing:

- `checked_out_at`
- `due_back_at`

The checkout is rejected when `due_back_at` is not later than `checked_out_at`.

Text values are stored without surrounding whitespace.

The feature does not validate tool ID format, recipient format, location values, condition values, business hours, rental duration limits, or calendar availability in this example.

## Tool Isn’t Registered

Checkout is rejected when no registered tool exists for the submitted `tool_id`.

From the business perspective, the tool is unknown to the inventory.

## Tool Already Checked Out

Checkout is rejected when the submitted tool is currently checked out.

From the business perspective, the tool is already rented out and is not available for another checkout.

## Successful Result

After successful checkout, the caller receives:

- `tool_id`
- `checked_out_to`
- `checked_out_at`
- `due_back_at`

The result confirms the checkout without returning a full inventory view.

## Failure Results

Checkout can fail for these business reasons:

- tool ID is missing or blank
- checked-out-to value is missing or blank
- checkout timestamp is missing
- a due-back timestamp is missing
- the due-back timestamp is not later than the checkout timestamp
- the tool is not registered
- tool is already checked out

Technical failures may also happen, but they are not part of the domain behavior.

## Out Of Scope

This feature does not cover:

- registering a tool
- returning a tool
- showing current inventory
- editing tool data
- deleting tools
- retiring tools
- reservations
- extending a checkout
- pricing
- billing
- customers
- user accounts
- authentication
- authorization
- HTTP endpoint design
- database schema design
- projection design