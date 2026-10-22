# Selection Hyper-Heuristics

English | [中文](README_ch.md)

Migration baseline: `ospf-kotlin` commit `2eb7943`. This module lives in `ospf-rust-core::solver::hyper_heuristic` and schedules persistent native `heuristic` sessions. It neither adds a crate nor simulates slicing by restarting complete solvers.

## Entry Points

Create a `SearchRegistry::with_builtins()`, register an independent problem factory, and build `HyperHeuristicConfig::new(...).algorithm(...)`. `ConfigurationFormats` reads the same configuration from JSON or YAML and rejects unknown fields and duplicate keys.

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
