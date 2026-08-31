# OSPF Rust Framework Gantt Scheduling

🇺🇸 English | 🇨🇳 [简体中文](README_ch.md)

This crate is the Rust migration target for `ospf-kotlin-framework-gantt-scheduling`.
The current state contains the Gantt domain framework foundations, task and
bunch compilation, resource/produce/capacity contexts, pricing graph/label
search, iterative column lifecycle, isolated branch-and-price tree search, and a
small final-MILP branch-and-price flow covered by tests.

## Public API

- Task domain: `domain::task::{TaskTrait, ExecutorTrait, AssignmentPolicyTrait,
  TaskPlanTrait, TaskStepGraph, TaskStepTrait, BasicTaskStep, StepRelation,
  Cost, BunchCostPolicy, CostBreakdown, DefaultBunchCostPolicy,
  SolverValueAdapter, F64SolverValueAdapter}`.
- Task compilation: `domain::task_compilation::{BasicTaskCompilationContext,
  IterativeTaskCompilationContext, Switch, SwitchCostMinimization,
  SwitchTimeMinimization}`.
- Capacity scheduling: `domain::capacity_scheduling::{CapacityCompilation,
  CapacityOrderCompilation, CapacityColumn, CapacityColumnAggregation,
  CapacitySchedulingSolution}`.
- Bunch compilation: `domain::bunch_compilation::{BasicBunchCompilationContext,
  IterativeBunchCompilationContext, BasicSlotBasedBunchCompilationContext,
  SlotBasedBunchCompilationContext, SlotBasedCapacityPreSolver, BunchEntry,
  BunchSolution}`.
- Bunch generation: `domain::bunch_generation::{SlotBasedBunchGenerator,
  BunchFeasibilityPolicy, BunchTaskCandidate, CapacityIntermediateValues}`.
- Application helpers: `application::service::create_bunch_branch_and_price` and
  `application::service::search_bunch_branch_and_price_with_fresh_model`.
- Dynamic lifecycle: `domain::common::GanttDynamicModelLifecycle`, re-exported from
  `ospf_rust_framework::model::DynamicModelLifecycle`.
- Calendar policies: `infrastructure::{CalendarPolicy, CompositeCalendarPolicy}`.

Use `SolverValueAdapter` for generic solver-value conversion and
`F64SolverValueAdapter` for the f64 solver boundary.

## Extension Points

- `domain::bunch_generation::BunchFeasibilityPolicy` injects task, resource,
  produce, capacity, and business feasibility rules into slot-based bunch generation.
- `domain::task::BunchCostPolicy` injects task, executor, connection, capacity,
  and soft-constraint cost formulas. `DefaultBunchGenerationPolicy::with_cost_policy`
  wires those formulas into reduced-cost calculation without changing the application
  branch-and-price flow.
- `infrastructure::CalendarPolicy` wraps `WorkingCalendar` with injectable complex
  shift rules, extra unavailable ranges, connection windows, and break rules.
- `domain::task::TaskStepGraph` models multi-step task dependencies as a
  validated DAG with start-step, forward-vector, and backward-vector semantics.
- `domain::task_compilation::Switch` supports the Kotlin-aligned static-time
  switch path and the optional `TaskTime` dynamic switch path. Its switch-cost
  and switch-time objective families are exposed as standard pipelines, including
  threshold slack support for switch-time minimization.
- `domain::capacity_scheduling::CapacityCompilation` and
  `CapacityOrderCompilation` register per-slot action upper bounds and extract
  unordered or ordered capacity solutions; `CapacityColumnAggregation` tracks
  iteration columns, deduplicates additions, and records removed columns.
- `domain::bunch_compilation::SlotBasedBunchCompilationContext` adds capacity
  pre-solving, slot-constraint lookup, slot-wise column addition, and slot-wise
  bunch queries on top of the iterative bunch compilation context.
- `domain::common::ConstraintIndexMap` maps business constraint keys to dual
  solution indices for stable shadow-price extraction.
