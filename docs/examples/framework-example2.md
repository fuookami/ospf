# Framework Example 2: Aircraft Cargo Load Planning — Overview

[中文](/zh-cn/examples/framework-example2)

## 1. Overview

This example documents aircraft cargo loading domains and their mode-dependent registration. It is an overview; the eleven bounded-context contracts are maintained on separate pages.

## 2. Contexts and Dependencies

| Context | Responsibility | Dependency |
|---|---|---|
| Aircraft | Aircraft, positions, phases, and limits | Input configuration |
| Stowage | Assignment, adjustment, payload, and recommended weight | Aircraft |
| MAC and airworthiness security | Moments, balance, and hard safety limits | Aircraft, stowage |
| Soft security and MAC optimization | Safety deviations and balance preferences | Aircraft, stowage, and related expressions |
| Express and loading effectiveness | Mode-dependent loading preferences | Stowage |
| Redundancy, recommended-weight equalization, payload maximization | Mode-specific objectives and limits | Stowage |

LoadingOrder, FullLoad, Predistribution, and WeightRecommendation register different subsets.

## 3. Concepts, Sets, and Predicates

$I$ is the cargo set, $J$ the position set, and $P$ the flight-phase set. Predicates identify assigned cargo, empty positions, loading areas, valid phases, and mode-specific pipelines. Aircraft supplies configuration data; stowage and safety contexts supply model symbols.

## 4. Variables and Intermediate Values

Core symbols include assignment $x_{ij}$, adjustment $u_{ij}$, payload $y_j$, and recommended weight $z_j$. Intermediate expressions include position load, phase moments and MAC, area density, total payload, empty-position indicators, and recommendation deviations. Their owning context defines the precise domains, units, and activation conditions.

## 5. Assertions, Constraints, and Objectives

Constraints and objectives cover assignment, adjustment ranges, loading limits, moments and airworthiness envelopes, soft-safety deviations, loading order, redundancy, recommendation equalization, and payload maximization. The application mode selects them; one solve does not automatically enable every context.

## 6. Algorithms and Lifecycle

The application selects a mode, initializes aircraft/stowage data, registers mode-specific pipelines, optionally builds Benders decomposition, solves the MILP, and analyzes the selected load plan.

## 7. Register → Construct → Solve → Analyze

Mode selection defines the registration scope. `register` adds its variables and pipelines; `construct` builds the model; `solve` executes the configured MILP or decomposition path; `analyze` returns positions, payload, MAC, and safety results.

## 8. Source Entry Points

- [Kotlin Demo2 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo2)
- [Rust Demo2 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo2)

## 9. Kotlin/Rust Comparison and Design Decisions

Both language entry points belong to the aircraft cargo-load framework example. The context pages follow Kotlin ownership and mode-registration boundaries; different modes must not be conflated into a default all-context model.

## 10. Context Model Pages

[Open the context index](framework-example2/domain-models) and read the 11 contexts in dependency order: aircraft, stowage, MAC, airworthiness security, soft security, MAC optimization, express effectiveness, loading effectiveness, redundancy, recommended-weight equalization, and payload maximization.

## 11. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual overviews, notation, and source entry points | Keep the overview consistent with its context models |
