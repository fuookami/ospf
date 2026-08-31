# ospf-rust-framework-csp1d

[中文](README_ch.md)

`ospf-rust-framework-csp1d` is the Rust migration of Kotlin `ospf-kotlin-framework-csp1d`. It provides a reusable one-dimensional cutting stock framework and keeps downstream request DTOs, formula languages, tenant context, heartbeat logic, and solver plugin selection outside the shared domain crate.

## Module Structure

The Kotlin implementation is split into several Gradle modules. The Rust migration keeps the same ownership boundaries inside one crate:

| Rust module | Kotlin module boundary | Description |
|-------------|------------------------|-------------|
| `infrastructure` | `csp1d-infrastructure` | Render DTOs and serialization boundary. |
| `domain::material` | `csp1d-domain-material-context` | Products, demands, materials, machines, costars, cutting plans, quantities, and shadow-price keys. |
| `domain::cutting_plan_generation` | `csp1d-domain-cutting-plan-generation-context` | Simple, DFS, N-Same, N-Sum, FullSum, Costar filler, reduced-cost pricing, generation constraints, statistics, and benchmark snapshots. |
| `domain::produce` | `csp1d-domain-produce-context` | Master-problem input, produce aggregation, MetaModel registration context, incremental column lifecycle, extension points, policies, and shadow-price lifecycle. |
| `domain::yield` | `csp1d-domain-yield-context` | Under/over-production analysis, yield slack aggregation, yield constraints, and yield objective terms. |
| `domain::wasting_minimization` | `csp1d-domain-wasting-minimization-context` | Trim width, rest material, material cost, over-production area, and waste objective terms. |
| `domain::length_assignment` | `csp1d-domain-length-assignment-context` | Dynamic product length assignment, assigned/over-length slack aggregation, bounds, and length objective terms. |
| `application` | `csp1d-application` | Assignment helper, problem/config builders, MILP and column-generation entries, schedule entry, warm start, recovery, solution enrichment, KPI, trace, and render output. |

## Current Boundary

The public Rust model covers the Kotlin framework-level entities: `Product`, `ProductDemand`, `Production`, `Costar`, `Material`, `Machine`, `CuttingPlanSlice`, `CuttingPlan`, demand contributions, `Csp1dAssignment`, yield/waste/length outputs, render DTOs, warm start, recovery, and extension policies.

Downstream-only concepts such as business request protocols, formula languages, project runtime parameters, tenant context, heartbeat logic, defect-specific constraints, segmentation, or solver plugin orchestration are intentionally left to adapters.

The migration is still in progress. Plain MILP, LP relaxation, and the final MILP stage currently use deterministic heuristic backends while keeping the Kotlin-compatible public surface and model-registration path. The real LP RMP dual source, solver adapter, and final MILP backend are still pending.

## Basic Use

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

`solve_config` can be attached to `Csp1dProblem`, passed directly to `solve(...)` or `solve_with_trace(...)`, or supplied through service-level defaults such as configured generators and warm-start usages. Explicit solve arguments take priority over problem defaults. `Csp1dSchedule` is also available as the Kotlin-compatible scheduling entry and delegates to column generation by default.

For low-level Kotlin `Csp1dMilpSolver` parity, Rust exposes `Csp1dMilpSolver::solve(...)` and `Csp1dMilpSolver::solve_lp(...)` over `ProduceInput<V>`. These return the built `MetaModel`, selected variable values, framework shadow-price maps, and lightweight `ShadowPriceMap<V>` data needed by pricing.

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

## Shadow Price Lifecycle

Rust keeps the Kotlin CGPipeline shape for shadow-price-aware constraints. `DemandConstraintPipeline`, `MaterialConstraintPipeline`, `MachineConstraintPipeline`, and `YieldConstraintPipeline` register constraints with `Csp1dShadowPriceKey` metadata and implement `Csp1dCGPipeline`.

`Csp1dShadowPriceLifecycle` refreshes framework shadow prices from model constraints and a dual slice, converts them into lightweight `ShadowPriceMap<V>`, and exposes plan-level contribution extraction for pricing. The current service path feeds a placeholder optimistic dual until the real LP solver backend is wired in, but the refresh/extractor API is already in place.

## Generation Semantics

The crate exposes `SimpleInitialCuttingPlanGenerator`, `DFSGenerator`, `NSameGenerator`, `NSumGenerator`, `FullSumGenerator`, `CostarFiller`, `SimplePricingGenerator`, and `ReducedCostPricingGenerator`.

Generation supports max/min knife constraints, max over-production length filtering, canonical de-duplication, candidate filters, width feasibility override from domain policies, same-contribution and cross-contribution dominance pruning, quantity cache statistics, material width-index cache statistics, and sequential slice-template cache statistics. Parallel large-scale generation and the full Kotlin statistics detail set are still being migrated.

## Outputs

`Csp1dSolution<V>` contains selected produce data, optional yield/waste/length results, generated plans, KPI details, render schema, solution status, failure message, and optional Top-K cutting plans.

`Csp1dColumnGenerationTrace` records initial/final plan counts, priced plan counts, iteration records, termination reason, initial/pricing statistics, final MILP status, partial availability, and failure messages.

`RenderSchemaDTO`, `RenderCuttingPlanDTO`, `RenderCuttingPlanProductionDTO`, and `RenderProductionType` are available from `infrastructure::dto`. Enable the `serde` feature to derive `Serialize` / `Deserialize`; DTO fields use Kotlin-style camelCase names such as `cuttingPlans`, `unitLength`, and `standardWidth`.

## Validation

Focused Rust checks:

```powershell
cargo check -p ospf-rust-framework-csp1d
cargo test -p ospf-rust-framework-csp1d
cargo check -p ospf-rust-framework-csp1d --features serde
cargo test -p ospf-rust-framework-csp1d --features serde render_schema_serializes_with_kotlin_camel_case_fields
cargo test -p ospf-rust-core remove_constraints_by_group_id --lib
```

Known pending work:

- Replace heuristic MILP/LP/final-MILP backends with real solver adapters.
- Drive shadow prices from real LP duals instead of placeholder dual values.
- Finish the full Kotlin generator statistics and large-scale/parallel cache behavior.
- Expand Kotlin fixture parity, real-solver warm-start smoke coverage, and module-level README files.
