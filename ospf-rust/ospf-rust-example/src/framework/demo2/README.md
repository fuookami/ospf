# Demo2 Aviation Loading

[简体中文](README_ch.md) | English

`FullLoadApplication`, `PredistributionApplication` and `WeightRecommendationApplication`
delegate to `Demo2ApplicationService`. They share one complete domain registration rather
than maintaining separate master/subproblem business models. `LoadingOrderApplication`
remains an independent cargo-list export.

## Model And Extension Boundary

- `domain/shared/loading_context.rs` adapts explicit DTO data to BPP1D (B737/B757) or
  BPP2D (B767/B747), registers the shared sparse assignment/whole-position occupancy/layout
  model, and adds all aviation pipelines to the same `MetaModel`.
- `model_registration.rs` exposes read-only projections for existing aviation pipelines.
  Illegal pairs share a fixed-zero column; no second dense assignment model is created.
- Position weights used by payload, cumulative load, envelope, target moment, lateral
  balance and optional wide-body CLIM include actual cargo and additional resources.
- `recommendation.rs` maximizes complete payload and registers pairwise hard equalization
  of integer recommended weights, relaxed by actual loading as in the fixed Kotlin source.
  Recommended and actual loading are mutually exclusive on stowage/adjustment positions.
- Add domain rules in a context/pipeline before `RegisteredDiscreteLoading::new`, not in
  the solver callback. All strategies then see the same rows, bounds and objectives.

The request is an explicit **f64 adapter boundary**, not a generic physical-quantity API.
Weights use kg; arms use m; moments/CLIM use kg*m. Geometry has an explicit meter,
centimeter or inch input unit and is converted once to `Quantity<f64, Meter>`;
the BPP problem uses `Quantity<f64, Kilogram>`. No cross-value-domain proof promotion occurs.

## Input Contract

Start with `Demo2Request::sample()` and populate `request.loading`:

- `positions` must exactly cover position IDs and provide a physical `space_id`, positive
  explicit bounds, `loading_order` and `loading_order_scope`. Narrow-body conflicts use
  longitudinal intervals; wide-body conflicts use rectangles. Touching is not overlap.
  Neither area nor list order is used to invent geometry or layouts.
- `cargos` optionally overrides `required`, allowed position IDs, space and strict cargo
  order. An empty allowed set means no legal assignment. Without an override,
  predistribution requires each cargo; the other modes permit omission. Priority alone
  does not invent mandatory cargo. Other explicitly registered business rules still apply.
- `loaded_items` fixes cargo by identity. Unknown cargo, duplicate fixed ownership and
  unknown allowed positions are errors. Disabled positions cannot accept new loads;
  valid fixed cargo remains fixed. Stowage/adjustment flags control new actual assignments.
- `fixed_occupied`, `empty_forbidden`, extra conflicts and exactly-one layout groups are
  explicit. Empty-forbidden constrains whole occupancy in **all three modes**, not count.
  Empty reservations are legal according to the shared BPP contract.
  Fixed occupancy is not evidence of actual cargo: actual-loaded indicators derive only
  from assigned cargo, so an empty fixed reservation can still receive recommended weight.
- Named adjacency in `loading.adjacent_positions` takes precedence over the legacy index
  field. Requests are normalized by explicit ordering before aviation registration and
  extraction; permuting the input list does not change a named contract.
  Priority order and source-early position sets stay within explicit ordering scopes;
  renaming a scope cannot create an aircraft-wide loading precedence.
- `predicate_weight` explicitly enables a bounded continuous resource. In recommendation
  mode, `recommended_weight` enables a bounded **integer** kg resource. Neither is inferred
  from the legacy `predicate_load_weight_min` hint. Capacity and occupancy links include
  both; a positive resource can never hide in an unoccupied position.
- `max_clim` is an optional explicit wide-body hard bound, independent of envelope and
  target-moment bounds. Supplying it for a narrow-body request is rejected.

## Strategies And Evidence

1. Ordinary MILP: solve the complete snapshot and validate the entire incumbent against
   source bounds, integrality, all aviation rows, layouts and geometry.
2. Safe Benders: partition that same expanded source. All integer decisions, including
   recommended weights, remain in the master. Certificate/representation limitations
   remain explicit; `benders_fallback_to_milp` permits a fresh equivalent full-MILP factory
   with independent model/context identity and only the remaining total budget. Input,
   callback and cancellation errors never become fallback attempts.
3. Independent partition CG: set `partition_column_generation` and provide exhaustive
   explicit `partitions`. It cannot be combined with preferred Benders. Additional
   resources have explicit position-derived ownership and a separate completion stage
   that restores integrality while freezing assignment/occupancy. Complete source rows
   remain in the projection. This is root CG plus an integer restricted-pool heuristic,
   **not branch-and-price**: even a feasible source can have an infeasible final pool.

### Complete-Source Finishing and Certification

`solve_policy.source_milp_finishing` explicitly controls complete-source solving after Benders/CG:

