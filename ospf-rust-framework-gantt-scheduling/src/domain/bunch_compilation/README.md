# Bunch Compilation

:us: English | :cn: [简体中文](README_ch.md)

This directory contains the master-problem side of task bunch compilation. It maps Kotlin `gantt-scheduling-domain-bunch-compilation-context`.

## Responsibilities

- Register task bunch decision variables and aggregation expressions into `MetaModel`.
- Maintain iterative bunch columns for column generation workflows.
- Extract task and bunch scheduling solutions from solver results.
- Provide slot-based compilation and capacity pre-solving helpers.
- Expose shadow-price pipelines used by pricing subproblems.

## Modules

- `model.rs`: bunch compilation, bunch aggregation, entries, solution, and solution summary.
- `context.rs`: basic and iterative bunch compilation contexts.
- `iterative.rs`: iterative bunch compilation state and column operations.
- `master.rs`: opt-in standard task coverage, executor selection, and owned objective component.
- `slot_based.rs`: slot-based bunch compilation context and capacity pre-solver.
- `service.rs`: solution analysis and service helpers.

## Public API

- `BunchCompilation`
- `BunchAggregation`
- `BunchEntry`
- `BunchSolution`
- `BunchSolutionSummary`
- `IterativeBunchCompilation`
- `BunchMasterConfiguration` and `BunchExecutorSelection`
- `BasicBunchCompilationContext`
- `IterativeBunchCompilationContext`
- `BunchShadowPricePipeline`
- `TaskShadowPriceKey`
- `SlotBasedBunchCompilationContext`
- `BasicSlotBasedBunchCompilationContext`
- `SlotBasedCapacityPreSolver`
- `StaticSlotBasedCapacityPreSolver`

## Extension Points

Add master-problem behavior through bunch compilation contexts, iterative bunch-column operations, slot-based capacity pre-solvers, and shadow-price pipelines. Keep pricing rules in `bunch_generation` and task-level variable semantics in `task_compilation`.

## Lifecycle and Data Flow

Compilation contexts register initial bunch columns and aggregation expressions into `MetaModel`, iterative contexts add or refresh columns during column generation, shadow-price pipelines expose dual values to pricing, and solution analyzers extract task and bunch schedules from final solver values.

`BunchEntry::id` is the stable identity of the complete business candidate, not a hash of
executor/task coverage. Distinct recoveries covering the same tasks need distinct IDs.
The caller must bind each ID to its immutable full payload. Equal IDs are idempotent only
when executor, ordered task indices, slot, and normalized cost agree; conflicting entries
fail the entire batch. `index` is a separate historical model-column index: retired indices
cannot be reused, and re-adding an identical retired candidate allocates a new variable.

Registration is idempotent only for the owning model and unchanged compilation definition.
Cross-model use is rejected. Registration, column addition, and model-aware retirement are
transactional for both context state and `MetaModel`. `remove_columns(indices, model)` and
`apply_lifecycle(lifecycle, model)` now return `GanttResult<()>`; retirement fixes old tokens
at zero, clears their objective terms, rebuilds symbols, and refreshes the owned master.
`next_column_index()` includes retired history. Slot batches commit together, not slot by slot.

Before registration, call `context.compilation.configure_master(BunchMasterConfiguration {
cancellation_costs, executor_selection })` to opt into the standard master. Each task is
covered exactly once by a selected bunch or cancellation. `None` cancellation cost forbids
cancellation; lack of coverage remains model infeasibility. Executor selection explicitly
uses `AtMostOne` or `ExactlyOne`, including enabled leisure variables. The component owns
its constraint group and named minimum-cost sub-objective, and refreshes them on every real
column mutation without replacing unrelated downstream constraints or objective terms.
Its independent binary-enumeration tests inspect the actual compiled backend matrix.

Existing `service::limits::*::from_compilation` pipelines remain **static registration
snapshots**, not live row-update subscriptions. Their column lookup uses stable mappings
even after retirement. Dynamic custom rows still need an explicit downstream lifecycle hook;
retiring a column fixes its token at zero but does not invent coefficients for newly added
columns in arbitrary external rules. The standard master does not certify pricing completion
or integer optimality, and does not itself complete the Demo4 application integration.

## Production Extraction and Solve Direction

`extract_solution_checked(model, values)` / `analyze_solution_checked(model, values)` validate
the complete current backend matrix, including downstream rows, bounds, finiteness,
dimensions and integrality. They check managed variable mappings and reject nonzero retired
columns. Cancellations come from actual `y` values, never from inferred missing coverage.
`analyze_solution_with_lifecycle_checked` also rejects unselected fixed columns and selected
hidden/retired columns rather than repairing the result. Legacy model-free extractors are
unchecked previews, not production validation.

`validate_bunch_incumbent` additionally checks the reported objective against actual `c * x`,
rejecting nonfinite coefficients and arithmetic overflow. LP callers must explicitly pass a
relaxed model. `BunchBranchAndPriceAlgorithm::solve_milp_report` preserves normal infeasible,
limited-without-incumbent and cancelled reports while rejecting technical failures and invalid
incumbents. The legacy `run` entry still requires an incumbent; it is not a new unified
standalone service.

The bunch algorithm resets `Iteration` for the registered model's actual objective direction.
Minimization accepts zero and negative improvements; a zero initial objective does not imply
pricing convergence. Final MILP errors are no longer swallowed by an earlier incumbent.
Retaining an earlier incumbent extends and revalidates its vector against the current model
and lifecycle, then builds a paired report without its old proof. Read it through
`incumbent_report()`; old-model reports must not be mixed with new solution dimensions.
Only production-validated incumbents update directional primal bounds; LP history alone never
creates a dual bound over the complete pricing domain.

## Verification

Use `cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --no-default-features -- --list`
and then the same command without `-- --list`. Verify `serde`, `async`, `big-decimal`, and
`big-rational` separately; test enumeration alone is not execution evidence.

## Related Directories

- [`../bunch_generation`](../bunch_generation/README.md)
- [`../task`](../task/README.md)
- [`../../application/algorithm`](../../application/algorithm/README.md)
