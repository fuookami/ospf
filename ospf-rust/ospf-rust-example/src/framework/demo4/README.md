# Framework Demo4 - Airline Recovery

English | [简体中文](README_ch.md)

## Migration Status

`RecoveryApplicationService` reuses the existing Gantt loop for initial columns, RMPs,
current-model duals, shared-path pricing, column insertion, final MILP and production extraction.
Hard passenger coverage, seat capacity and cross-aircraft transfers participate in the same
master/pricing lifecycle. Bounded passenger cancellation/cabin/flight changes, terminal fleet balance,
automatic initial columns and lifecycle-wide task locks use this same context and row provider.
Soft link-split and per-aircraft idle costs use this registration, pricing and production-extraction path too.
Actual-assignment restrictions now support relation black/whitelists, general conditions and hard/soft
severity, with soft usage and costs participating in the same master.
Finite airport flow controls now use actual internal events and complete stays, with capacity
and optional overage costs participating in this lifecycle too.
The specified-snapshot responsibilities are migrated within the finite recovery-activity domain below,
with nonempty offline, actual Gurobi application and affected regression gates exercised.
This does not establish acceptance of the entire Network / Gantt migration plan or global integer optimality.
`app::run` shows the domain
with `demo4-offline` and additionally runs the native Gantt application with `demo4-gurobi-cg`.
The legacy ID-only graph/generator stack and fixed-zero branch-and-price placeholder are removed;
callers use complete recovery domains, shared paths and `RecoveryApplicationService`.

## Recovery Records

- `FlightTaskRecovery` retains the original `FlightTaskImpl`, actual aircraft, recovered
  route and actual start/end times. Creating a record never rewrites the original task.
- `FlightTaskBunch::new(id, aircraft, activities)` now returns `Result`. It rejects duplicate
  original tasks, conflicting aircraft configurations, disconnected routes and overlapping
  activities. It replaces the old ID-list constructor and mutable endpoint fields.
- `task_ids()`, `dep()`, `arr()` and `time()` are derived views. Empty bunches have no fabricated
  endpoints or wall-clock-dependent timestamps.
- `arrived_when` and `departed_when` inspect every activity, including internal airports,
  using recovered event times and inclusive time bounds.
- `flight_hour()` and `flight_cycle()` consume the same actual records and exclude ground
  maintenance/AOG activities.

## Rules And Cost Policies

`FeasibilityJudger::judge_bunch` checks aircraft availability, the actual predecessor,
connection times, original task change restrictions, hard-adjacency or penalized-split
links and remaining flight allowances. An explicit `ConnectionTimeIgnoring` link overrides
the aircraft connection time, but not other declared link minima. An executed predecessor
supplies the effective starting location. Whole flights departing before a flight-allowance
expiry consume that period's allowance; crossing flights are not truncated.
Flight/virtual-flight reassignment checks the original task's passenger/cargo capacity category
against the actual aircraft. Ground activities and undeclared categories do not invent this requirement.
`RecoveryProblem::with_transition_policy(Arc<dyn RecoveryTransitionPolicy>)` supplies the snapshot's
injectable transition rules and turnaround calculator. `allows` receives the actual aircraft,
actual predecessor and current recovery, including a first activity without a predecessor;
`connection_time` can depend on the task pair and defaults to aircraft/airport turnaround.
The policy must be a deterministic, model-free read-only business definition. Automatic seeds,
shared-path terminal/independent trace checks, pricing, replay and production extraction all use
it; a generation-only callback is insufficient. Negative turnaround or a policy error returns
a structured error; an unmet valid rule rejects a candidate rather than proving global infeasibility.
Connection-time-ignoring links override turnaround only, not custom rules, other hard link minima,
spacetime continuity or flight allowances.
Executed predecessor IDs must be nonempty, unique across aircraft and absent from the pending
task catalog. Independent bunch validation also rejects executing that original task again as a
different recovery version. History determines the actual starting point/stay but is not a new
column and neither incurs a second cost nor consumes remaining flight allowances again.

