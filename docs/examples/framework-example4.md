# Framework Example 4: Flight Recovery Branch-and-Price — Overview

[中文](../zh-cn/examples/framework-example4)

## 1. Overview

Demo4 is an architecture sample. `Application` is empty, while the generic quantity sample is executable; context pages describe implemented contracts without claiming that one production model is assembled.

## 2. Context map and dependencies

Task, rule, crew, and cargo data feed bunch generation; generated bunches feed bunch compilation; bunch selection coordinates branch-and-price policy. Passenger contributes optional compilation pipelines.

## 3. Concepts, sets, and predicates

The vocabulary includes flight tasks, links, crew members, cargo, passengers, feasible bunches, generated columns, and selected columns. Predicates classify feasible task transitions, compatible resources, and columns accepted by compilation.

## 4. Variables and intermediate values

The compilation context may select bunch columns `x_b` and expose task/aircraft/link coverage expressions. Generation and selection contexts are services and callbacks; they do not create an independent master variable family.

## 5. Assertions, constraints, and objective

Implemented compilation limits cover task/link/fleet/capacity relationships and generated-column consistency. Passenger pipelines add constraints and objective terms when registered. A complete global branch-and-price objective is not present in `Application`.

## 6. Algorithms and lifecycle

The intended flow is policy construction, shadow-price initialization, bunch generation, master compilation, reduced-cost evaluation, and branching. The generic quantity sample demonstrates quantity and linear-symbol APIs only.

## 7. Register → construct → solve → analyze

Each context registers its own contract. Because the top-level `Application` is empty, there is no current end-to-end register/solve/analyze path for a runnable model.

## 8. Source and verification

- [Kotlin Demo4 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
- [Rust Demo4 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo4.rs)

## 9. Kotlin/Rust comparison and design decisions

Both implementations are architecture references. Context registration and generic quantity behavior must be checked from each source; no unimplemented global solver model is inferred.

## 10. Context model pages

[Open the Demo4 context index](framework-example4/domain-models), which links task, rule, crew, cargo, passenger, bunch generation, bunch compilation, and bunch selection.

## 11. Change log

| Version | Change | Reason |
|---|---|---|
| 1.0 | Standardized architecture overview and implementation boundary | Avoid implying an unimplemented global model |

