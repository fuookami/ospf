# Selection Hyper-Heuristics

English | [中文](README_ch.md)

Migration baseline: `ospf-kotlin` commit `2eb7943`. This module lives in `ospf-rust-core::solver::hyper_heuristic` and schedules persistent native `heuristic` sessions. It neither adds a crate nor simulates slicing by restarting complete solvers.

Adaptive extension baseline: `c66cd7f`, retaining the newer shared CMA and asynchronous replay fixes.

## Entry Points

Create a `SearchRegistry::with_builtins()`, register an independent problem factory, and build `HyperHeuristicConfig::new(...).algorithm(...)`. `ConfigurationFormats` reads the same configuration from JSON, YAML, or TOML and rejects unknown fields and duplicate keys.

`HyperHeuristicSolver::open` creates an independent run. `advance` executes one serial slice or one parallel batch; `run` continues until termination; `close` is idempotent. Inspect `result` after a failure to retain consumed costs, valid candidates, and diagnostics without reevaluating objectives.

Runnable example: [`hyper_heuristic_demo.rs`](../../../examples/hyper_heuristic_demo.rs).

```text
cargo run -p ospf-rust-core --example hyper_heuristic_demo
```

The existing base crate uses specialization and needs an appropriate toolchain. Local verification on a stable-only machine used command-scoped `RUSTC_BOOTSTRAP=1`; this is not a new requirement introduced by this module or a recommended production toolchain configuration.

## Coverage

- Seven selectors: fixed, round-robin, stagnation, random, epsilon-greedy, UCB1, and Q-learning.
- Thirty-two scalar native algorithm IDs, covering population, PSO/APSO, CLPSO, SHADE/L-SHADE, CMA-ES, and trajectory families; see `registry::BUILTIN_ALGORITHMS` for the complete list.
- NSGA-II/III vector objectives. Scalar algorithms require an explicitly registered scalarization; the archive retains original vectors and `best()` is absent for multi-objective runs.
- Persistent island/island-ga and memetic child sessions sharing a parent budget.
- Typed update/mutation pools for population and NSGA sessions, neighborhood pools for trajectory sessions, and named parameter pools for PSO, CLPSO, L-SHADE, and simulated annealing.
- Serial execution, synchronous threaded batches, and a continuously replenished completion-ordered asynchronous scheduler, without requiring the async feature.
- Instance-isolated training, frozen holdouts, scoped replay, JSONL trajectories, and equal-evaluation-budget baseline comparisons.

The Kotlin operator catalog includes single-point/arithmetic/order/cycle/SBX (`eta`) updates; none/Gaussian (`sigma`)/polynomial/swap/inversion mutations; perturbation (`amount`) neighborhoods; and constant/linear/geometric parameter sources. Rust additionally provides two-point/uniform/blend/differential updates, uniform mutations, and random/Gaussian neighborhoods. Global policies include not-worse/always acceptance and none/periodic exchange (default period 5). Unsupported slots and parameters fail before searching rather than being ignored. CMA-ES imports remain unavailable unless `restartOnImport=true` explicitly enables a distribution restart.

## Evaluation Semantics

Every evaluation request consumes allowance, including infeasible, duplicate, initialization, and import requests. `objective_calls` counts actual objective invocations separately. Initialization consumes one complete low-level step. Partial generations do not commit native state, but successfully evaluated candidates remain eligible for the archive.

Active timers pause only at quiescent boundaries; time continues while asynchronous slices are in flight. Stop and time limits are cooperative and cannot interrupt a blocking business callback. Archive and working-set acceptance are separate; acceptance never rolls back private native algorithm state. Inner policies receive feedback only when their selected operator was actually used, not during unused initialization slots.

Algorithm and operator transition `steps` both count complete committed low-level steps; an invoked operator in a partial generation still earns feedback but records zero steps. Filling a slice exactly does not by itself make an operator transition terminal while global allowance remains. A denied evaluation reports `EvaluationLimit` even for a local slice limit; a complete step that exactly fills the allowance remains `SliceComplete`. Slot-feedback failures retain committed steps, evaluated candidates, and feedback for the other invoked slots.

