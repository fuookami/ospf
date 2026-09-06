# <module-name>

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

Briefly describe what this module provides, where it sits in the `ospf-rust` workspace, and which Kotlin module or package it maps to when relevant.

## Scope

State what belongs in this module.

Explicit non-goals:

1. Business-specific DTOs, formula languages, tenant context, heartbeat logic, and runtime policies unless this module explicitly owns them.
2. Solver backend implementations unless this is a solver backend module.
3. Starter dependency aggregation or demo orchestration unless this is an example or starter module.

## Module Structure

| Rust module or directory | Kotlin boundary | Responsibility |
| --- | --- | --- |
| `<path>` | `<kotlin-module-or-package>` | `<responsibility>` |

## Architecture Overview

For framework modules, describe the context / aggregation / model component / pipeline flow and the `MetaModel` registration path. For utility modules, describe the primary type and data-flow boundaries.

## Core Concepts

List the concepts a reader must understand before using or extending this module.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `<TypeOrFunction>` | `<responsibility>` | stable / migration / internal |

## Modeling Extensions

For framework modules, explain the context, aggregation, model component, pipeline, extra context, or extra pipeline extension points. If this module is not a framework module, omit this section or replace it with the relevant extension surface.

## Generic Numeric Boundaries

Explain whether public APIs use generic numeric types and where conversion to `f64` is allowed.

## Physical Quantity Boundaries

Explain which values use `Quantity<V, U>` or explicit physical-unit wrappers, and which values may remain dimensionless.

## Solve Lifecycle

For framework modules, describe registration, LP/MILP solving, shadow price extraction, column addition/removal, final solve, and solution extraction as applicable.

## Outputs

Describe solution, trace, KPI, render DTO, or diagnostic outputs.

## Usage

```rust
// Minimal example.
```

## Local Validation

```powershell
cargo check -p <crate-name>
cargo test -p <crate-name>
```

## Current Boundaries

For migration modules, describe Kotlin-aligned behavior, Rust-specific substitutes, remaining gaps, and long-term non-goals.

## Related Modules

- [Root project README](../README.md)
