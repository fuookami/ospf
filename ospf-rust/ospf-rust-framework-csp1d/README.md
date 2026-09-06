# ospf-rust-framework-csp1d

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework-csp1d` is the Rust migration of Kotlin `ospf-kotlin-framework-csp1d`. It provides a reusable one-dimensional cutting-stock framework and keeps downstream request DTOs, formula languages, tenant context, heartbeat logic, and solver plugin selection outside the shared domain crate.

## Scope

This crate owns the reusable CSP1D kernel: material/product models, cutting-plan generation, master-problem produce modeling, yield/waste/length extensions, render DTOs, solution enrichment, recovery, warm start, and application-level MILP/column-generation entries.

Explicit non-goals:

1. Business request protocols, formula languages, project runtime parameters, tenant context, heartbeat logic, or solver plugin orchestration.
2. Downstream-only defect, segmentation, position, or project-specific constraint models until they become general domain entities.
3. Solver backend installation and license management.

## Module Structure

The Kotlin implementation is split into several Gradle modules. The Rust migration keeps the same ownership boundaries inside one crate:

| Rust module | Kotlin module boundary | Responsibility |
| --- | --- | --- |
| `infrastructure` | `csp1d-infrastructure` | Render DTOs and serialization boundary. |
| `domain::material` | `csp1d-domain-material-context` | Products, demands, materials, machines, costars, cutting plans, quantities, and shadow-price keys. |
| `domain::cutting_plan_generation` | `csp1d-domain-cutting-plan-generation-context` | Simple, DFS, N-Same, N-Sum, FullSum, Costar filler, reduced-cost pricing, generation constraints, statistics, and benchmark snapshots. |
| `domain::produce` | `csp1d-domain-produce-context` | Master-problem input, produce aggregation, `MetaModel` registration context, incremental column lifecycle, extension points, policies, and shadow-price lifecycle. |
| `domain::yield` | `csp1d-domain-yield-context` | Under/over-production analysis, yield slack aggregation, yield constraints, and yield objective terms. |
| `domain::wasting_minimization` | `csp1d-domain-wasting-minimization-context` | Trim width, rest material, material cost, over-production area, and waste objective terms. |
| `domain::length_assignment` | `csp1d-domain-length-assignment-context` | Dynamic product length assignment, assigned/over-length slack aggregation, bounds, and length objective terms. |
| `application` | `csp1d-application` | Assignment helper, problem/config builders, MILP and column-generation entries, schedule entry, warm start, recovery, solution enrichment, KPI, trace, and render output. |

## Architecture Overview

The modeling path is centered on `Csp1dProduceContext` and `ProduceAggregation`. Built-in and extension pipelines register variables, intermediate values, constraints, objectives, and shadow-price metadata into `MetaModel`. Plain MILP, column-generation LP master, and final MILP reuse the same produce context and policy families where possible.

Application services build the problem, choose generator and solve configuration, coordinate MILP/LP/final-MILP stages, apply warm starts or recovery, and assemble `Csp1dSolution` plus trace/KPI/render output. Domain contexts own modeling semantics and should not be duplicated in application solver flow.

## Core Concepts

1. `Product`, `ProductDemand`, `Material`, `Machine`, `Costar`, `CuttingPlanSlice`, and `CuttingPlan` define the reusable material domain.
2. `ProduceInput`, `Produce`, `ProduceAggregation`, and `Csp1dProduceContext` define the master-problem modeling surface.
3. `Csp1dShadowPriceKey` and `Csp1dShadowPriceLifecycle` keep LP dual extraction stable across column generation.
4. Generation strategies create initial and pricing cutting plans with feasibility, dominance, cache, and statistics contracts.
5. Yield, waste, and length-assignment contexts add optional constraint and objective families through standard pipelines.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `csp1d_problem` / `Csp1dProblemBuilder` | Builder DSL for problem construction. | migration |
| `Csp1dMilp` | Plain MILP application entry. | migration |
| `Csp1dColumnGeneration` | Column-generation application entry with trace. | migration |
| `Csp1dSchedule` | Kotlin-compatible scheduling entry that delegates to column generation by default. | migration |
| `Csp1dMilpSolver` | Low-level Kotlin `Csp1dMilpSolver` parity over `ProduceInput<V>`. | migration |
| `Csp1dSolveConfig` / `Csp1dExtensionSet` | Modeling extensions, policy families, Top-K, partial solution, and recovery controls. | migration |
| `Csp1dModelingExtension` / `Csp1dIncrementalPipeline` | Extension pipeline and accepted-column refresh hooks. | migration |
| `SimpleInitialCuttingPlanGenerator`, `DFSGenerator`, `NSameGenerator`, `NSumGenerator`, `FullSumGenerator`, `CostarFiller` | Initial plan generation. | migration |
| `SimplePricingGenerator`, `ReducedCostPricingGenerator` | Pricing generation. | migration |
| `RenderSchemaDTO` and related DTOs | Renderer serialization boundary. | stable within migration |

## Modeling Extensions

`Csp1dSolveConfig<V>` exposes modeling extensions and a policy set. Modeling extensions wrap a `Pipeline<MetaModel<f64>>` and are filtered by `Csp1dExtensionMode`:

- `MILP`: plain MILP only.
- `LP`: column-generation LP master only.
- `FINAL_MILP`: column-generation final MILP only.
- `ALL`: every stage.

Use `Csp1dModelingExtension::new(...)`, `Csp1dModelingExtension::with_mode(...)`, or the builder helpers `extension_pipeline(...)`, `extension_pipeline_with_mode(...)`, `context_aware_extension_pipeline(...)`, and `context_aware_extension_pipeline_with_mode(...)`.

Extensions that need to react to accepted pricing columns can implement `Csp1dIncrementalPipeline`. `Csp1dProduceContext::add_columns(...)` registers new plan variables, refreshes built-in demand/material/machine/yield/length constraint groups, resets the objective, and then lets incremental extensions adjust the confirmed column batch.

## Extension Policies

`Csp1dExtensionSet<V>` aggregates the same policy families as Kotlin:

- `Csp1dDomainPolicy`: candidate feasibility and width-feasibility override.
- `Csp1dObjectivePolicy`: objective batch-coefficient modification.
- `Csp1dGenerationStrategy`: candidate acceptance, canonical key override, and dominance acceptance.
- `Csp1dPricingPolicy`: reduced-cost cost/benefit modification and custom improvement judgment.
- `Csp1dFlowPolicy`: initial plan filtering, column equivalence, early stop, termination selection, partial acceptance, and recovery fallback.
- `Csp1dExtractionPolicy`: solution enrichment into KPI details and render KPI maps.

Policies have no-op defaults that preserve the built-in behavior. Extraction policy failures are caught during enrichment so one downstream output hook cannot break the solve result.

## Generic Numeric Boundaries

Public domain models use generic numeric values where the quantity semantics are reusable. Solver registration currently converts to `MetaModel<f64>` through explicit material/domain conversion helpers such as `to_f64`, `from_f64`, and `convert_solver_value`. Raw `f64` should stay in solver adapter, registration, extraction, and renderer/serialization edges.

## Physical Quantity Boundaries

Widths, lengths, material dimensions, product dimensions, weights, demand amounts, production amounts, rest material, trim width, assigned length, and over-length values should be expressed through `Csp1dQuantity`, `QuantityRange`, `WidthRange`, or explicit quantity wrappers. Bare numeric values are acceptable for dimensionless configuration limits, counts, statistics, and solver-internal coefficients.


## Warm Start and Recovery

### Warm Start

The framework supports warm start from previous solutions:

- **Previous solution warm start**: Accepts `List<CuttingPlanUsage<V>>` where each element pairs a `CuttingPlan` with a usage amount.
- **Native initial values**: Warm start usages are written as native initial assignment values into the final MILP model.
- **Configuration**: `warmStartPlanUsages` parameter in `Csp1dColumnGeneration` constructor.

### Recovery

The recovery mechanism handles solution adaptation:

- **Previous solution recovery**: `Csp1dRecovery` adapts a previous solution to a modified problem.
- **Compatible subset filtering**: Automatically filters usages for cutting plans still valid in the new problem.
- **Machine capacity recovery**: Special handling for machine capacity constraints with yield adjustments.
- **Fallback control**: `retryWithoutWarmStart` flag controls behavior when warm start fails.

### Error Handling

- `Csp1dRecoveryFallbackDisabledException`: Thrown when recovery fails and fallback is disabled.
- `Csp1dRecoverySolveException`: Wraps solver failures with trace context.

## Solve Lifecycle

The tested lifecycle is:

1. Build `Csp1dProblem<V>` and optional `Csp1dSolveConfig<V>`.
2. Generate or accept initial cutting plans.
3. Register `Csp1dProduceContext` into `MetaModel`.
4. For plain MILP, solve and extract `Produce` directly.
5. For column generation, solve LP master, refresh shadow prices, run pricing, and call `add_columns`.
6. Solve final MILP over the refreshed plan pool.
7. Extract produce/yield/waste/length results, KPI details, render DTOs, Top-K plans, trace, and partial/recovery status.

## Shadow Price Lifecycle

Rust keeps the Kotlin `CGPipeline` shape for shadow-price-aware constraints. `DemandConstraintPipeline`, `MaterialConstraintPipeline`, `MachineConstraintPipeline`, and `YieldConstraintPipeline` register constraints with `Csp1dShadowPriceKey` metadata and implement `Csp1dCGPipeline`.

`Csp1dShadowPriceLifecycle` refreshes framework shadow prices from model constraints and a dual slice, converts them into lightweight `ShadowPriceMap<V>`, and exposes plan-level contribution extraction for pricing. The current service path feeds a placeholder optimistic dual until the real LP solver backend is wired in, but the refresh/extractor API is already in place.

## Generation Semantics

Generation supports max/min knife constraints, max over-production length filtering, canonical de-duplication, candidate filters, width feasibility override from domain policies, same-contribution and cross-contribution dominance pruning, quantity cache statistics, material width-index cache statistics, and sequential slice-template cache statistics. Parallel large-scale generation and the full Kotlin statistics detail set are still being migrated.

## Outputs

`Csp1dSolution<V>` contains selected produce data, optional yield/waste/length results, generated plans, KPI details, render schema, solution status, failure message, and optional Top-K cutting plans.

`Csp1dColumnGenerationTrace` records initial/final plan counts, priced plan counts, iteration records, termination reason, initial/pricing statistics, final MILP status, partial availability, and failure messages.

`RenderSchemaDTO`, `RenderCuttingPlanDTO`, `RenderCuttingPlanProductionDTO`, and `RenderProductionType` are available from `infrastructure::dto`. Enable the `serde` feature to derive `Serialize` / `Deserialize`; DTO fields use Kotlin-style camelCase names such as `cuttingPlans`, `unitLength`, and `standardWidth`.

## Usage

Build a problem directly with `Csp1dProblem::new(...)` or through the builder helper:

```rust
use ospf_rust_framework_csp1d::{
    csp1d_problem, Csp1dColumnGeneration, Csp1dConfiguration, Csp1dMilp,
};