## Parallel Execution

Each worker constructs and owns its model, callbacks, native session, and random stream inside its thread. Models and operators need not be Send/Sync; factories must be. Only the coordinator mutates global memory and the upper policy. Candidate transport requires a registered lossless archive codec.

Disjoint evaluation, step, and decision allowances are reserved before dispatch. Synchronous batches merge by stable algorithm ID. Asynchronous `advance` fills free worker slots and merges one completion, retaining other in-flight slices; the next call can immediately replenish a fast worker without waiting for slower peers. Feedback retains the dispatch context, action, and policy revision. Checkpointing, policy export, stopping, failure, and shutdown drain all outstanding outcomes exactly once. `checkpoint` and `export_policy` therefore require mutable access. Cross-mode trajectory equality is not promised.

## Extension Points

`SearchRegistry<O,V>` registers independent problems, sessions, archive codecs, and explicit scalarizations. `registry.components` registers selectors, features, rewards, allocators, acceptance, and exchange policies. `registry.operators` registers typed update, mutation, neighborhood, and parameter-source factories. `RecoverableComponent` requires explicit statelessness or save/load support; enabling persistence rejects nonrecoverable components before search.

The orchestration API is generic. Existing value traits require floating-point semantics; built-in scalar runner/codec boundaries are `f64/f64`, and vector boundaries are `Vec<f64>/f64`. Non-continuous genetic/NSGA sessions require explicit compatible update and mutation pools, such as order/cycle and swap/inversion; continuous defaults fail preflight. Other native families reject non-continuous encodings; custom sessions remain available. Scalarizations are currently declared stateless and their meaning must be included in the problem fingerprint.

## Persistence And Learning

`export_policy` transfers algorithm and operator learning without problem state. Policy and full checkpoint inputs are mutually exclusive. Frozen learning disables both updates and exploration in learning selectors. Full checkpoints include native states, actual random streams, callback/operator/global-component states, memory, budget, active time, and transitions.

`FileSearchStore` uses independently versioned documents, SHA-256 checksums, restricted keys, and temporary-file atomic replacement. Unknown versions require an explicit migration. Problem fingerprints must cover inputs, objective directions, encoding, and callback semantics. Complete recovery requires the identical configuration.

Built-in native session archives retain only committed state members and imported candidates, with capacity 100; all valid evaluations still reach the upper layer. Scalar search memory deduplicates by solution identity, preserving distinct solutions with equal fitness. Fingerprints must also cover variable bounds and the callback's numerical operation semantics.

`PolicyTrainer` splits instances before assigning seeds and freezes validation/test runs. `PolicyEvaluator` takes an explicit quality callback rather than implicitly summing objective vectors. Replay requires an explicit scope and credits only the executed action.

## SHADE Numerical Contract

SHADE and L-SHADE accept the explicit algorithm parameter `"numerics": "shade-fdlibm-v1"`. It fixes the Cauchy parameter sampler to fdlibm tangent (Rust `libm` pinned to 0.2.16; Kotlin `StrictMath.tan`), binary64 arithmetic order, and five uniform draws per parameter pair. Omitted numerics or `"native"` retains the historical platform implementation. Incompatible algorithm/contract pairs and unknown IDs fail before search.

The contract is part of the algorithm definition and checkpoint fingerprint: restoring with a different numerical mode fails instead of silently changing the trajectory. Use the updated Kotlin reader and ordinary `save_checkpoint`; the original Kotlin export guard rejects explicit numerical contracts, including in composite children. The contract covers this sampler, not arbitrary objective callbacks, parameter-source implementations, CMA-ES, or asynchronous completion order. Bitwise comparisons of representative SHADE runs do not imply a universal cross-platform guarantee.

## Compatibility Boundary

The updated Kotlin reader is verified separately against workspace commit `c66cd7fa` plus the shared numerical and schedule contracts, not substituted for the original migration baseline. Imported `previousAction` replay metadata is preserved; this does not add support for the newer adaptive selectors, heterogeneous operators, or constraint-handling extensions. Export to the original baseline rejects nonempty predecessor metadata rather than silently losing it.

