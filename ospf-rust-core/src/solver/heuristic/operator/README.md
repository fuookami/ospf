# Operator families

[中文](README_ch.md) | English

Operators are the replaceable parts of a metaheuristic. Every family is a trait plus concrete stateless implementations;
randomness and the entropy source are injected by the caller, never owned by an operator.

## Families

| Module | Trait | Responsibility |
| --- | --- | --- |
| `initializer` | `Initializer` | Build the initial solutions |
| `fitness` | `FitnessEvaluator` | Turn a solution into an individual and reject non-finite objectives |
| `ranking` | `Ranking` | Order individuals |
| `boundary` | `BoundaryHandler` | Clamp, reflect, or wrap out-of-range values |
| `topology` | `Topology` | Define neighbourhood relations |
| `diversity` | `DiversityMetric` | Measure population spread |
| `acceptance` | `Acceptance` | Decide whether a candidate replaces the current solution |
| `stochastic` | `LevyFlight`, `NoiseStep` | Heavy-tailed and per-dimension random steps |
| `evolution` | `ParameterSource` | Schedule parameters across iterations |
| `neighborhood` | `Neighborhood` | Generate single-solution neighbours |
| `survivor` | `SurvivorSelection` | Merge parents and offspring |
| `migration` | `IslandMigration` | Exchange individuals between islands |
| `linear` | `SymmetricEigen` | Symmetric eigendecomposition (Jacobi) |
| `update` | `SolutionUpdate` | Crossover and position updates |
| `mutation` | `Mutation` | Per-gene mutation |
| `rl` | `StateDiscretizer`, `ActionSelector`, `QTable` | Reinforcement-learning driven selection |
| `multi_objective` | `Dominance`, `FastNonDominatedSort`, `CrowdingDistance`, `ParetoArchive`, `ReferencePoints` | Pareto tooling |
| `archive` | `Archive` | Maintain the best set and the improvement signal |
| `stopping` | `StoppingCriterion` | Decide when to stop |

## Shared helpers

`operator/mod.rs` provides the boilerplate every family uses: `compare_strictly` (two-way comparison normalisation),
`value_at` / `set_value_at` (canonical-order access), `bounds_of`, `clamp_to`, `clamped_value`, `pick_two`, `unit`,
`should_fire`, `uniform_between`, and `clamp_unit`.

## Conventions

- Operators are stateless; the entropy source is always a parameter.
- Non-finite values are rejected at the evaluator and clamped at the boundary; they never enter the archive.
- Positional semantics always go through `VariableModel::variable_order()`.
- Survivor selection uses repeated best-of rather than sorting, because model comparators may be non-transitive.

## Related

- [Module overview](../README.md)
- [Skeleton overview](../runner/README.md)
