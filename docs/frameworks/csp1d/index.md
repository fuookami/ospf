# 1D Cutting · CSP1D

Material has an available width and orders require products of different widths. Select cutting plans and their usage counts to satisfy demand, with configurable material, machine, production-deviation and waste rules.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Product and demand | `Product` / `ProductDemand` | Widths, optional length, quantity and demand mode |
| Material and machine | `Material` / `Machine` | Width, batches, machines and optional resource limits |
| Cutting plan | `CuttingPlan` | Product contribution per execution |
| Problem | `Csp1dProblem` | Domain data and solve configuration |
| Solving and output | `Csp1dMilp` / `Csp1dColumnGeneration` | Plan usage, production, demand satisfaction and enriched metrics |

**Variables count plan usage, not product output.** Production is the sum of contributions times usage. Roll, sheet, length and weight demands are not interchangeable.

## Entry Points

Both languages offer ordinary MILP and column generation, with different adapters, result types and configuration. Learn ordinary solving before initial plans, LP, pricing, column addition and final integer solving.

Business HTTP DTOs, tenancy, formula languages and runtime policy belong in the business adapter, not the reusable kernel.

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

## Reference

- [Kotlin API and extension contracts](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/README.md)
- [Rust API and solving lifecycle](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp1d/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Demand Units and Business Rules](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