Existing native snapshot codecs are retained. Configuration default omission, canonical JSON separators, UTF-16 key ordering, algorithm/slot SplitMix64 streams, local slot IDs, and empty controlled-session wrappers follow Kotlin conventions. Floating-point parsing preserves binary precision, and stored-document checksum verification preserves the source numeric spelling, including exponent notation.

The verification baseline independently compiles five foundational modules and the heuristic module from Kotlin `2eb7943`, without using pre-existing Kotlin workspace classes. Both sides use identical explicit bounds, initialization, feasibility, and objective operations. The 51 representative scenarios cover all 32 scalar native IDs, NSGA-II/III, island/memetic, synchronous/asynchronous execution, explicit scalarizations, order/cycle with swap/inversion permutation operators, stateful parameter sources, and different parameter schedules/seeds. Beyond bidirectional policy loading and full-checkpoint restoration, serial slice boundaries compare native states, random streams, archives, budgets, and learning events, excluding active runtime and serialization-default omission.

Scalarized native states remain scalar, while controlled local archives, rewards, and operator feedback retain canonical objective vectors. Controlled and composite sessions retain an arrival-ordered nondominated prefix in local multiobjective archives (capacity 100); only the global multiobjective archive uses crowding-distance truncation. Checkpoint restoration preserves this distinction. Stateful parameter sources can resume through an agreed JSON schema; compatibility is not limited to stateless built-ins. Custom codecs, component implementations, and other encodings still need matching formats and semantics. Representative scenarios do not certify arbitrary closures or every parameter combination.

The shared inner-reward protocol uses `rewardStateVersion: 1` and `rewardState` in each controlled session. Global reward state remains in `components.reward`; each controlled worker/composite leaf owns a separate reward instance. Only invoked slots advance local reward state, in update, mutation, neighborhood, then lexicographically sorted parameter-slot order. `RewardContext.action` identifies `slot/action` locally and the worker ID globally. Missing versions read legacy unversioned `rewardState` or Rust `reward`; unknown versions, missing versioned state, and conflicting aliases fail. Component IDs, versions, and configuration fingerprints still have to match, and custom payloads require an agreed schema. Duration metadata is preserved when reading transitions but is not a deterministic quantity.

The original Kotlin `2eb7943` does not understand reward-state fields. `RunCheckpoint::validate_kotlin_v1()` and `FileSearchStore::save_kotlin_checkpoint()` remain strict export guards for that baseline: unsupported fields fail before writing, including in nested children, without replacing existing files. Use ordinary `save_checkpoint` with the updated Kotlin reader for shared-protocol checkpoints; do not strip state to satisfy the old guard. Stateless snapshots omit the new fields and remain compatible with the baseline.

**Bit-identical trajectories across execution environments are not guaranteed.** JVM and Rust transcendental functions may differ by ULPs and eventually affect branches; strict equality requires a shared numerical implementation, not suppressed errors or a final-best-only comparison. Asynchronous completion order is inherently nondeterministic, so restoring complete state does not reproduce another process's thread timing. Use serial/synchronous execution or explicitly record and replay a schedule when stable merge order is required. Report numerical-tolerance acceptance separately from exact structural, random-stream, and budget checks.

## CMA Numerical Contract

CMA-ES accepts `"numerics": "cma-polar-fdlibm-v1"`. Direct runners select
`CmaNumerics::PolarFdlibmV1` with `with_numerics` in Rust, or `CmaNumerics.PolarFdlibmV1`
in Kotlin. It uses Marsaglia polar Gaussian sampling: reject zero/outside-unit-disk pairs,
consume two uniform draws per attempt, use the first component only, and do not cache its
partner. This preserves Gaussian sampling without relying on platform cosine. Native
Box-Muller remains the default.

