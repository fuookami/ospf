# Framework Example 5: VRPTW Branch-and-Price — Overview

[中文](../zh-cn/examples/framework-example5)

## 1. Overview

Demo5 is the runnable VRPTW integration. It builds a route-based restricted master, prices elementary resource-constrained routes, and applies branch-and-price through the network-scheduling framework.

## 2. Context map and dependencies

VRP owns customers, vehicles, routes, units, and validators. Route generation consumes the VRP vocabulary and master dual prices; route compilation owns customer coverage, fleet, artificial coverage, and route-cost pipelines.

## 3. Concepts, sets, and predicates

`C` is the customer set, `V` the vehicle-type set, `R` the route-column set, and `A` the directed arc set. Predicates classify feasible routes, customer coverage, capacity-feasible labels, and improving pricing results.

## 4. Variables and intermediate values

Compilation selects route columns `x_r` and Phase-I artificial coverage `a_i`. Generation derives label load, arrival time, route cost, and reduced cost. VRP supplies demand, service windows, capacity, and policy-specific distance/cost values.

## 5. Assertions, constraints, and objective

Routes begin/end at a depot and satisfy visitation, capacity, and time-window rules. The master enforces customer coverage and fleet limits, penalizes artificial coverage in Phase I, and minimizes route cost in Phase II.

## 6. Algorithms and lifecycle

The application initializes the instance and policies, builds initial routes, solves the restricted master, prices ESPPRC routes, adds improving columns, and branches until the configured termination condition is met.

## 7. Register → construct → solve → analyze

VRP validates input; route generation builds the pricing graph; route compilation registers the master; branch-and-price alternates master and pricing solves; analysis returns selected routes and customer/fleet results.

## 8. Source and verification

- [Kotlin Demo5 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo5)
- [Rust Demo5 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo5.rs)
- [Kotlin network-scheduling contexts](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling)

## 9. Kotlin/Rust comparison and design decisions

The route/master vocabulary is shared, while adapters, numeric domains, cost policies, and branching details are language-specific. The context pages identify the registered mathematical boundary.

## 10. Context model pages

- [VRP context](framework-example5/domain-vrp/domain-model)
- [Route generation context](framework-example5/domain-route-generation/domain-model)
- [Route compilation context](framework-example5/domain-route-compilation/domain-model)

## 11. Change log

| Version | Change | Reason |
|---|---|---|
| 1.0 | Standardized VRPTW overview structure | Align the overview with the other complex examples |