`judge_passengers` independently checks hard passenger itineraries across selected bunches,
including cross-aircraft transfers and aggregate seat usage by task/class. Missing selected
flights are not silently counted as served. `RecoveryProblem::with_passengers` prepares hard
coverage, task/class aggregate capacity and incompatible recovery-variant transfer rows before
registration. Valid but unsatisfiable itineraries remain normal model infeasibility. Conflict
rows use finite activity variants, not pre-enumerated routes. Production extraction repeats
independent passenger validation; test-side checks cannot replace it.

`with_passenger_recovery(RecoveryPassengerConfiguration)` replaces hard-passenger mode with
explicit per-leg policies: a bounded integer cancellation decision (or hard zero), allowed cabin
changes and finite nonnegative per-person costs. Every leg requires exactly one policy. Original
cabin demand is debited once for cancellation and outbound changes; destination cabins receive
only actual incoming changes. Nonnegative remaining demand prevents over-disposition. Cancellation
propagates through the itinerary. Without flight changes, incompatible actual transfer variants require cancellation of
all remaining passengers on the later leg, without forcing cancellation of the earlier flown leg.
Capacity uses the actual selected aircraft/class, and all auxiliary rows, including zero column
coefficients, participate in current-model signed-dual pricing. `RecoverySchedule::passengers`
retains actual canceled/changed/transported counts and separate penalties. Production extraction
independently checks those counts, capacities and actual connections even if model rows are removed.

`RecoveryPassengerLegPolicy::flight_change_costs` explicitly maps allowed `(flight, cabin)`
destinations to total per-person rebooking costs; empty disables flight changes. Targets must be
other flight-type tasks with the same original OD. Every used original or target recovery must
retain the passenger leg's actual OD; a diverted original flight cannot silently serve the old OD.
Rebooking debits the original cabin and credits the actual target cabin;
the flight penalty is not charged again as a separate cabin change. Adjacent legs involving
flight changes use bounded integer connection flows between compatible actual recovery variants.
Each continuing passenger needs an incoming connection, and outgoing connections cannot exceed
the previous leg's actual passengers. Gates bind both endpoints to selected full recovery records,
not merely task IDs or estimated original times. Bounds come from the passenger amount, not an
arbitrary big M. Partial group splits and counts greater than one are supported. Results retain
flight/cabin changes, incoming connection counts and separate rebooking penalties. Independent
production validation rejects absent/wrong-OD variants, broken flows and capacity violations even
after model rows are removed.

`RecoveryCostPolicy` exposes activity and complete-bunch terminal contributions. The default
`CostCalculator` uses actual flight duration and the actual aircraft rate. Complete cost
aggregation rejects NaN, infinity and overflow. Demo4's existing cost/solver boundary is
`f64`; this does not claim arbitrary-precision or runtime-unit support for airline input.

Executed predecessor IDs must be nonempty, unique across aircraft domains and absent from the pending
task directory; otherwise construction fails before registration. Independent `judge_bunch` also rejects
re-executing the predecessor's original task, even through a different timed recovery variant. History
still determines actual origins and connection/stay intervals, without consuming remaining flight
allowances again or becoming another selectable column.

## Shared Path Generation

`RecoveryBunchGenerationContext` accepts a finite set of `RecoveryActivityCandidate`s and
uses `PathProblem` through `NetworkBunchGenerationContext`. Its state preserves the full
recovery sequence; candidates for the same original task cannot both occur on a path.

The result retains business bunches, complete costs, predecessor traces, diagnostics and
actual search completion. Nonadditive terminal costs are evaluated before sorting. Dominance
is disabled. Output limits only truncate returned candidates, while budgets and interruption
remain visible. A request describes exactly one unslotted aircraft domain.

This domain generator deliberately returns **no no-improvement certificate**. Path exhaustion
is not verified LP provenance, an LP lower bound or global integer optimality.
`RecoveryPricingPolicy` consumes all signed original-row duals normalized by Gantt from the
current verified LP report, including explicit zeros. Raw prices are heuristic only and cannot
raise bounds. Missing/unmapped rows, nonfinite duals and scope mismatches fail.

## Recovery Master Registration

`bunch_compilation::context::BunchCompilationContext::new(RecoveryMasterConfiguration)` owns
original tasks, complete aircraft definitions and cancellation costs. It reuses Gantt's
`BasicBunchCompilationContext` managed coverage, cancellation, executor-selection rows and
cost objective, replacing the empty compilation-list context without a private modeling kernel.