The shared mode fixes the original fdlibm logarithm's operation grouping (not the differently
grouped musl log), Rust `libm=0.2.16` exp/pow/sqrt against Kotlin `StrictMath`, positive-zero
sequential sums, and a 60-sweep Jacobi decomposition with `1e-12` stopping/pair thresholds,
unsorted eigenvalues and column eigenvectors. Mode identity participates in configuration
and policy compatibility. Restart-on-import uses all imported candidates for the mean and
sample covariance; a single candidate retains covariance, and a better historical best is
retained. Multi-dimensional, scalarized and SHADE-to-CMA memetic trajectories have exact
full-state and binary64 tests. Arbitrary callback arithmetic, rounding modes and every
machine are not certified.

## Asynchronous Schedule Replay

For `Asynchronous` execution with parallelism >= 2 and **no time budget**, enable
`record_schedule()` (Rust) / `recordSchedule()` (Kotlin) before the first dispatch. After
termination, `export_schedule()` / `exportSchedule()` returns a version-1 `ScheduleLog`.
`FileSearchStore` provides validated atomic `save_schedule` / `load_schedule` (Rust) and
`saveSchedule` / `loadSchedule` (Kotlin). A fresh run binds `replay_schedule(&log)` /
`replaySchedule(log)` and executes the algorithms normally. It waits for the recorded
worker even when natural completion order is reversed; it does not replay stored objective
values or just learning updates.

Events `begin/dispatch/complete/drain/yield/end` capture advance boundaries, ordered available
actions, random bits, reserved/actual budgets, stop/failure flags and quiescent drain barriers.
Canonical JSON SHA-256, configuration/problem identities, event pairing and totals are
verified. Divergence fails and drains already-dispatched slices exactly once; a diverged
run cannot produce a checkpoint. Source corruption/truncation and incompatible identities
are rejected rather than silently falling back to natural completion order.

Quiescent checkpoints embed the consumed prefix and cursor in optional `schedule`. Restore
normally, then explicitly rebind replay with the **complete source log**, or resume recording,
before advancing. The source must match that prefix; checkpoint identities and completion
totals must agree with the run. Stop and worker-failure drains are covered. The original
Kotlin export guard rejects this new state. Custom callbacks/components must independently
have identical deterministic semantics; time-dependent rewards or external side effects
are outside this contract. Ordinary unrecorded asynchronous execution remains nondeterministic.

## Business Coordinates And Search Domains

`SearchProblem` separates encoding validity (`valid_solution`), search admission (`admissible`),
and strict feasibility (`feasible`). Admission defaults to feasibility, preserving existing
problems. Explicit overrides can admit infeasible native-search candidates, but global and
controlled valid archives, working sets, reward coordinates, and memory restoration still
require strict feasibility and admission outside forbidden business domains. `evaluate`
charges a request token first; only valid, admissible encodings increment objective calls.

Credit coordinates require both `has_objective_coordinates() -> true` and
`objective_coordinates(&O) -> Result<Vec<f64>>`. Normalize using fixed business scales with
smaller values preferred; explicitly negate maximization. Never implicitly sum vectors or
replace business objectives with penalty values. `checked_objective_coordinates` rejects
empty/nonfinite coordinates and multidimensional scalar projections; reward construction
also checks dimensions across reference and candidate points. Scalarized native states stay
scalar, while both credit levels project the canonical original objective vectors instead
of treating scalarization results as original coordinates.

`has_constraint_violation` and `constraint_violation` expose an independent domain callback.
`checked_constraint_violation` rejects missing/error/negative/nonfinite readings. This callback
must not bypass metering to evaluate a full objective, and zero violation never proves strict
feasibility. Leaf `AlgorithmConfig.constraints` supports `FeasibilityFirst`, `Epsilon`, and
`StochasticRanking` with independent restorable state. Composite roots reject this field;
configure each child separately. Custom factories opt in with
`register_algorithm_with_constraints(id, true, factory)` and must honor the transaction protocol.

Constraint handling preserves `admissible` by default: Rust cannot detect trait overrides.
Explicitly override `constraint_admissible` to relax a feasibility-only default, retaining all
business exclusions, or define an explicit `admissible` search domain. There is no automatic
`true` admission. Original-domain custom `evaluate` callbacks remain in use and are charged
exactly once. If an explicitly relaxed domain also needs custom evaluation/cache semantics,
override `evaluate_constraint_admissible`; its default uses the original `evaluate` where
admitted, and metered `objective` only in the explicitly expanded domain.

