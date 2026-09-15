# Framework Example 5: VRPTW Branch-and-Price — Overview

[中文](/zh-cn/examples/framework-example5)

## 1. Overview

Demo5 is the runnable VRPTW integration. It builds a route-based restricted master, prices elementary resource-constrained routes, and applies branch-and-price through the network-scheduling framework.

## 2. Contexts and Dependencies

| Context | Responsibility | Dependency |
|---|---|---|
| VRP | Customers, vehicle types, routes, resources, and validation | Normalized input and calculation policies |
| Route generation | Resource-constrained elementary-route pricing | VRP, dual prices, branch rules |
| Route compilation | Customer/fleet rows, artificial coverage, and phase objectives | VRP, generated routes |

The application coordinates branch nodes and phase transitions. Generation and compilation exchange route columns and dual prices.

## 3. Concepts, Sets, and Predicates

$C$ is the customer set, $V$ the vehicle-type set, and $R_{b,t}$ the inserted compatible routes at branch node $b$, iteration $t$. Predicates express customer coverage, elementarity, resource feasibility, and branch compatibility.

## 4. Variables and Intermediate Values

$x_r$ is route usage and $u_i$ is phase-I artificial coverage. The master derives customer coverage $Y_i$ and fleet usage $F_v$. Generation computes label load, service-start time, and reduced cost. The child pages distinguish input attributes, algorithm state, linear expressions, and solver variables.

## 5. Assertions, Constraints, and Objectives

Customer coverage satisfies $Y_i+u_i=1$ and fleet usage cannot exceed availability. Phase I minimizes artificial coverage; phase II fixes $u_i=0$ and minimizes route cost. This is not a mixed objective using an arbitrary large constant. Pricing and validation enforce route capacity and time windows.

## 6. Algorithms and Lifecycle

The application seeds routes, solves node LPs, searches improving columns with ESPPRC, and handles fractional solutions and branch nodes. Pricing includes customer and fleet duals. A return constrained by a column limit is not automatically complete pricing.

## 7. Register → Construct → Solve → Analyze

VRP validates and normalizes input. Generation builds the pricing graph. Compilation registers the restricted master. Branch-and-price coordinates master solves, pricing, and phase transitions. Analysis turns integer route usage into a validated solution.

## 8. Source Entry Points

- [Kotlin Demo5 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo5)
- [Rust Demo5 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo5)

## 9. Kotlin/Rust Comparison and Design Decisions

Both framework examples use route columns. The equations distinguish node LPs from integer solutions and phase-I feasibility restoration from phase-II cost optimization. A direct arc-model test oracle does not replace the route-column production master.

## 10. Context Model Pages

- [VRP context](framework-example5/domain-vrp/domain-model)
- [Route generation context](framework-example5/domain-route-generation/domain-model)
- [Route compilation context](framework-example5/domain-route-compilation/domain-model)

## 11. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual overviews, notation, and source entry points | Keep the overview consistent with its context models |