`add_candidates` atomically records complete `RecoveryBunchCandidate`s and adds columns.
Business IDs use aircraft namespaces and length encoding. Reusing an ID with changed cost,
original tasks, recovered times, routes or airport configuration fails. Unknown tasks, rewritten
original definitions and conflicting aircraft configurations fail before model mutation.
Retirement retains immutable business history; re-adding a candidate creates a new variable.

`extract_solution(model, values)` returns `RecoverySchedule` only after Gantt's complete
matrix/integrality production checks, retaining original tasks, actual times and internal
transfer events. Cancellations come from actual cancellation variables. Read-only `master()`
exposes stable variable mappings for downstream rules on the same `MetaModel`. Independent
binary-enumeration tests cover the backend matrix, rollback, hard tasks and empty domains.

`with_resolver(configuration, Arc<RecoveryProblem<_>>)` implements Gantt's iterative context
contract. The immutable directory owns no model registration state; contexts exclusively own
column maps, constraint groups and model identity. Complete candidate IDs encode activity
variants with UTF-8 byte lengths, replaying actual records rather than inferring times from
task IDs. Passenger refresh and Gantt insertion/retirement share a model transaction while
preserving downstream objectives/constraints. Initial incumbents remain improvable and do
not automatically pin aircraft; real model bounds and explicit lifecycle operations still apply.

## Application And Proof Scope

### Link Splits And Aircraft Idle Costs

`Link::split_cost = None` retains mandatory same-aircraft adjacency. `Some(cost)` permits a
penalized split with a finite nonnegative cost and both endpoints in the recovery task set.
Actually preserved links still obey their minimum connection time; soft costs do not excuse
an infeasible actual connection. Each soft link registers
`kept + 0.5 * cancel_from + 0.5 * cancel_to + slack >= 1`, with slack bounded by `0..1`.
Physical loss is zero for a preserved link or two canceled endpoints, one half for a single
canceled endpoint, and one for two executed but split endpoints. This follows the specified
snapshot's `FlightLink`/threshold-slack semantics without copying the contradictory
`slack >= 1` in `FlightLinkLimit`.

`with_aircraft_idle_costs(BTreeMap<aircraft registration, cost>)` configures idle costs;
unknown aircraft, negative costs and nonfinite values fail before registration. Only nonempty
selected activity bunches use an aircraft: no selected bunch and a selected empty bunch both
count as idle. The component owns its objective and constraint group and refreshes inside the
same column transaction. All rows and explicit zero coefficients enter signed-dual pricing.
`RecoverySchedule::operating_costs` retains physical link loss, actual modeled slack, idle states
and separate penalties. Valid time-limited incumbents may have nonminimal link slack; costs use
actual slack rather than minimum physical loss. Production extraction independently checks
these quantities even after related rows are removed. A half-integral grid oracle covers the
one-half slack case; a binary-only oracle would not suffice.

Build `RecoveryProblem::new(configuration, domains, links, cost)`, optionally call
`with_passengers`, then pass its `Arc` and `RecoveryApplicationConfiguration` to
`RecoveryApplicationService::new(solver).solve(...)`. Every call creates a fresh context,
policy and model. The old constant-symbol passenger models, fake fleet/link slack models and
their duplicate aggregation/pipeline entry points are removed; callers use this context and
`with_passenger_recovery`/fleet builders instead. Hard coverage/passenger domains require feasible initial columns: this
entry does not implement Phase I or a nested global solve to repair an infeasible seed.

