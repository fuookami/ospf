# 1D Cutting · CSP1D: Scope and Learning Path

Material has an available width and orders require products of different widths. Select cutting plans and their usage counts to satisfy demand, with configurable material, machine, production-deviation and waste rules.

## Minimum Vocabulary

| Concept | API | Role | Key Fields |
| --- | --- | --- | --- |
| Product and demand | `Product` / `ProductDemand` | Input | Widths, optional length, quantity and demand mode |
| Material and machine | `Material` / `Machine` | Input | Width, batches, machines and optional resource limits |
| Cutting plan | `CuttingPlan` | Intermediate | Product contribution per execution |
| Problem | `Csp1dProblem` | Input | Domain data and solve configuration |
| Solving and output | `Csp1dMilp` / `Csp1dColumnGeneration` | Entry / validation | Plan usage, production, demand satisfaction and enriched metrics |
| Production result | `Csp1dSolution` | Output | Plan usage, per-execution contribution, shortfall, remaining width and KPIs |

**Variables count plan usage, not product output.** Production is the sum of contributions times usage. Roll, sheet, length and weight demands are not interchangeable.

::: tip Missing Is Not Zero Waste
An absent remaining width means it could not be calculated from the data/arithmetic, not zero remaining width. Supply product data for weight/length demand; unknown weight is not 0 kg.
:::

## Entry Points

::: info Kotlin and Rust Entry Points
Both languages offer ordinary MILP and column generation, with different adapters, result types and configuration. Learn ordinary solving before initial plans, LP, pricing, column addition and final integer solving.
:::

Business HTTP DTOs, tenancy, formula languages and runtime policy belong in the business adapter, not the reusable kernel.

Use `Csp1dMilp` when a small candidate pool is easy to inspect; consider `Csp1dColumnGeneration` as width combinations grow. Bound initial plans, new plans per round and iterations. There is no universal order-count threshold; see the [same-input switch](./business-scenario#entry-budget).

## Boundaries

- Identify product, quantity and unit together; do not collapse heterogeneous demand into a bare number.
- Machine data alone does not enable every capacity constraint. Check plan resource consumption and configuration.
- An insufficient initial pool may require a feasibility phase. Restricted-pool infeasibility is not full-problem infeasibility.
- Heuristic or bounded pricing termination does not automatically prove integer optimality.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first slitting plan](./first-slitting), checking how plans contribute production.
3. Add overproduction bounds, shortage penalties, waste and length configuration, then consider custom pipelines.
4. Read [framework example 3](/examples/framework-example3) and its context models for composition and column generation.

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## Reference

- [Kotlin API and extension interfaces](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/README.md)
- [Rust API and solving lifecycle](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp1d/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First Slitting Plan](./first-slitting). Then explore [Demand Units and Business Rules](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
