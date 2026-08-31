# Application Services

[中文](README_ch.md)

This directory provides ergonomic constructors and default policies for application-level solving flows. It is intentionally thin: service functions assemble algorithms and domain contexts instead of building optimization expressions directly.

## Modules

- `task.rs`: helper for creating task column generation workflows.
- `bunch.rs`: helpers for bunch column generation, branch-and-price construction, fresh-model search, and default bunch generation policy.

## Public API

- `create_task_column_generation`
- `create_bunch_branch_and_price`
- `search_bunch_branch_and_price_with_fresh_model`
- `search_bunch_branch_and_price_with_hooks`
- `DefaultBunchGenerationPolicy`

## Usage Pattern

Application callers should use this directory when they need a ready-to-compose solver workflow. Domain extensions, cost formulas, feasibility rules, and custom callbacks should be injected through the policy and hook types exposed by `application::algorithm` and `domain`.

## Related Directories

- [`../algorithm`](../algorithm/README.md)
- [`../../domain`](../../domain/README.md)
