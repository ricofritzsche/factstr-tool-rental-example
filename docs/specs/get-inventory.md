# Domain Feature Spec: Get Inventory

## Purpose

A tool rental business needs a current view of its inventory.

The business wants to see which registered tools exist, where they currently are, and whether each tool is available or checked out.

The current inventory is not the source of truth. It is a maintained view derived from recorded facts.

## Business Capability

Show the current inventory.

## Maintained View

This feature maintains a current inventory view from these recorded facts:

- `tool-registered`
- `tool-checked-out`
- `tool-returned`

The view contains one item per registered tool.

The view can be rebuilt or caught up from the recorded fact history.

## Inventory Status

A tool has one of these statuses:

- `available`
- `checked_out`

## Status Rules

A registered tool is available until it is checked out.

A checked-out tool becomes available again when it is returned.

For this example:

- after `tool-registered`, the tool is `available`
- after `tool-checked-out`, the tool is `checked_out`
- after `tool-returned`, the tool is `available`

## Inventory Item Data

Each inventory item contains:

- `tool_id`
- `serial_number`
- `name`
- `category`
- `manufacturer`
- `model`
- `home_location`
- `current_location`
- `status`
- `checked_out_to`
- `due_back_at`

## Location Rules

The `home_location` is the location recorded when the tool was registered.

The `current_location` changes as facts are applied:

- after `tool-registered`, `current_location` is the registered `home_location`
- after `tool-checked-out`, `current_location` is the checkout `use_location`
- after `tool-returned`, `current_location` is the `returned_to_location`

## Checkout Data Rules

When a tool is `available`:

- `checked_out_to` is empty
- `due_back_at` is empty

When a tool is `checked_out`:

- `checked_out_to` is the value from the latest checkout
- `due_back_at` is the value from the latest checkout

## Applying Facts

### `tool-registered`

When a `tool-registered` fact is applied, the view creates one inventory item.

The item starts with:

- status: `available`
- current location: registered `home_location`
- checked-out-to: empty
- due-back-at: empty

### `tool-checked-out`

When a `tool-checked-out` fact is applied, the matching inventory item is updated.

The item changes to:

- status: `checked_out`
- current location: checkout `use_location`
- checked-out-to: checkout `checked_out_to`
- due-back-at: checkout `due_back_at`

### `tool-returned`

When a `tool-returned` fact is applied, the matching inventory item is updated.

The item changes to:

- status: `available`
- current location: return `returned_to_location`
- checked-out-to: empty
- due-back-at: empty

## Unknown Tool Facts

If a checkout or return fact refers to a tool that is not present in the inventory view, the fact is ignored by this view.

The command features are responsible for preventing such facts during normal use.

The inventory view does not become a second validation layer.

## Result

The feature returns the current inventory as a list of inventory items.

The default result includes all registered tools.

## Ordering

The returned inventory list is ordered by:

1. `category`
2. `name`
3. `serial_number`

This keeps the result stable and readable.

## Empty Inventory

When no tools have been registered, the feature returns an empty list.

## Out Of Scope

This feature does not cover:

- registering a tool
- checking out a tool
- returning a tool
- editing tool data
- deleting tools
- retiring tools
- reservations
- maintenance
- pricing
- billing
- customers
- user accounts
- authentication
- authorization
- filtering inventory
- searching inventory
- pagination
- overdue detection
- HTTP endpoint design
- database schema design