# Application Services

:us: English | :cn: [简体中文](README_ch.md)

This directory provides ergonomic constructors and default policies for application-level solving flows. It is intentionally thin: service functions assemble algorithms and domain contexts instead of building optimization expressions directly.

## Responsibilities

- Provide ready-to-compose constructors for task and bunch solving flows.
- Keep default policies discoverable at the application edge.
- Forward domain customization through algorithm policies, hooks, and context parameters.

## Modules

- `task.rs`: helper for creating task column generation workflows.
- `bunch.rs`: helpers for bunch column generation, branch-and-price construction, fresh-model search, and default bunch generation policy.

## Public API

- `create_task_column_generation`
- `create_bunch_branch_and_price`
- `search_bunch_branch_and_price_with_fresh_model`
- `search_bunch_branch_and_price_with_hooks`
- `DefaultBunchGenerationPolicy`

## Extension Points

Application callers should use this directory when they need a ready-to-compose solver workflow. Domain extensions, cost formulas, feasibility rules, and custom callbacks should be injected through the policy and hook types exposed by `application::algorithm` and `domain`.

## Lifecycle and Data Flow

Service constructors assemble algorithm policies, domain compilation/generation contexts, and optional hooks, then return application-level workflows. The returned workflows still delegate model registration and pricing semantics to domain modules.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` and include service constructor paths when changing defaults or hook wiring.

## Related Directories

- [`../algorithm`](../algorithm/README.md)
- [`../../domain`](../../domain/README.md)
