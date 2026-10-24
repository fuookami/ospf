# Time and Resources · Gantt: Scope and Learning Path

Choose execution modes and start times while jointly reserving machines, people and other resources, respecting dependencies, calendars and material events. The first lesson uses finite-candidate activity MILP, without first turning activities into paths.

## Minimum Vocabulary

| Concept | API | Role | Key Fields |
| --- | --- | --- | --- |
| Activity | `Activity` | Input | Work unit; cancellation needs an explicit cost |
| Mode | `ActivityMode` | Input | Duration, allowed starts, cost and reservation intervals |
| Resource | `ActivityResource` | Input | Capacity, unavailable intervals and time-varying capacity |
| Dependency | `ActivityDependency` | Input | Endpoint relations and time bounds |
| Scheduling problem | `ActivitySchedulingProblem` | Input | Activities, resources, dependencies, execution facts and baselines |
| Scheduling result | `ActivitySchedule` | Output | Modes, times, reservations, cancellations, cost and execution facts |

Allowed starts define the business domain, not a search budget. Supplying 08:00 and 10:00 does not implicitly permit 09:00. Resource reservations are half-open intervals; availability at the start alone is insufficient.

::: tip Cancellation and Phase Nullability
A missing cancellation cost makes an activity mandatory; zero permits free cancellation. A missing resource-use `until` means the mode end, not a zero-duration phase.
:::

## Entry Points

::: info Kotlin and Rust Entry Points
Both languages offer `ActivitySchedulingApplicationService`. Standalone services and embedded contexts share domain registration. Reusing the MILP capability of `ColumnGenerationSolver` does not make this activity path a column-generation algorithm.
:::

APS / MPS / LSP are business-level markers, not the standalone solving entry for this lesson. Task-bunch, capacity and branch-and-price paths come later.

Finite-start activity scheduling needs no column-generation switch. Allowed starts define business choices, not a node budget; time/node limits belong to the backend. Start with a small declared set and expand according to business granularity. The [second lesson](./business-scenario#entry-budget) retains this entry.

## Boundaries

- This is not a promise of arbitrary continuous-time scheduling, automatic preemption/batching or complete industry regulations.
- Calendar downtime does not automatically mean pausing and resuming an activity.
- Joint resources and material inventory have separate contracts; skill-pool capacity does not replace named-person exclusivity.
- Ordinary Gantt does not require Network. Evaluate composition for travel subproblems across locations.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first activity schedule](./first-schedule), checking modes, precedence and reservations.
3. Add downtime, baseline changes and material events. Distinguish execution facts from decisions still being optimized.
4. Read [rolling optimization](/guide/rolling-optimization), then the [flight-recovery example](/examples/framework-example4).

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## Reference

- [Kotlin activity rules](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-domain-activity-context/README.md)
- [Rust activity rules](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-gantt-scheduling/src/domain/activity/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First Activity Schedule](./first-schedule). Then explore [Calendars and Rolling Recovery](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
