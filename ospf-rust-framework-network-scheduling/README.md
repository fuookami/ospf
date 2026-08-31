# ospf-rust-framework-network-scheduling

:us: English | :cn: [简体中文](README_ch.md)

`ospf-rust-framework-network-scheduling` provides generic directed network/flow primitives and a VRPTW branch-and-price pipeline. This README is the long-lived delivery and validation contract for the crate. The implementation uses `ospf-kotlin/ospf-kotlin-framework-network-scheduling` as its mathematical reference and consumes the workspace [unified solve contract](../docs/solve-contract.md).

## Delivery status

The Network Scheduling migration closed on 2026-08-14 with all `99/99` implementation and acceptance items complete. The shared solver-contract migration it depends on is also complete (`201/201`): public terminal states, proof gates, cancellation, attempt identity, remote reports, CP/Logic-Based Benders, and portable checkpoint contracts are owned by the core/framework solver documentation rather than redefined in this crate.

Recorded validation evidence from the closing environment is summarized below. These are historical execution results, not generated artifacts committed to the repository.

| Gate | Recorded result |
| --- | --- |
| Network default / `serde` / `big-decimal` library tests | `50/50`, `50/50`, `53/53` passed |
| Example default tests | `8/8` passed |
| Network formatting and strict Clippy (`--no-deps`) | passed |
| Gurobi Demo5 library gates | `3/3` passed: 25-customer direct-MIP, 100-customer bounded smoke, and 100-customer strict-proof |
| SCIP Demo5 library gate | `1/1` passed: 25-customer legal-terminal validation |
| Gurobi / SCIP Demo5 integration targets | `3/3` / `2/2` passed |
| 100-customer strict-proof fixture | direct-MIP and Branch-and-Price both `Optimal`; objective, lower bound, and upper bound all `100` |

## Architecture

The crate is intentionally split into four layers:

| Module | Responsibility |
| --- | --- |
| `infrastructure` | Stable graph IDs, generic nodes/arcs, capacity/cost values, and solver-value conversion. |
| `domain::flow` | Single/multi-commodity flow context, conservation, capacity, minimum-cost pipelines, and a small-instance integral-flow oracle. |
| `domain::vrp` | VRPTW entities, runtime units, time windows, route validation, policies, pricing duals, and branch masks. |
| `domain::route_generation` / `domain::route_compilation` | ESPPRC pricing and the incremental Phase I/Phase II restricted master. |
| `application` | Best-bound Branch-and-Price, node limits, deadlines, cancellation, bounds, trace, and solution assembly. |

The application layer receives a `BranchNodeSolver` through the `BranchNodeSolverProvider` contract. Model registration remains in route compilation context/aggregation, so custom compilation extensions can add variables, constraints, shadow-price handling, or solution extraction without changing the application loop.

## Public flow

1. Build a validated `VrptwInstance<V>` with `Quantity<V, Unit>` values and an absolute `TimeWindow<V>`.
2. Select distance, travel-time, arc-cost, route-cost, and optional arc-feasibility policies.
3. Construct `BranchNodeSolver` with an injected `LinearProgrammingSolver` or a core `LinearSolver` adapter.
4. Run `VrptwApplicationService::solve` and inspect the shared `SolveReport<VrptwSolution<V>>`: use `problem_status`, `termination_reason`, `solution_presence`, `statistics`, and `trace`; read a validated route incumbent with `report.incumbent()`. The application no longer publishes a parallel Branch-and-Price terminal result.

The restricted master uses exact customer coverage, finite fleet upper bounds, artificial Phase I variables, and real route costs in Phase II. Pricing is elementary ESPPRC with time, load, reduced-cost, waiting, dominance, Feillet-style unreachable-customer marking, and branch-mask checks. Route signatures retain ordered arc IDs, so parallel arcs remain distinct.

The offline test suite currently covers graph/flow contracts, unit normalization, route validation, exhaustive ESPPRC comparison, Phase I/II lifecycle, extension hooks, failure paths, an MRP-shaped integral-flow oracle, two-vehicle-type assignment/arc branches, and BigDecimal domain values. With `big-decimal`, explicit base arcs can use `ExactBigDecimalEspprcPricer`; label time, load, cost, and reduced cost remain BigDecimal throughout, while implicit complete graphs are explicitly rejected by that exact entry point. The Demo5 Gurobi gate passes the 25-customer direct-MIP comparison, the five-customer full-route master oracle, a bounded Demo17 100-customer smoke, and a separate 100-customer strict-proof fixture whose objective and bounds close at 100; the SCIP gate passes the 25-customer legal-terminal check when the native solver is available.

## Correctness contracts