Comparisons retain complete individuals and original fitness. Stochastic ranking uses bounded
batch scans, never a random comparator; pairwise best/acceptance decisions stay deterministic.
Strict feasibility outranks zero violation in native modes. Epsilon calibrates once from the
first committed step unless `initialEpsilon` is explicit, then tightens over `horizon` steps
with `exponent`. Partial/failed steps restore native state, local archive, calibration, and
constraint RNG, without refunding requests or removing evaluated candidates from reports.
Imports do not advance epsilon; failures roll back constraint state, while successful partial
imports retain their admitted prefix. Native/controlled snapshots preserve `constraints`, and
the old Kotlin 2eb7943 export guard rejects it instead of discarding it.

For already metered domain values, `operator::fitness` also exposes `ConstraintAssessment<T>`,
`EpsilonConstraintEvaluator`, and `StochasticRanking`. These standalone rankers consume no
objective tokens and do not certify feasibility or relax strict global archives.

`RewardPolicy::requires_coordinates` checks capabilities when opening a run without inferring
scalar projections. `RewardContext` carries the actual action ID, request count, worker-local
execution duration, `reference` coordinates frozen before dispatch, and filtered completion
`candidates`. Projection neither evaluates objectives nor consumes objective tokens. Composite
leaves retain independent reward instances. Coordinate errors produce failed feedback without
discarding already-evaluated valid candidates.

Custom rewards depending on real time must declare `requires_elapsed_time() -> true`.
Schedule recording/exact replay reject this combination before binding; ordinary execution
remains supported. Only explicitly time-dependent rewards populate transition duration
metadata; others retain zero to preserve existing deterministic trajectories. Context duration
is always actually measured and must not secretly affect policy updates without that declared
dependency. Built-in adaptive policies are described next.

## Adaptive Credit And Acceptance

`learning.reward` supports `time-normalized` (epsilonSeconds=1e-9), `extreme-value`
(window=50), `dominance`, and `hypervolume-delta`. The first two require an explicitly
projected scalar objective; dominance consumes effective novel contribution. Extreme
credit expires through a global action window, not an unbounded per-action maximum.
Each leaf and the root own independent reward state. Time-normalized credit is available
for ordinary execution but explicitly rejected by exact E4 recording/replay.

Hypervolume uses arbitrary-dimensional smaller-is-better coordinates. Set `reference`
to a finite array, or use the paired `referenceX`/`referenceY` aliases; do not mix them.
With no reference, `calibrationObservations=32` valid candidates calibrate and freeze a
worse reference with a margin. Calibration and the freezing action receive zero credit.
Exact recursive slicing defaults to `maxWork=1000000` and a depth limit of 256. Exhaustion
fails explicitly: it never silently switches to sampling. Explicit `mode="sampled"`
uses `samples=16384`, `seed=1`, common random numbers, standard error, and a 99% Hoeffding
absolute error bound. Reference calibration and `last_measurement()` survive checkpoints;
failed geometry does not partially commit calibration or overwrite the last measurement.

`memory.acceptance` additionally supports `late` (length=50), `record-to-record`
(initialRatio=0.05, horizon=1000), and `auto` (no component parameters). Late acceptance
advances original-objective history even on rejection. Record acceptance requires scalar
minimization coordinates and tightens its threshold to zero. Auto derives capacity from
the first available decision/step/evaluation limit, adapts its span using strict improvement,
and preserves decisions under positive objective scaling and translation. Its budget
hint is not a stopping limit. Late and record acceptance are not parameter-free policies.

For custom objective types, register `SearchRegistry::set_objective_codec` before using
late or auto acceptance. Histories store the original business objective losslessly, not
its coordinate projection, and do not require `O: Serialize`. Scalar and vector built-in
registries provide their native codecs. Acceptance only changes working memory: rejected
valid candidates remain eligible for the archive, and native private state is never rolled
back by acceptance. Old Kotlin 2eb7943 exports reject the new components explicitly.

