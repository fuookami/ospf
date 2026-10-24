# Shared Resources, Shortage and Embedding

After [your first network flow](./first-flow), add a non-arc capacity before deciding whether shortage is allowed. Connectivity and adequate arc capacity do not establish business feasibility.

## 1. A Shared Handling Resource Can Make Flow Infeasible

Node a supplies 2 kg and b demands 2 kg, with one a→b arc of capacity 3 kg. Add shared resource dock with capacity 1 kg, consuming 1 kg of dock capacity per transported kilogram.

The arc can carry demand but dock cannot. Another arc does not necessarily help if it consumes the same dock. Feasibility requires more resource capacity, different actual consumption or explicitly authorized demand relaxation.

Start from the first-lesson flow problem, retain balances and add only the shared resource row. Solve again and verify flows and resource consumption instead of filling a witness vector into an extractor.

## 2. Convert Resource Units Across Commodities

| Object | Kotlin | Rust |
| --- | --- | --- |
| Shared resource | `FlowCapacity` | `FlowResource` |
| Commodity-arc usage | usage keyed by `FlowKey` | commodity/arc keys in consumptions |
| Allowed services | `allowedArcs` | `allowed_arcs` |
| Elastic balances | `FlowSlack` | `FlowBalanceSlack` |

Commodities measured in kg, m or pieces need resource-unit/commodity-unit conversion coefficients before summation. Normalized integer steps are not physical usage. Distinct parallel-service IDs must remain distinct.

## 3. Shortage Needs Bounds and Costs

Construct a separate spill/shortage scenario using one normalized unit:

| Condition | Value |
| --- | --- |
| Supply at a / demand at b | 3 / 4 |
| Arc capacity / transport price | 2 / 1 |
| Allowed unused supply at a / penalty | At most 1 / 5 |
| Allowed unmet demand at b / penalty | At most 2 / 7 |

Flow 2, unused supply 1 and shortage 2 cost `2×1 + 1×5 + 2×7 = 21`. Shortage does not change the order from 4 to 2; retain original demand in output. Relaxation bounds are finite, not unlimited.

In either language, report transported quantities, unused supply, unmet demand and each cost component separately. Equal costs for different inputs do not establish equivalent business outcomes.

## 4. Wire a Joint Model Correctly

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