- Flow capacity, supply/demand, and cost are normalized under explicit runtime units. Commodity balance is checked in the generic `V` domain; duplicate commodity IDs, self-loop conservation mistakes, negative capacity bounds, and cross-`MetaModel` reuse are rejected.
- `FlowContext` and route compilation registration are transactional. A failed partial registration restores model bindings, aggregate indices, variables, and lifecycle state so the same context can be repaired and registered against a fresh model.
- Parallel arcs, ordered arc-ID route signatures, customer-side `RequireArc`, Phase I/II transitions, dual/proof gates, route revalidation, six terminal classes, and honest lower/upper bounds have regression coverage.
- f64 and BigDecimal pricing graphs bind the instance, branch mask, dual snapshot, graph contents, and reduced costs with integrity fingerprints. Pricing rejects stale or mutated public data before label search; the reusable f64 graph entry is crate-private, and BigDecimal additionally validates every arc origin.
- Exact pricing completion is required before an LP objective becomes a certified node bound. Interrupted pricing retains only an inherited valid bound, and unreliable dual/Farkas evidence cannot drive pricing or exact pruning.
- The small-instance full-route master and exhaustive ESPPRC oracles agree with Branch-and-Price. The integral-flow oracle completely enumerates shared capacity, node conservation, and integral flows for its bounded fixtures, including the MRP-shaped inventory/production optimum.
- Vehicle-assignment and type-local arc branches constrain both existing columns and subsequent pricing. The two-vehicle-type fixture covers both sides of each branch, while cancellation, iteration exhaustion, solver failure, trace failure, and solution-enricher failure remain structured outcomes.
- Serial combinatorial wrappers consume a pre-existing or in-flight cancellation in both synchronous and asynchronous report paths and do not start a later fallback solver.

## Extension points

- `ArcFeasibilityPolicy` filters static arcs consistently in initial routes, pricing graphs, and validation.
- `RouteCostPolicy`, `DistanceCalculator`, `TravelTimeCalculator`, and `ArcCostCalculator` define the cost/time contract.
- `LabelDominancePolicy` and `PricingColumnSelector` customize ESPPRC behavior.
- `RouteCompilationExtension` adds model and extraction behavior through the standard lifecycle.
- `TraceListener` observes node-level Branch-and-Price trace snapshots.
- `CancellationToken`, deadlines, and `BranchAndPriceConfig` control interruption and resource limits.

## Numeric and serialization boundaries

Domain quantities remain generic over `V`; flow units are normalized at graph construction and commodity balance totals are validated in the `V` domain before any solver conversion. The ordinary solver master, duals, reduced costs, bounds, and trace use `f64`. `NetworkSchedulingSolverValueAdapter<V>` is the conversion boundary for that ordinary solver path. With `big-decimal`, the explicit-arc exact pricer uses independent BigDecimal dual and label types and does not cross that f64 boundary; both numeric paths are tested.

The optional `serde` feature covers serializable IDs, graph payloads, branch metadata, dual snapshots, and diagnostics. Runtime `Quantity`/`Unit` models are deliberately not derived for serde because the workspace quantities crate does not yet define a lossless runtime-unit serialization contract.

## Current boundaries

- Ordinary route generation, validation, master registration, duals, bounds, and external-solver adapters use `f64`. Arbitrary-precision ESPPRC is available only through the explicit-base-arc BigDecimal entry point; implicit complete graphs are not presented as exact arithmetic.
- Demo17's original 100-customer case is a time/node-budget smoke and does not claim optimality. Strict proof uses the separate hand-checkable 100-customer fixture. The SCIP 25-customer gate requires a legal terminal state and independent route validation.
- The MRP oracle is a complete bounded-fixture enumerator, not a production-scale solver. Two-vehicle-type branch tests prove branch-mask behavior, not large-instance performance.
- Gurobi and SCIP solves are external-environment gates. A missing runtime or license is an explicit failed/unsupported native gate and is never counted as ordinary offline coverage.

## Validation

```powershell
pwsh -NoProfile -Command "cargo test -p ospf-rust-framework-network-scheduling --no-default-features"
pwsh -NoProfile -Command "cargo test -p ospf-rust-framework-network-scheduling --features serde"
pwsh -NoProfile -Command "cargo test -p ospf-rust-framework-network-scheduling --features big-decimal"
pwsh -NoProfile -Command "cargo clippy -p ospf-rust-framework-network-scheduling --all-targets --no-default-features --no-deps -- -D warnings"
pwsh -NoProfile -Command "cargo fmt -p ospf-rust-framework-network-scheduling -- --check"
```

Demo5's library tests and its integration targets are separate gates. The library tests are the commands that execute the direct-MIP, 100-customer smoke, and strict-proof bodies:

```powershell
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored"
```

The `demo5_gurobi_bp` and `demo5_scip_bp` integration targets additionally cover their target-level fixture/parser harness; they do not replace these library commands. Missing native runtime or license is a failed native gate, not a passing test.

The full clippy command without `--no-deps` is also useful, but currently reports pre-existing warnings in `ospf-rust-base` and `ospf-rust-math`. Gurobi/SCIP execution remains feature-gated and requires the corresponding local runtime/license or bundled build.

The ordinary route-generation and solver-facing model still use `f64` because that is the `MetaModel` and external-solver contract. For arbitrary-precision path pricing, use `ExactBigDecimalRouteGraphBuilder` and `ExactBigDecimalEspprcPricer` with explicit base arcs; the exact entry point does not present the default Euclidean policy as exact arithmetic.

## Related documentation

- [Chinese README](README_ch.md)
- [Unified solve contract](../docs/solve-contract.md)
- [Solver native validation matrix](../docs/solver-native-matrix.md)
- [Solver source traceability](../docs/solver-traceability.md)
- [Workspace README](../README.md)
