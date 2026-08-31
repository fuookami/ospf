# Bunch Generation

[中文](README_ch.md)

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

## Related Directories

- [`../bunch_compilation`](../bunch_compilation)
- [`../task`](../task/README.md)
- [`../../application/algorithm`](../../application/algorithm/README.md)
