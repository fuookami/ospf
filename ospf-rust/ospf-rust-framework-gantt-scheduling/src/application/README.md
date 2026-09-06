# Application Layer

:us: English | :cn: [简体中文](README_ch.md)

This directory contains the application-facing orchestration layer for the Gantt scheduling framework. It maps the Kotlin `gantt-scheduling-application` module, while keeping model construction in domain contexts and pipelines.

## Responsibilities

- Coordinate task-level and bunch-level solving workflows.
- Select and compose column generation, branch-and-price, and service helpers.
- Keep iteration snapshots, trace-friendly state, and public entry markers.
- Delegate variable, constraint, objective, and solution extraction details to `domain`.

## Modules

- [`algorithm`](algorithm/README.md): column generation policies, task column generation, bunch column generation, and branch-and-price tree search.
- [`service`](service/README.md): constructors and default service policies used by application callers.
- `model`: task and bunch iteration models.
- `iteration.rs`: shared iteration and iteration snapshot structures.

## Public API

- `APS`: advanced planning and scheduling marker.
- `MPS`: master production scheduling marker.
- `LSP`: lot scheduling planning marker.
- `create_task_column_generation`
- `create_bunch_branch_and_price`
- `search_bunch_branch_and_price_with_fresh_model`
- `search_bunch_branch_and_price_with_hooks`

## Extension Points

The application layer should not assemble domain variables or hard-code constraint families directly. New business rules should be added through domain contexts, aggregations, pipelines, policies, or solver hooks, then composed here.

## Lifecycle and Data Flow

Callers enter through service constructors or algorithm types, compose task or bunch workflows, run column generation or branch-and-price loops, and receive iteration snapshots plus solver-facing results from domain contexts. The application layer coordinates the flow and keeps model registration delegated to domain modules.

## Verification

Use `cargo check -p ospf-rust-framework-gantt-scheduling` for application entry points and `cargo test -p ospf-rust-framework-gantt-scheduling --lib` for algorithm/service coverage.

## Related Directories

- [`../domain`](../domain/README.md)
- [`../infrastructure`](../infrastructure/README.md)
