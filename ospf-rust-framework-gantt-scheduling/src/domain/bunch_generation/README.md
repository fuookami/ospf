# Bunch Generation

:us: English | :cn: [简体中文](README_ch.md)

This directory implements pricing-side task bunch generation. It maps the Kotlin `gantt-scheduling-domain-bunch-generation-context` module.

## Responsibilities

- Build candidate bunches for column generation pricing.
- Represent task transition graphs and label states.
- Apply feasibility and slot constraints before returning generated columns.
- Support planned-task, unplanned-task, and slot-based generation flows.

## Modules

- `model.rs`: graph and bunch-generation model structures.
- `label.rs`: label state and label expansion helpers.
- `pricing.rs`: `BunchPricingProblem` and `LabelSettingAlgorithm`.
- `service.rs`: bunch generators, feasibility policy, slot constraints, and generation config.

## Public API

- `BunchPricingProblem`
- `LabelSettingAlgorithm`
- `BunchGenerationConfig`
- `BunchFeasibilityPolicy`
- `DefaultBunchFeasibilityPolicy`
- `PlannedTaskBunchGenerator`
- `UnplannedTaskBunchGenerator`
- `SlotBasedBunchGenerator`

## Extension Points

Add pricing behavior through `BunchFeasibilityPolicy`, `BunchGenerationConfig`, slot constraints, label-state extensions, or specialized generator implementations. Keep master-problem column registration in `bunch_compilation`.

## Lifecycle and Data Flow

Pricing receives task graph data, shadow prices, and generation configuration, runs label-setting or slot-based generation, filters candidate bunches through feasibility policies, and returns columns that bunch compilation can register in the master problem.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing label expansion, feasibility policy behavior, planned/unplanned generation, or slot-based generation.

## Related Directories

- [`../bunch_compilation`](../bunch_compilation/README.md)
- [`../task`](../task/README.md)
- [`../../application/algorithm`](../../application/algorithm/README.md)
