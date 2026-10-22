# Heuristic Solvers

[中文](README_ch.md) | English

`ospf-rust-core`'s metaheuristic layer, rebuilt on one idea: **every metaheuristic is a lifecycle skeleton plus a set of operators**.

## Layering

| Layer | Module | Responsibility |
| --- | --- | --- |
| Contract | `contract.rs` | `HeuristicSolver`, `Runner`, `RunnerSolver`, `MultiObjectiveSolver` |
| Model adaptation | `model.rs` | `VariableModel`, `ObjectiveModel`, `ObjectiveView`, `HeuristicValue` |
| Operators | `operator/` | Operator families (initialisation, update, mutation, survivor, multi-objective, reinforcement learning, …) |
| Skeletons | `runner/` | Unified `drive` loop; population, trajectory, particle swarm, island, SHADE, CLPSO, CMA-ES, NSGA-II/III |
| Composition | `composition/` | Factories wiring skeletons to operators, including NSGA and memetic scheduling |
| Entries | `facade/` | One convenience function per algorithm |
| Session | `session/` | `SearchBudget`/`SearchSlice`/`SearchRandom`, problem protocol, resumable `RunnerSession` |

## Quick start

```rust
use ospf_rust_core::model::callback::AbstractCallBackModel;
use ospf_rust_core::solver::heuristic::{
    SeededEntropy, Iteration, HeuristicSolver,
    composition::SearchConfig,
    facade::hill_climbing,
};

let mut model = AbstractCallBackModel::<f64>::new("demo");
// register variables, bounds, objective, and an initial solution
let config = SearchConfig::new(500);
let mut solver = hill_climbing(config.stopping(), Box::new(SeededEntropy::new(42)));

let result = solver.solve(&mut model, &mut Iteration::new())?;
println!("best = {:?}", result.best_objective);
```

Configure termination and the archive in one place with `SearchConfig`:

```rust
let config = SearchConfig::new(1_000)
    .with_not_better_limit(100)
    .with_time_limit(std::time::Duration::from_secs(5))
    .with_archive_capacity(8);
```

## Operator families

`operator/` contains: `initializer`, `fitness`, `ranking`, `boundary`, `topology`, `diversity`, `acceptance`,
`stochastic`, `evolution`, `neighborhood`, `survivor`, `migration`, `linear`, `archive`, `stopping`, `update`,
`mutation`, `rl`, and `multi_objective`.

Randomness is always injected through `ospf_rust_math::distribution::Entropy`; operators never own global state, so the
same solver is reproducible for the same seed and safe to reuse.

## Algorithm entries

`facade/` exposes every algorithm with default operators: hill climbing, neighbourhood search, genetic, differential
evolution, particle swarm, APSO, CLPSO, SHADE, L-SHADE, CMA-ES, island GA, Q-learning, plus 20 position-update
algorithms (GWO, SCA, MVO, WOA, HHO, AOA, HS, WCA, SOA, TLBO, SSA, GSA, FA, CS, BA, MFO, GOA, WarSO, EVO, GCO).
NSGA-II/III and memetic scheduling are available through `composition`.

## Sessions

`session/` slices a search into budgeted fragments so it can be paused, checkpointed, and resumed. The generic
`RunnerSession` knows only four things (state type, initialise, advance, flatten), so one implementation carries every
skeleton — population, trajectory, particle swarm, island, CLPSO, SHADE, and CMA-ES:

```rust
use std::sync::Arc;
use ospf_rust_core::solver::heuristic::{
    RunnerSession, SearchBudget, SearchSlice, SessionCodecs,
    session::{IndividualListCodec, PopulationHooks},
};

let hooks = PopulationHooks::new(runner, Box::new(SeededEntropy::new(7)));
let codecs = SessionCodecs::new(
    Some(Box::new(IndividualListCodec::new(order.clone()))),
    Some(Box::new(IndividualListCodec::new(order))),
);
let mut session = RunnerSession::new(Box::new(hooks), problem, codecs);

let budget = Arc::new(SearchBudget::new(10_000));
let mut slice = SearchSlice::new(budget, 500, 10).with_duration(Duration::from_millis(50));
let report = session.step(&mut slice)?;
let snapshot = session.snapshot()?;   // persist here
session.restore(&snapshot)?;          // resume elsewhere
```

