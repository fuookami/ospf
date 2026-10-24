# Network Optimization: Scope and Learning Path

Meet supply and demand on a directed network, respect transport/processing capacities and minimize cost. Add network construction, resource paths, structure and explicit failure scenarios later. Network is not just vehicle routing.

## Minimum Vocabulary

| Concept | API | Role | Key Fields |
| --- | --- | --- | --- |
| Topology | Node / Arc | Input | Stable identities, endpoints and parallel services |
| Commodity and balance | Commodity / Balance | Input | Per-commodity node balances and units |
| Capacity and cost | Capacity / Cost | Input | Arc/shared-resource bounds and tariffs |
| Flow problem | `FlowProblem` | Input | Topology, commodities, costs and resources |
| Registration/extraction | `FlowContext` | Entry / validation | Conservation, capacity, cost expressions and flow results |
| Flow result | `FlowSolution` | Output | Commodity-arc flows, resource usage, spills, shortages and cost |

An integer step is a measurement increment, not an unsplittable order. Sharing resources across commodities needs physical conversions; tonnes and pieces cannot be added directly.

::: tip Allowed-Arc Nullability
A missing allowed-arc set permits all service arcs; an empty set permits none. Retain units on balances and shared capacity; integer steps define measurement granularity.
:::

## Entry Points

::: info Kotlin and Rust Entry Points
Standalone flow entries are Kotlin `NetworkFlowApplicationService` and Rust `FlowApplicationService`. Embed a `FlowContext` in another model instead of starting an independent optimization with unrelated variables.
:::

Learn flow and capacity before construction and resilience. VRPTW route columns, ESPPRC and branch-and-price are advanced paths.

Use the standalone flow service for flows and shared capacity, and a shared model for coupled decisions. There is no `NetworkColumnGeneration` switch here. Route-column generation is a separate advanced workflow; flow backend budgets are not column counts. See the [call example](./business-scenario#entry-budget).

## Boundaries

- Parallel arcs need distinct identities, not just endpoint-based deduplication.
- Connectivity does not prove transport capacity; constructed total capacity is not incremental capacity.
- Explicit resilience covers supplied scenarios, not arbitrary unknown failures.
- Repair timing, staffing and calendars belong to [Gantt](../gantt/), not automatic Network decisions.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first network flow](./first-flow), checking conservation, costs and capacities.
3. Add shared resources, capacity levels, budgets and one failure scenario, changing one condition at a time.
4. Continue to the [VRPTW integrated example](/examples/framework-example5) when routes are needed.

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## Reference

- [Kotlin network domains and standalone entries](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-network-scheduling/README.md)
- [Rust network domains and solver-result documentation](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-network-scheduling/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First Network Flow](./first-flow). Then explore [Shared Resources, Shortage and Embedding](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
