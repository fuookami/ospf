# Search skeletons

[中文](README_ch.md) | English

A skeleton owns only the lifecycle: build the initial state, advance one step, and flatten the state into individuals.
The loop, archive, stopping, and observers are implemented once, by `drive`.

## The loop

```text
state = initialize(model, iteration)
archive.update(members(state))
iteration.next(true)
while !stopping.should_stop(iteration):
    state = advance(model, iteration, state)
    improved = archive.update(members(state))
    broadcast observers(iteration, members(state), improved)
    model.flush()
    iteration.next(improved)
return archive.individuals()
```

`improved` is the only source of the stagnation signal; `iteration.next(improved)` keeps `not_better_iteration` in sync.

## Skeletons

| Skeleton | State | One step | Typical algorithms |
| --- | --- | --- | --- |
| `PopulationRunner` | population | update → mutation → survivor | GA, DE, GWO, WOA, HS, … |
| `TrajectoryRunner` | one individual | neighbour → acceptance | hill climbing, SA, neighbourhood search |
| `ParticleSwarmRunner` | particles (position, velocity, personal best) | velocity update | PSO, APSO |
| `IslandRunner` | several populations | per-island generation + periodic migration | island GA |
| `Nsga2Runner` / `Nsga3Runner` | multi-objective population | breed → non-dominated sort → crowding / reference points | NSGA-II, NSGA-III |

`PopulationRunner` also exposes `initialize_island` / `step_island`, which is how `IslandRunner` composes it instead of
reimplementing the population logic.

## Conventions

- Execution is serial: individuals advance one at a time, so a fixed seed reproduces the same run.
- `drive` receives the entropy source and passes it to `advance`; skeletons never own randomness.
- The archive is the only convergence carrier; skeletons never keep their own "best".
- Observers are broadcast after every advance, including the very first archive update.

## Related

- [Module overview](../README.md)
- [Operator families](../operator/README.md)
