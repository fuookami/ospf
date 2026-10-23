# Finite-Candidate Activities

English | [简体中文](README_ch.md)

This module implements finite-candidate scheduling and event inventory from Kotlin
snapshot `3f15899`, independently of the older bunch-generation loop. Preparation,
transactional registration, native validation and single-MILP orchestration share
one context and production extraction path.

## Preparation

`ActivityPreparation::new(problem, adapter, additional_cost)` takes ownership of
the input. Read-only accessors expose normalized inputs, every complete choice
in stable key order, and explicit cancellation costs. No `MetaModel` is created
or modified. The optional `ActivityCostPolicy` is evaluated once per complete
choice during preparation, and its result is cached; callers must keep the
policy read-only. Arbitrary external policy side effects cannot be rolled back.

Finite start sets are the business domain, not a search budget. Nothing is
sampled, truncated or silently clipped. Mandatory activities without candidates
are valid inputs producing an infeasible model. Declared candidates outside the
horizon are input errors. Milestones may occur at either horizon endpoint.
Running facts retain their complete expected end beyond the horizon and must
explicitly declare only remaining reservations.

Mode and extra costs may be negative. Cancellation and baseline-change costs
must be non-negative. Cancellation does not incur additional baseline fees.
Frozen baselines must reference an existing choice; non-frozen baselines may
lie outside the new domain. Resources, modes, materials, fixed events, facts,
baselines and extension rows are checked even when there are no candidates.
Opening-stock violations are not input errors: opening precedes horizon-start
events and must yield model infeasibility, not be repaired by receipts.

## Numeric Boundary

Public inputs retain `Quantity<V, Unit>`. Preparation normalizes in `V` and
retains native values and finite solver cost coefficients. Checked boundaries
reject affine units before arithmetic, incompatible units, non-finite values,
nonzero underflow, invalid zero outputs and date arithmetic overflow. Linear
temperature differences remain supported. Native values are not reconstructed
from rounded solver coefficients.

Custom `SchedulingSolverValueAdapter` implementations must provide
`try_into_value` and `try_from_value`. Activity code never invokes legacy
infallible methods. Returning `Err` is the failure contract; arbitrary user
panics are not caught. Non-f64 values, non-identity adapters and unit conversions
set `solver_model_only()`. This conservative flag does not itself produce a
proof. Finite-candidate optimality never implies continuous-time or older bunch
column-generation global optimality.

`serde` covers choice keys and semantic enums, not generic serialization of
runtime quantities or policy objects.

## Embedded Lifecycle

1. Construct `ActivitySchedulingContext::new` with an adapter, optional cached cost
   policy and `ActivitySchedulingConfiguration`. Every fallible row and cost
   conversion completes before model mutation.
2. Call `register` or `register_with` on a `MetaModel<f64>`. Failures roll back model
   and staged aggregation and allow retry. Successful contexts belong to one model
   and cannot register twice. External closure side effects are not rolled back.
3. Append constraints/objectives using source-token variables and `cost_expression`.
   Activity cost appends one objective; it never clears external objectives.
   Set `minimize_cost=false` to supply your own objective.
4. Call `compile` after all extensions. `ActivityCompiledModel` privately retains
   the complete actual matrix, source/context/model identity, objective offset and
   VariableId-based source-token-to-solver-column mapping.
5. Use `compiled.solve(&solver, options)` and pass its private-construction
   `ActivitySolveExecution` to `extract_report`. Each compilation permits one solve
   attempt; retry requires a fresh compilation. Even equal matrices cannot replay
   another compilation's execution. `extract_solution` may audit a raw report for
   a feasible payload, but cannot promote any proof. Source
   changes invalidate extraction. Full-vector bounds, expanded rows, auxiliary
   integrality, dimensions, finite values and objectives are audited at this entry,
   not only in the application. Never assign the whole compiled vector to source
   tokens: deferred fallback can add columns.

`validate_activity_selection` and `ActivityPreparation::validate` independently
check choices, cancellation, freezing, four endpoint dependencies, half-open joint
resource phases, calendars and extension rows in native `V`. Inventory replay
settles simultaneous events atomically, checks opening before receipts, includes
both horizon endpoints and does not infer historical flows from execution facts.
Different materials do not compensate each other. Native comparisons do not use
the backend's `1e-7` tolerance to forgive small business violations.

## Single MILP And Evidence Scope

`application::service::ActivitySchedulingApplicationService::solve` accepts a fresh
context factory and `FrameworkSolveOptions`; `solve_with` adds a transactional
extension hook. Sync and `async` implementations invoke the existing
`ColumnGenerationSolver::solve_milp_report_with_options` exactly once, never LP or
pricing. Normal no-incumbent, limit, cancellation and infeasible outcomes remain
`GanttResult<SolveReport<ActivitySchedule<V>>>`; invalid inputs, incumbents and
technical failures are errors. A zero-column `Some(solution)` differs from `None`
and preserves fixed facts and inventory.

Backend objective, bound and gap retain the compiled `c^T x` convention.
`objective_offset()` separately records the direction/weight-normalized declared
constant. `ActivitySchedule::cost` recomputes native business costs, excluding
downstream objectives and sunk execution costs. Do not add the offset only to the
objective while leaving bound/gap unchanged.

Diagnostic extension `activity.solverModelReport.debug` retains the original full
`SolveReport<f64>` as a Debug string for audit, not a machine-readable SourceModel
certificate. `activity.declaredObjective`, `activity.objectiveOffset` and `activity.proofScope` declare the complete objective, offset
and finite candidate scope. For non-f64, unit-converted or non-identity/lossy
models, only native-revalidated feasible incumbents transfer. Top-level proof,
bounds, gaps, iteration evidence, model mapping and diagnostic evidence are cleared;
without an incumbent the SourceModel status is `Unknown`/`None`. Original SolverModel
evidence stays audit-only. No finite candidate result proves continuous-time or
older bunch column-generation global optimality.

See [migration notes](MIGRATION.md) for compatibility and verification boundaries.
