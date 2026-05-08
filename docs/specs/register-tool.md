# Domain Feature Spec: Register Tool

## Purpose

A tool rental business needs a way to add a physical tool to its inventory.

Registering a tool means the business records that this specific tool exists and can later be checked out, returned, and shown in the current inventory.

## Business Capability

Register a new tool in the rental inventory.

## Domain Fact

Successful registration records this fact:

`tool-registered`

## Domain Rule

A tool can be registered only when no tool with the same serial number has already been registered.

The `serial_number` is the business uniqueness candidate in this example.

The `tool_id` is the system identity assigned to the registered tool.

## Input From The User

The user provides the information known at registration time.

Required:

- `serial_number`
- `name`
- `category`

Optional:

- `manufacturer`
- `model`
- `home_location`
- `initial_condition`

The user does not provide `tool_id`.

## System-Assigned Data

The system assigns:

- `tool_id`

The `tool_id` identifies the tool inside the system and is used by later facts.

## Recorded Fact Data

When a tool is registered successfully, the recorded `tool-registered` fact contains:

- `tool_id`
- `serial_number`
- `name`
- `category`
- `manufacturer`
- `model`
- `home_location`
- `initial_condition`

## Default Values

When optional information is not provided or is blank, the registration uses these values:

- `manufacturer`: `unknown`
- `model`: `unknown`
- `home_location`: `unassigned`
- `initial_condition`: `usable`

These defaults keep the recorded fact complete while keeping the registration form small.

## Validation

The registration is rejected when one of these required values is missing or blank:

- `serial_number`
- `name`
- `category`

Text values are stored without surrounding whitespace.

The feature does not validate serial-number format, category values, or condition values in this example.

## Duplicate Registration

A second registration with the same `serial_number` is rejected.

From the business perspective, the serial number is already assigned to another registered tool.

## Successful Result

After successful registration, the caller receives:

- `tool_id`
- `serial_number`

The result confirms which tool was created without returning a full inventory view.

## Failure Results

Registration can fail for these business reasons:

- serial number is missing or blank
- name is missing or blank
- category is missing or blank
- serial number is already registered

Technical failures may also happen, but they are not part of the domain behavior.

## Out Of Scope

This feature does not cover:

- checking out a tool
- returning a tool
- showing current inventory
- editing registered tool data
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
- HTTP endpoint design
- database schema design
- projection design