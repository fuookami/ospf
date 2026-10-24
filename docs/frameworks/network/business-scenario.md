# Shared Resources, Shortage and Embedding

After [your first network flow](./first-flow), add a non-arc capacity before deciding whether shortage is allowed. Connectivity and adequate arc capacity do not establish business feasibility.

## 1. A Shared Handling Resource Can Make Flow Infeasible

Use normalized units of one commodity: source supplies 2, sink demands 2, and the delivery arc has capacity 3. Add shared dock capacity 1, consuming one resource unit per transported commodity unit.

The arc can carry demand but dock cannot. Another arc does not necessarily help if it consumes the same dock. Feasibility requires more resource capacity, different actual consumption or explicitly authorized demand relaxation.

Start from the first-lesson flow problem, retain balances and add only the shared resource row. Solve again and check actual flows and shared-resource consumption.

## 2. Convert Resource Units Across Commodities

### Build Balances and Arcs Before Adding the Dock

The code normalizes one business unit of a single commodity to `NoneUnit`, with nodes `source` and `sink`. Start with `makeProblem(2.0)` / `make_problem(2.0)`, then reduce dock capacity to 1.0. Arc capacity stays at 3 and balances stay at ±2.

::: code-group

<<< ../_examples/network-business.kt#input [Kotlin]

<<< ../_examples/network-business.rs#input [Rust]

:::

| Object | Kotlin | Rust |
| --- | --- | --- |
| Shared resource | `FlowCapacity` | `FlowResource` |
| Commodity-arc usage | usage keyed by `FlowKey` | commodity/arc keys in consumptions |
| Allowed services | `allowedArcs` | `allowed_arcs` |
| Elastic balances | `FlowSlack` | `FlowBalanceSlack` |

::: tip Unit Constraints
Commodities measured in kg, m or pieces need resource-unit/commodity-unit conversion coefficients before summation. Normalized integer steps are not physical usage. Distinct parallel-service IDs must remain distinct.
:::

## 3. Shortage Needs Bounds and Costs

Construct a separate spill/shortage scenario using one normalized unit:

| Condition | Value |
| --- | --- |
| Supply at source / demand at sink | 3 / 4 |
| Arc capacity / transport price | 2 / 1 |
| Allowed unused supply at source / penalty | At most 1 / 5 |
| Allowed unmet demand at sink / penalty | At most 2 / 7 |

Flow 2, unused supply 1 and shortage 2 cost `2×1 + 1×5 + 2×7 = 21`. Shortage does not change the order from 4 to 2; retain original demand in output. Relaxation bounds are finite, not unlimited.

In either language, report transported quantities, unused supply, unmet demand and each cost component separately. Equal costs for different inputs do not establish equivalent business outcomes.


### Inject Bounded Relaxation

`makeSlackProblem()` / `make_slack_problem()` constructs the separate table scenario without the dock restriction. Solve it through the first lesson’s service, then read spills, shortages and cost components.

::: code-group

<<< ../_examples/network-business.kt#slack [Kotlin]

<<< ../_examples/network-business.rs#slack [Rust]

:::

### Call the Standalone Service {#entry-budget}

The same service solves the problem with shared capacity. Pure flow has no column-count or pricing-round option. Configure budgets in the actual linear/integer backend and retain its report with the business result.

::: code-group

<<< ../_examples/network-business.kt#entry [Kotlin]

<<< ../_examples/network-business.rs#entry [Rust]

:::

## 4. Wire a Joint Model Correctly

### Minimal Business Rule: A Shared Dock Capacity Row

Use the existing domain contract rather than inventing a policy interface. Kotlin injects `FlowCapacity` through `FlowProblem.resources`; Rust injects `FlowResource` with `with_resources`. Consumption is one resource unit per normalized commodity unit; add more commodity/service keys to the same row for sharing.

::: code-group

<<< ../_examples/network-business.kt#rule [Kotlin]

<<< ../_examples/network-business.rs#rule [Rust]

:::

Standalone entry points are Kotlin `NetworkFlowApplicationService` and Rust `FlowApplicationService`. For joint network construction, scheduling or shared-budget decisions, register `FlowContext` in the common model and solve at the upper layer.

Shared IDs alone are insufficient: decisions must connect through actual variables, constraints or expressions. Cheapest transport followed by a separate scheduler is generally a decomposed business process, not joint optimality. A decomposition algorithm needs explicit feedback, feasibility repair and termination evidence.

Keep context/variable mappings consistent. Never replay an old vector into a fresh model by stale indices. Isolate request state and do not reuse partially registered state after an error.

## 5. Acceptance and Exercises

1. Increase dock capacity from 1 to 2 to recover hard-demand feasibility; increase to 3 and verify only demand 2 is transported.
2. Retain original demand, add explicit bounded shortage, then tighten its limit below the capacity deficit and verify infeasibility returns.
3. Check conservation, arc bounds, shared resources and slacks per commodity. Recompute operating, unused-supply and shortage costs separately.
4. In a joint model, change the shared budget and verify both domains' decisions respond, rather than returning unrelated outputs.

Whether a shortage plan is deliverable is a business decision, not implied by outer success. Retain shortfalls and true status using the [integration guide](../integration); continue to [Gantt](../gantt/business-scenario) for time and personnel across locations.

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.


## Complete Application Sources

The snippets come from the same source files. Keep imports and helpers, then call `runBusiness` / `run_business` from your application. Constructors can also be used independently for input validation. Reuse first-lesson backend dependencies; these functions do not create an implicit backend.

:::: details Full Kotlin / Rust sources

::: code-group

<<< ../_examples/network-business.kt [Kotlin]

<<< ../_examples/network-business.rs [Rust]

:::

::::