| Policy | Behavior |
| --- | --- |
| `SourceMilpFinishingPolicy::IfUnproven` (default) | Finish on a **fresh equivalent complete-source MILP** within the remaining budget unless a reliable complete-source optimality, infeasibility or unboundedness terminal proof is already available. |
| `SourceMilpFinishingPolicy::Disabled` | Append no source solve; preserve the accelerator result, termination and algorithm reports. Still validate complete-source identity, vector feasibility and objective, without promoting restricted-pool proofs. |

With finishing enabled, `loading.acceleration_time_ratio` must be strictly between zero and one
and defaults to 0.5: acceleration receives at most half the remaining time, reserving the rest
for source certification. With finishing disabled, this ratio is ignored and no time is reserved;
acceleration may use the entire remaining budget (still capped by CG's own
`column_generation.time_limit`). Factory, validation and solve time share one total deadline.
Direct MILP is unaffected by the finishing policy.

For pure Benders/CG comparisons, use the following; disabling finishing guarantees neither a
feasible plan nor a global optimality proof:

```rust
request.solve_policy.source_milp_finishing = SourceMilpFinishingPolicy::Disabled;
request.solve_policy.benders_fallback_to_milp = false; // 同时关闭 Benders 内部回退 / Also disable internal Benders fallback.
```

Only fully validated feasible values become a warm start through checked equivalent column
bindings; old tokens, duals and restricted-pool proofs never cross sessions. An empty, limited
or worse finishing result cannot discard a better validated incumbent. The actual finishing
report stays in `algorithm_reports`; only reliable complete-source evidence can make the final
status Optimal. Infeasibility or a worse claimed optimum contradicting the incumbent is rejected.

With finishing enabled, an infeasible restricted pool, missing optimal LP/dual evidence or
unsupported decomposition capabilities can lead to full-source finishing. When disabled,
existing plans still undergo full-source validation; no-plan results retain their actual unknown/limit
terminals, and unhandled backend/capability errors propagate without implicit source recovery.
Invalid supplied certificates, wrong mappings
and input/modeling/callback contract errors still propagate without weakening gates.
The legacy `benders_fallback_to_milp` switch independently controls early fallback inside acceleration:
even with finishing disabled, setting it to true still permits internal Benders-to-MILP fallback.
Disable both to prohibit all ordinary MILP solves after Benders. Cancellation, interruption or an exhausted total
deadline cannot start another solve; an accelerator's local time cap is not the total deadline.
Recoverable finishing-backend failures retain the plan and an explicit failure terminal, not
an invented optimality proof.

Actual finishing uses `benders_source_milp` or `partition_column_generation_source_milp` in
`framework_path`. An internal fallback that already proves the full source optimal is not
repeated; completed infeasible/unbounded source proofs are likewise retained unchanged. This
closes source solving/certification when enabled, without calling root CG branch-and-price.

`solve_report` preserves the actual source problem status, termination, solution presence,
proof and audited model identity. `algorithm_reports` separately retain actual restricted,
refinement, master/subproblem and fallback reports. Source Benders/CG solutions remain
Feasible unless a full-source report itself supplies verified optimality. `status` is
Optimal only when `solve_report.is_optimal()`; source Infeasible/Unbounded and limit/cancel
terminals are not collapsed into a generic NoSolution string. A restricted-pool infeasibility
is never promoted to source infeasibility.

`objective` is the normalized **minimizing complete-source objective**, including its
constant once, not just cargo weight or a restricted-master objective. Legacy quality
overrides are observational diagnostics from actual available bounds/cut counts; they
cannot manufacture trajectories, discard incumbents or override safe fallback policy.
Unsupported legacy stall switches are reported as inactive.

## Callbacks And Results

The three applications keep `execute(request)` and add `execute_with_handle` and
`execute_with_options(Demo2ExecutionOptions)`. Options control the shared cancellation
handle, phase heartbeat, final callback and rendering. Callbacks execute outside backend
recovery; their errors propagate and completion fires once, after final extraction.

Assignments, selected layouts, occupied positions, additional kg weights, render and KPIs
come from the **final retained registration**, including fallback or a retained limited
incumbent. They never come from an intermediate master or the rejected fallback candidate.
`with_render=false` disables rendering without removing KPI or solver evidence.

## Verification

With a working Gurobi 10 runtime/license:

```powershell
cargo +nightly-2026-08-01 test -p ospf-rust-example --features backend-gurobi framework::demo2 -- --test-threads=1
cargo +nightly-2026-08-01 run -p ospf-rust-example --features backend-gurobi -- framework:demo2
```

`migration_tests.rs` covers the three modes/strategies, explicit wide-body geometry,
sparse/fixed/disabled assignments, source target/CLIM limits, integer recommendation and
hard equalization, request permutation, fresh fallback, independent CG, truthful reports,
limited incumbents, callback failures and final render/KPI extraction. The root-CG tests
explicitly distinguish source feasibility from restricted-pool proof.
