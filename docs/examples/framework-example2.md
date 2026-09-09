# Framework Example 2: Aircraft Cargo Load Planning — Overview

[中文](../zh-cn/examples/framework-example2)

## 1. Overview

This example documents aircraft cargo loading domains and their mode-dependent registration. It is an overview; the eleven bounded-context contracts are maintained on separate pages.

## 2. Context map and dependencies

The common dependency direction is `aircraft → stowage → {mac, airworthiness_security, soft_security, mac_optimization, express_effectiveness, loading_effectiveness, redundancy, recommended_weight_equalization, payload_maximization}`. LoadingOrder, FullLoad, Predistribution, and WeightRecommendation register different subsets.

## 3. Concepts, sets, and predicates

`I` is the cargo-item set, `J` the aircraft-position set, and `P` the flight-phase set. Predicates classify assigned items, empty positions, loading zones, valid phases, and active mode-specific pipelines. Aircraft is configuration data; stowage and derived safety domains provide optimization symbols.

## 4. Variables and intermediate values

Core stowage symbols are assignment `x_{ij}`, adjustment `u_{ij}`, loaded amount `y_j`, and recommended amount `z_j`. Derived values include position load `L_j`, torque/MAC by phase, zone density, payload, empty-position indicators, and recommendation deviation. Active symbols depend on the selected mode.

## 5. Assertions, constraints, and objective

Registered constraints include item assignment, adjustment bounds, loading limits, MAC/torque and airworthiness envelopes, soft-security penalties, loading-order rules, redundancy limits, recommended-weight equalization, and payload maximization. A context page does not imply registration in every mode.

## 6. Algorithms and lifecycle

The application selects a mode, initializes aircraft/stowage data, registers mode-specific pipelines, optionally builds Benders decomposition, solves the MILP, and analyzes the selected load plan.

## 7. Register → construct → solve → analyze

Mode selection determines the registration boundary. `register` creates variables and pipelines for that mode; `construct` creates the meta-model; `solve` runs the selected MILP/Benders path; `analyze` returns item positions, load, MAC, and security results.

## 8. Source and verification

- [Kotlin Demo2 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo2)
- [Rust Demo2 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo2.rs)

## 9. Kotlin/Rust comparison and design decisions

The two implementations share the context vocabulary, but mode registration and variable bounds must be checked against each language's pipeline generator. The context pages describe the active mathematical boundary rather than a single global model.

## 10. Context model pages

[Open the Demo2 context index](framework-example2/domain-models), which links aircraft, stowage, MAC, security, effectiveness, redundancy, recommendation, and payload contexts.

## 11. Change log

| Version | Change | Reason |
|---|---|---|
| 1.0 | Standardized overview structure and mode boundary | Make context registration explicit |