Budget semantics match Kotlin: a claim is charged to the slice, then to its parent (if nested), then to the global budget,
and rolls back unless every level admits it.

State codecs mirror Kotlin's `RunnerStateCodecs`: `IndividualListCodec` (population), `IslandStateCodec`, `TrajectoryStateCodec`,
`ParticleStateCodec`, `ClpsoStateCodec`, `ShadeStateCodec`, `CmaEsStateCodec`. Sessions support `import_solutions` when an
importer is supplied.

Snapshots are plain `serde_json::Value` trees, so `serde_json::to_string` persists them and another process can restore
them. `serde` and `serde_json` are unconditional dependencies of this crate; the `serde` feature is kept only for backward
compatibility.

Snapshot text is interchangeable with the Kotlin baseline:

- field names match Kotlin verbatim (`initialized`, `iteration`, `stagnation`, `state`, `archive`, `activeNanos`), and
  `version` / `activeNanos` are omitted while default, mirroring kotlinx's `encodeDefaults = false`;
- skeleton state uses Kotlin's `NativeState` shape (`individuals` / `vectors` / `counters` / `solutions` / `children`), with
  empty fields omitted;
- every number is encoded as a string in `Double.toString()` form (`1.0`, `1.0E300`), matching Kotlin's `SearchCodecs.flt64`.

So a snapshot written by either side can be read by the other.

Objective type is generic like Kotlin's `SearchProblem<ObjValue, V>`: `SearchProblem<f64, V>` for single-objective and
`SearchProblem<Vec<f64>, V>` for multi-objective. The partial order is always expressed through `dominates`, so one session
implementation serves both. For NSGA-style search use `NsgaSessionHooks`:

```rust
let hooks = NsgaSessionHooks::new(size, dominance, update, mutation, entropy, divisions);
let codecs = SessionCodecs::new(
    Some(Box::new(MultiObjectiveArchiveCodec::new(order.clone()))),
    Some(Box::new(MultiObjectiveArchiveCodec::new(order))),
);
let mut session = RunnerSession::new(Box::new(hooks), problem, codecs);
```

`select_front` is the reusable last-front truncation (crowding distance when `divisions` is `None`, Das–Dennis reference
points otherwise).

The session archive follows Kotlin's `Archive` semantics. Single-objective search keeps the best `capacity` individuals;
multi-objective search keeps the non-dominated set and trims it to `capacity` by crowding distance, evicting dominated
members as new ones arrive. `RunnerSession::new` defaults to capacity 1 for single-objective and 100 for multi-objective
(matching Kotlin's `archiveAmount`); use `RunnerSession::with_archive_capacity` to override it.

## Differences from the Kotlin baseline

- **Synchronous.** No `async`; objective evaluation runs on the caller thread.
- **No value converter.** `V: HeuristicValue` provides two-way `f64` conversion.
- **Canonical order.** Solutions are `HashMap<VariableId, V>`; positional semantics always go through
  `VariableModel::variable_order()`.
- **Serial execution.** Individuals advance sequentially, keeping results reproducible.
- **No separate crate.** Everything lives under `ospf-rust-core/src/solver/heuristic` because the workspace is
  single-level.

## Determinism

`SeededEntropy` uses the same LCG as the Kotlin tests (multiplier `6364136223846793005`, increment
`1442695040888963407`, high 53 bits), so a fixed seed produces the same stream on both sides.

## Related

- [Operator family overview](operator/README.md)
- [Skeleton overview](runner/README.md)