## Non-Stationary Selection

Root `selection` and each algorithm's slot `selection` accept `frr-mab` (window 50,
exploration 0.5, decay 0.8), `choice-function` (rate 0.2, recencyWeight 0.1), and
`adaptive-pursuit` (rate 0.2, minimumProbability 0.01). Available-action masks preserve
canonical action order; pursuit renormalizes its probabilities inside the mask.

Snapshots retain the complete credit window, quality, transition pairs, probabilities,
last-use counters, predecessor, pending decisions, and clock. Restore matches action IDs
and validates definitions and probability floors; `LearningMode::Frozen` does not update
credit. Choice-function freezes predecessors at dispatch, including an absent predecessor,
so asynchronous completion order cannot silently change attribution. New non-stationary
states are rejected by the old Kotlin 2eb7943 export guard rather than stripped.

`SelectionPolicy::predecessor` and `discard` have compatible defaults for custom selectors.
Failed preparation and unused slots discard pending credit without fabricating feedback;
each executed slot receives one observation. Local terminal feedback follows its own
execution result, not a racing shared global counter. Stop/failure drains and E4 schedule
replay preserve complete selector state at quiescent checkpoints. Shared fixed numerical
vectors and complete cross-language traces cover representative binary64 interoperability;
they do not certify every platform, callback, or parameter combination.

## Native Import Synchronization

PSO/APSO/CLPSO imports use `runner::ContextStateSynchronizer`: each already-metered incoming
candidate matches the nearest particle in the original destination state, derives clamped
displacement velocity using canonical variable order, and retains that source's genuine
historical best when better. Imports replace the worst personal-best slots in reverse order;
they do not reset every velocity to zero. CLPSO then rebuilds legal self-exemplar indices.

SHADE/L-SHADE deduplicate and rank the merged population, add displaced destination members
to the bounded external archive, and preserve the actual `M_F`, `M_CR`, and memory cursor.
No successful parameter samples are invented for imported solutions. Existing CMA imports
continue to use the full covariance restart protocol, retaining covariance for a single seed.
Native imports still use the ordinary evaluation budget and do not reset paused sessions.

## Local Search And Orthogonal Actions

Update slots support `pattern-search` (`initialStep=0.1`, `contraction=0.5`, `sweeps=32`),
`opposition`, `mutation-stage` (`operator`, `rate=1`), `neighborhood-stage` (`operator`),
and `pipeline`, `vnd` (`maxTrials=50`), `elite-local` (`amount=1`). Composites require a
nonempty `stages` array. VND restarts at the first neighborhood on strict improvement;
elite-local intensifies only the caller's ranked Top-K and retains other parents.
Opposition reflects only dimensions with finite lower and upper bounds.

`VariableModel::local_evaluation` is absent by default. Native scalar and NSGA sessions
explicitly provide lifetime-bound `LocalEvaluation`/`EvaluatedSolution` ledgers, retaining
the original objective type inside the model. Local seeds, coordinate/pattern trials,
and every pipeline intermediate use the ordinary complete, metered evaluation entry.
Ranking never evaluates objectives; handles cannot compare across ledgers or persist as
cross-step caches. The outer runner still reevaluates final output instead of assigning
stale fitness. Pattern search stops polling at two remaining requests to reserve room for
the outer evaluation. A plain variable model without a ledger cannot silently evaluate
objectives outside the budget.

Nesting is limited to depth 16, 1..64 stages per composite, and 1024 nodes per action tree.
Custom children require `register_*_with_capabilities` with explicit stateless, compatible
encoding declarations, and instances must also declare `stateless`. Legacy registration
continues to support continuous top-level use, not implicit nesting or permutation use.
Stateful compositions must implement recoverable components without discarding child state.

`OrthogonalOperatorPortfolio::continuous(size)` requires at least four individuals and
returns distribution, adaptive-DE, encircling, escape, and intensification niches.
`permutation(size)` requires a positive size and uses only order crossover and inversion/swap.
Both return ordinary `AlgorithmConfig` lists; the domain model supplies valid initial
solutions. Each slot has at most two actions, keeping continuous and permutation encodings separate.