`with_fleet_balance` configures airport/minor-type terminal minima with optional bounded integer
shortage costs. `with_original_fleet_balance` derives the targets from complete original bunches;
unused aircraft retain their actual initial location (or their executed predecessor's arrival).
Columns contribute actual terminal minus replaced origin counts, including zero for idle bunches.
`RecoverySchedule::fleet_balance` separates physical shortage from actual modeled slack and cost,
allowing nonminimal slack in a valid time-limited incumbent. Unknown references and illegal costs
fail before registration; valid impossible hard targets remain model infeasibility. Fleet and
passenger components own only their own objectives and refresh inside the same column transaction.

### Actual-Assignment Restrictions

The placeholder `Restriction::check()` that always returned `NotMatter` and ID-only
`FeasibilityJudger::judge` are replaced. `RestrictionType` now denotes `Weak`, `ViolableStrong`
and `Strong` severity; `RestrictionRule` denotes the business condition. Relation restrictions
check a directed airport pair against the actual aircraft's black/whitelist and do not apply to
nonflight activities. General conditions cover original task sets, actual departure/arrival events,
full-interval containment on bidirectional routes, aircraft sets and conjunctions. Windows are
inclusive; missing required times do not match. Actual records never fall back to planned routes
or estimated original times. `check_task`, `check_aircraft` and
`check_assignment(RestrictionAssignment)` use the same semantics. Read-only custom predicates
implement `GeneralRestrictionPredicate` without model registration state.

`RecoveryProblem::with_restrictions` validates identities, references, windows, weights, costs
and bounded total costs before model mutation. Unpriced `Strong` restrictions are hard throughout
automatic seed generation, pricing, replay, managed rows and independent extraction. Priced
`Strong`, weak and violable restrictions permit violations. General `enabled_aircraft` and
`disabled_aircraft` are applicability and exemption scopes, conjoined with the hit condition as in
the snapshot, not an additional reversed blacklist. Unpriced weak/violable restrictions report
counts and weights with zero monetary cost rather than guessing a default price.

Soft usage is balanced by an integer count bounded by the task count. The objective uses actual
count times declared weight and cost. Every original row dual, including explicit zero, uses the
same coefficients as the master. `RecoverySchedule::restrictions` retains violating tasks, weights
and separate penalties. Extraction independently checks records, hard violations and integer counts
even after related model rows are removed. Contexts own independent variables/groups, refresh
inside Gantt transactions and preserve downstream objectives/constraints.

### Finite Airport Flow Controls

`with_flow_controls(Vec<FlowControl>)` registers finite airport event/stay rules in the same
master, without a second solver or a general global spacetime framework. Rules explicitly
declare an airport, scene, finite time range, positive `interval`, per-window capacity and
fixed external `current_usage` in every window. External usage is not a selectable task.
Interior windows are half-open and the final window includes the overall end event, preventing
adjoining-window double counts. The final window may be truncated. Independent bunch
`arrived_when`/`departed_when` queries retain their existing inclusive semantics.

`Departure`/`Arrival` inspect every actual activity, not just bunch endpoints.
`DepartureArrival` counts an activity at most once per window. `Stay` follows the specified
snapshot's full-containment semantics: the entire interval from actual predecessor arrival to
current departure (ground-activity end for ground tasks) must fit in the window. Partial overlap
does not imply generic apron occupancy. The first activity consumes the actual executed predecessor;
without a predecessor it uses a departure event. Empty bunches invent neither infinite stays nor
another standalone event for an already-executed predecessor.

`FlowControlCondition` derives domestic/regional/international type from the actual route and
combines actual aircraft minor types with an optional `FlowControlPredicate`. Conditional filtering
applies to flight activities, correcting the snapshot's reversed check that excluded all flights.
Unconditional controls can still explicitly cover ground-activity events. The location-only
`evaluate` and placeholder condition DTO are replaced by actual-record/window APIs.

`over_cost=None` is a hard upper bound. Finite nonnegative costs enable bounded integer overage;
bounds derive from task count and fixed external usage, not arbitrary big M. Valid external usage
already exceeding a hard capacity remains model infeasibility. Unknown references, duplicate rules,
nonpositive intervals, invalid times/costs, inexact integers and nonfinite bounded total costs fail
before registration. All rows and explicit zero coefficients share one provider with complete
signed-dual pricing; refresh shares Gantt transactions and preserves extensions.
`RecoverySchedule::flow_controls` retains actual windows, external/recovery usage, physical excess,
actual modeled slack and penalties. Extraction independently checks full actual events and integer
overage even after model rows are removed. Valid nonminimal time-limited slack uses actual modeled
cost, not the minimum physical excess.

`with_locked_tasks` forbids cancellation and wrong-aircraft execution throughout initial generation,
pricing, candidate replay and production extraction.
`with_departure_locks(Vec<Lock>)` independently requires execution at an exact actual start time,
not a scheduled time, lower bound or implicit aircraft assignment. `Lock` replaces the ID-only DTO
with a rule containing `departure_time`; `locked_departure(task_id)` provides a read-only query.
Time and aircraft locks compose in either builder order and clear independently without restoring
cancellation still forbidden by the other rule. Unknown tasks and duplicate rules/task locks fail
before registration. A valid time with no matching finite-domain variant remains normal no-incumbent,
without inventing activities. Actual-time filtering persists through automatic seeds, shared-path
pricing, replay and independent production extraction.

Set
`RecoveryApplicationConfiguration::initial_generation = Some(RecoveryInitialConfiguration { originals })`
to use the shared path generator for automatic seed columns. Full original records are a preference,
not a source of invented recovered times: the finite activity directory supplies actual records.
Per-aircraft selection maximizes original task-order overlap, then minimizes added activities and
complete cost. This heuristic seed cannot guarantee global hard-passenger feasibility. Search traces,
scope, budget exhaustion and interruption remain in `RecoverySolution::initial_generation`; report
diagnostics also preserve completion and column count when no incumbent exists. No Phase I,
no-improvement certificate or additional global solver is introduced.

The unified result is `GanttResult<SolveReport<RecoverySolution>>`. Normal no-incumbent
outcomes retain termination reasons; finite-pool infeasibility maps to domain `Unknown` with
the original status in `demo4.finiteMasterStatus`. Technical failures and invalid incumbents
are errors. Results retain complete schedules, final `candidate_ids`, per-round traces/scopes/
reduced costs and the finite-master report. The outer report carries no global proof or
speculative bounds, and records `demo4.proofScope=finite-candidates-only`.
Even exhaustive search currently issues no no-improvement certificate and is not a generic
exact branch-and-price tree. The legacy retirement callback lacks extension duals, so this
policy conservatively disables automatic retirement; explicit context retirement/re-addition
remains tested.

### Fixed-Schedule Release Recommendations

`bunch_compilation::service::FreeAircraftSelector` replaces the placeholder set difference between
all aircraft and used aircraft. `FreeAircraftSelectorConfiguration` applies bad reduced cost,
highest complete cost, most actual aircraft changes and random-without-replacement quotas in order.
Defaults are 3 per strategy, FIFO tabu capacity 10, strict threshold 0 and seed 42.
`FreeAircraftSelectionRequest` supplies one full fixed bunch per aircraft, the valid aircraft
directory, hidden aircraft and explicit complete reduced costs for every fixed aircraft, including
zero. The old task-only price callback cannot supply missing contributions. True cost stays separate
from reduced cost, and changes derive from original tasks versus actual aircraft, not stale caches.

Hidden aircraft enter first without consuming quotas twice. Bad reduced cost bypasses tabu; other
strategies respect cross-call FIFO tabu. Zero quotas/tabu capacity are valid. Finite sampling without
replacement replaces the snapshot's retry cap; equal seeds/inputs reproduce results without promising
bit-identical Kotlin RNG output. Invalid references, aircraft configuration conflicts, repeated fixed
aircraft and nonfinite costs/prices fail before random/tabu state changes. Each application owns its
selector rather than sharing state across branches.

This is a business recommendation service, not a model mutation, branch-decision override or pricing
certificate. `RecoveryApplicationService` retains Gantt's default free-executor strategy and does not
automatically turn recommendations into new fixed/hidden domains.

## Snapshot Responsibility Map

Only `907d28d` and `3dc1225` are sources; later Kotlin changes, duplicate kernels and identical file
layouts are not migration requirements.

| Snapshot responsibility | Rust counterpart and explicit boundary |
| --- | --- |
| `FlightTaskBunch`, recovery assignments and internal-airport events | Immutable records and derived queries, preserving original tasks. |
| Route graphs, labels, reordering and seeds | Finite actual-activity domains, shared Network elementary paths and original-order preferences; explicit recovery variants represent reordering/diversions, not every continuous-time solution. |
| Usability, predecessors, connections, flight hours/cycles | Full actual-record terminal validation and production replay, without unsafe static-plan pruning. |
| `RuleChecker` and `ConnectionTimeCalculator` operators | Read-only `RecoveryTransitionPolicy` shared by seeds, paths, replay and production extraction, preserving default entry behavior. |
| `Lock`, links, restrictions and flow controls | Independent aircraft/time locks, hard/soft links, actual-assignment restrictions and finite capacity windows sharing the managed lifecycle. |
| Compilation pipelines, fleet and passenger cancellation/rebooking | Gantt master with exclusively owned components, complete shared signed-dual coefficients, transactional columns and independent extraction. |
| `FreeAircraftSelector` | Stateful full-cost/actual-change/explicit-complete-reduced-cost recommendations, never undoing branches. |
| `BranchAndPriceAlgorithm` | Existing Gantt loop and unified reports, never promoting finite-candidate optima to global integer proofs. |
| Empty Cargo/Crew aggregation, empty DTOs and generic quantity samples | Domain-example boundaries remain; no invented global cargo/crew solver, and type samples are not aviation end-to-end evidence. |

## Feature Gates

```text
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-offline -- --list
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-offline
```

`demo4-offline` enables domain/bridge tests without native solver dependencies. Tests include
nonempty recovered multi-airport sequences, effective link/passenger violations, an independent
permutation oracle, alternate recoveries, terminal costs, budgets and empty bunches.

```text
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- --list
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- --include-ignored
cargo +nightly-2026-08-01 run -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- framework:demo4
```

Native tests do not silently skip after checking environment variables: the runtime and license
must actually work. Cases check internal-airport events and effective links; a seed-improvement case
improves a seed through real RMP duals/new columns, checks cross-aircraft passenger transfers,
and compares against a complete-activity binary oracle. Additional cases exercise actual fleet
shortage costs, automatic original-based seeds with locks, passenger cabin changes, actual-dual
flight-rebooking seed improvement, and partial/nonbinary passenger connection counts through
the real RMP/pricing/final-MILP loop. Link-split/idle cases verify seed improvement through
actual RMP duals, cost-driven final selection and actual half-unit slack against independent
grid/hand-computed results. Rerun list checks, execution and strict
Clippy; historical counts do not substitute for current acceptance.
Native restriction cases check actual-time seed penalties, real RMP-dual column improvement and
violation counts greater than one.
Native flow-control cases check actual internal-event seed selection, real-dual column improvement
and final-MILP extraction of nonbinary excess.
The executed-history native case starts from the actual predecessor arrival rather than the original
usability location and verifies that history is neither reselected/recharged nor counted against
remaining hours/cycles again, while the full-stay row consumes its actual time.
The injectable-transition native case imposes a longer turnaround than the aircraft default and
selects the late actual variant through the same Gantt loop, checking internal events, times and
cost. Offline cases also cover first-activity rules, actual history, invalid durations and atomic
batch-registration failure.

## Delivery And Proof Boundaries

The responsibility/API map above records the specified-snapshot audit, including fixed-schedule
release recommendations, exact actual departure locks, executed-history identity checks and
lifecycle-wide injectable transition rules. Finite recovery domains, hard/soft passenger contracts
and compilation-pipeline costs now share the managed chain, without a remaining private generation
or solve kernel to replace. Demo4 migration and the current application, affected numeric/async-feature,
native-backend and workspace regression acceptance are complete; strict Clippy for affected Core and
framework packages is included. Subsequent changes must rerun current-version gates; neither service
unit tests nor historical test counts replace application acceptance. Finite flow-control windows, actual
activity events, complete stays and production checks do not extrapolate to arbitrary overlapping apron occupancy,
every continuous-time recovery or a general global spacetime framework. Passing passenger and cost cases
alone does not replace full acceptance. Explicitly ignored optional-backend tests in the default workspace
do not count toward native gates; Demo4, standalone Network services and Demo5 native gates must execute separately.
Certifying a full-path-domain bound additionally requires no-improvement
evidence bound to current LP provenance, actual normalized prices and the complete domain.
Heuristic termination and finite-pool MILP reports cannot become global integer proofs.
