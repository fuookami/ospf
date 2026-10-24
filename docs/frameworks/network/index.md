# Network Optimization

Meet supply and demand on a directed network, respect transport/processing capacities and minimize cost. Add network construction, resource paths, structure and explicit failure scenarios later. Network is not just vehicle routing.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Topology | Node / Arc | Stable identities, endpoints and parallel services |
| Commodity and balance | Commodity / Balance | Per-commodity node balances and units |
| Capacity and cost | Capacity / Cost | Arc/shared-resource bounds and tariffs |
| Flow problem | `FlowProblem` | Topology, commodities, costs and resources |
| Registration/extraction | `FlowContext` | Conservation, capacity, cost expressions and flow results |

An integer step is a measurement increment, not an unsplittable order. Sharing resources across commodities needs physical conversions; tonnes and pieces cannot be added directly.

## Entry Points

Standalone flow entries are Kotlin `NetworkFlowApplicationService` and Rust `FlowApplicationService`. Embed a `FlowContext` in another model instead of starting an independent optimization with unrelated variables.

Learn flow and capacity before construction and resilience. VRPTW route columns, ESPPRC and branch-and-price are advanced paths.

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

## Reference

- [Kotlin network domains and standalone entries](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-network-scheduling/README.md)
- [Rust network domains and proof contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-network-scheduling/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Shared Resources, Shortage and Embedding](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