## Domain Ruin And Recreate

`heuristic::operator::neighborhood` exposes generic `RuinResult<T>`, `Ruin<T>`, `RandomRuin`,
`WorstCostRuin<T>`, `RelatednessRuin<T>`, `InsertionCost<T>`, `Repair<T>`,
`RegretKInsertRepair<T>`, `GreedyInsertRepair<T>`, and `RuinRecreateOperator<T>`.
For `T: Clone + PartialEq`, equality must represent stable unique identity. Duplicate or
non-reflexive identities such as NaN are rejected. Removal is without replacement and
partitions retain original order. The action also checks custom implementations for lost,
duplicated, or substituted elements.

Typed callbacks provide removal benefits, relatedness distances, and incremental insertion
costs; closures are never serialized or implicitly used to evaluate full objectives.
Costs must be finite, distances also nonnegative, and insertion `None` forbids a position.
Regret-K first prioritizes items with fewer than K options, then regret and minimum cost;
K=1 is global greedy insertion. Irreparability, overflowing regret, and callback failures
are explicit errors rather than silently dropped items. Finite cost sorting preserves JVM
signed-zero ordering and stable equal-cost positions.

`runner::RuinRecreateSearchSession` takes an exclusively owned `SearchProblem`, an action,
an original-objective individual codec, and `RuinRecreateSessionConfig { amount, random,
stateless_estimates }`. It requires permutation encoding and a valid removal count. Register
it using ordinary `SearchRegistry::register_algorithm`, without coordinator changes or a
hardcoded domain factory. This direct session does not declare native constraint transaction
support; custom registration flags cannot bypass that check. Complete neighbors and imports
use `SearchProblem::evaluate`. Partial estimates must not call unmetered full domain objectives.
Failure reports retain completed prefixes, evaluated candidates, and real consumed requests,
without refunds or duplicate feedback.

The session saves current state, iteration, and an independent random stream. Estimates must
explicitly be stateless or provide `EstimateState` through `with_estimate_state`. Failed
estimate restoration attempts to roll back the old estimate state and rejects unexpected
state; restoration never evaluates objectives. Problem callbacks use the existing, separate
outer problem-state protocol; standalone callers save/restore using `problem()`/`problem_mut()`.
The outer configuration fingerprint protects amount and other configuration. Original O does
not require Serde, and codecs must not replace domain objectives with scalarizations.

## Configuration And Adaptive Numerics

`ConfigurationFormats::encode_format` / `decode_format` accept `Json`, `Yaml`, and `Toml`.
All three use the same structural validation and explicit version migrations; component
parameters, versions, capabilities, encodings, and nested stages are checked by the registry
before evaluation. Input is limited to 4 MiB of UTF-8. Non-finite numbers and duplicate keys
are rejected. TOML omits absent model options, but rejects business parameter nulls rather
than silently losing them. TOML date/time values become ISO strings; JSON strings remain
strings. Constraint configurations and snapshots omit declared defaults, while required
`steps`, nullable `scale`, and independent constraint `random` are retained.

New FRR-MAB logarithms/powers, epsilon powers, sampled-HV uncertainty, and Auto square roots
use fixed fdlibm operations matching Kotlin `StrictMath`, not platform-native approximations.
Observed one-ULP differences motivated this fix; shared vectors cover 50,000 primitive/formula
results plus capacity boundaries. This is not a certification of arbitrary callback arithmetic
or all machine/toolchain combinations. Existing explicit CMA/SHADE numerical modes are
unchanged. Historical archives use deterministic complete-individual comparisons and do not
consume the separate stochastic-ranking stream reserved for population selection.

Custom algorithms registered with a lossless archive codec use the same empty-slot control
layer as Kotlin, including local archive and decision history. The current Rust custom factory
API does not expose operator bindings, so configured slots are rejected rather than ignored.
Legacy bare Rust custom checkpoints still restore in their original passthrough mode; absent
control history is never fabricated. Runs without an archive codec retain the non-recoverable
passthrough path.