- `domain::common::GanttDynamicModelLifecycle` reuses the shared framework dynamic
  lifecycle for warm start, `setSolution`, `flush`, column hiding/fixing/removal,
  and column range restoration. It uses `GanttModelStateFacade` as a compatibility
  alias for shared selectable-column state.
- `application::algorithm::BranchAndPriceTreeSearch` provides a model-agnostic
  multi-node branch-and-price search skeleton with `StrongBranchingStrategy`,
  `BranchCutCallback`, and `BranchNodeCallback` extension points.
- `application::algorithm::BunchBranchAndPriceAlgorithm::solve_branch_node`
  adapts branch decisions into column-state fallback rules and reuses the
  current single-node branch-and-price flow. It snapshots and restores
  application state around each node solve, so sibling nodes do not share
  shadow-price, fixed/hidden column, incumbent, or iteration state.
- `application::algorithm::BunchBranchAndPriceAlgorithm::solve_branch_node_with_fresh_model`
  adds a stronger isolated-node entry for tree search callers: it snapshots the
  compilation context and runs each node against a caller-built fresh `MetaModel`.
- `application::service::search_bunch_branch_and_price_with_fresh_model`
  wires `BranchAndPriceTreeSearch` to the isolated-node entry as the preferred
  public application helper for multi-node branch-and-price.
- `application::service::search_bunch_branch_and_price_with_hooks` adds the same
  isolated-node search with strong-branching, cut, and node-trace hooks.

## Branch-And-Price Flow

The tested application flow is:

1. Build a fresh `MetaModel<f64>` per branch node.
2. Register the bunch compilation context.
3. Solve the initial MILP.
4. Solve the RMP LP and extract shadow prices through the context.
5. Generate bunches through `BunchCGPolicy` and register them with `add_columns`.
6. Solve the final MILP and extract `BunchSolution` through the shared context.
7. Restore application and context state before moving to the next tree node.

## Current Dynamic Model Boundary

`GanttDynamicModelLifecycle` is now a Gantt compatibility alias over the shared
framework dynamic model lifecycle. It records solver solutions, derives warm-start
columns, flushes transient state, restores column ranges, and keeps removed
columns durable. Core `MetaModel` exposes solution, flush, and variable-range
interfaces; the lifecycle can write cached solutions back to `MetaModel` before
MILP solves so adapters can consume token results as native warm starts. For
multi-node tree search, prefer `solve_branch_node_with_fresh_model`, which
restores application state, the dynamic lifecycle, and the compilation context
while discarding the node-local `MetaModel` after solve.

## Current Boundaries

- Ordinary MILP, RMP LP, and final MILP share the context and iterative
  compilation entry points for registration, column addition, shadow-price
  extraction, and solution extraction. Solver conversion remains in the
  application layer.
- Multi-step task graphs are exposed as a domain extension model. Existing
  single-step task compilation remains unchanged.
- Slot-based bunch compilation currently uses the shared `MetaModel<f64>` solver
  boundary and a pluggable capacity pre-solver trait; downstream capacity
  schedulers can provide richer intermediate values without changing the
  application solver flow.
- Capacity scheduling and task switch registration now use shared core
  variable-range, solution, function-symbol, and objective-pipeline interfaces
  instead of Gantt-local lifecycle shims.
- Native solver warm start remains an adapter capability. The shared lifecycle
  writes cached solver-order solutions into `MetaModel`; Gurobi adapters already
  map token results to native `Start`, and other adapters can reuse the same
  solution state.
- Concurrent tree execution and solver-native node callbacks are still outside
  this crate, but the branch-and-price search exposes strong-branching, cut, and
  node callback traits for downstream integration.
- Complex shift calendars and custom cost formulas are supported through standard
  policy extension points and covered by minimal tests.

See [gantt.md](gantt.md) for the detailed migration target, checklist, and acceptance criteria.