let problem = csp1d_problem::<f64, _>(|builder| {
    builder
        .products(products)
        .materials(materials)
        .machines(machines)
        .demands(demands)
        .configuration(Csp1dConfiguration {
            max_initial_plans: 128,
            max_pricing_plans: 32,
            iteration_limit: 16,
        })
        .solve_config_with(|config| {
            config
                .top_k_plan_limit(Some(10))
                .allow_partial_solution(true);
        });
});

let milp_solution = Csp1dMilp::default().solve(problem.clone(), None);
let cg_result = Csp1dColumnGeneration::default().solve_with_trace(problem, None);
```

`solve_config` can be attached to `Csp1dProblem`, passed directly to `solve(...)` or `solve_with_trace(...)`, or supplied through service-level defaults such as configured generators and warm-start usages. Explicit solve arguments take priority over problem defaults.

## Local Validation

```powershell
cargo check -p ospf-rust-framework-csp1d
cargo test -p ospf-rust-framework-csp1d
cargo check -p ospf-rust-framework-csp1d --features serde
cargo test -p ospf-rust-framework-csp1d --features serde render_schema_serializes_with_kotlin_camel_case_fields
cargo test -p ospf-rust-core remove_constraints_by_group_id --lib
```

## Current Boundaries

The public Rust model covers the Kotlin framework-level entities: `Product`, `ProductDemand`, `Production`, `Costar`, `Material`, `Machine`, `CuttingPlanSlice`, `CuttingPlan`, demand contributions, `Csp1dAssignment`, yield/waste/length outputs, render DTOs, warm start, recovery, and extension policies.

The migration is still in progress. Plain MILP, LP relaxation, and the final MILP stage currently use deterministic heuristic backends while keeping the Kotlin-compatible public surface and model-registration path.

Known pending work:

1. Replace heuristic MILP/LP/final-MILP backends with real solver adapters.
2. Drive shadow prices from real LP duals instead of placeholder dual values.
3. Finish the full Kotlin generator statistics and large-scale/parallel cache behavior.
4. Expand Kotlin fixture parity, real-solver warm-start smoke coverage, and module-level README files.

## Related Modules

- [Root README](../README.md)
- [Migration handoff](csp1d.md)
- [Kotlin CSP1D README](../../ospf-kotlin/ospf-kotlin-framework-csp1d/README.md)
