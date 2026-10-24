# Time and Resources · Gantt

Choose execution modes and start times while jointly reserving machines, people and other resources, respecting dependencies, calendars and material events. The first lesson uses finite-candidate activity MILP, without first turning activities into paths.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Activity | `Activity` | Work unit; cancellation needs an explicit cost |
| Mode | `ActivityMode` | Duration, allowed starts, cost and reservation intervals |
| Resource | `ActivityResource` | Capacity, unavailable intervals and time-varying capacity |
| Dependency | `ActivityDependency` | Endpoint relations and time bounds |
| Problem and output | `ActivitySchedulingProblem` / `ActivitySchedule` | Candidates, schedule, reservations, cancellations and inventory |

Allowed starts define the business domain, not a search budget. Supplying 08:00 and 10:00 does not implicitly permit 09:00. Resource reservations are half-open intervals; availability at the start alone is insufficient.

## Entry Points

Both languages offer `ActivitySchedulingApplicationService`. Standalone services and embedded contexts share domain registration. Reusing the MILP capability of `ColumnGenerationSolver` does not make this activity path a column-generation algorithm.

APS / MPS / LSP are business-level markers, not the standalone solving entry for this lesson. Task-bunch, capacity and branch-and-price paths come later.

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

## Reference

- [Kotlin activity contracts](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-domain-activity-context/README.md)
- [Rust activity contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-gantt-scheduling/src/domain/activity/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Calendars and Rolling Recovery](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
