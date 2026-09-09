# Framework Example 1: Shortest Service Path (SSP) — Overview

[中文](../zh-cn/examples/framework-example1)

## 1. Overview

This example composes Route and Bandwidth bounded contexts to assign services to normal nodes and account for bandwidth on directed edges. The current Kotlin pipeline registration is authoritative.

## 2. Context map and dependencies

| Context | Responsibility | Dependency |
|---|---|---|
| Route | Graph, clients, services, assignments, assignment constraints, and service-cost objective | — |
| Bandwidth | Edge/service/node bandwidth expressions and active bandwidth limits | Route |

## 3. Concepts, sets, and predicates

`N` is the node set, `E` the directed edge set, and `S` the service set. `N^C` is the client subset; `E^N` contains edges whose source is a normal node. A valid assignment refers to a graph node and a defined service demand.

## 4. Variables and intermediate values

`x_{n,s}` is the binary assignment variable and `y_{e,s}` is non-negative integer bandwidth. The contexts derive service demand, edge usage, node usage, and service cost. Client assignment rows are fixed according to the current `Assignment` implementation.

## 5. Assertions, constraints, and objective

Active pipelines enforce node assignment, service assignment, edge bandwidth, service capacity, and demand limits. Route registers the service-cost objective. Bandwidth registers edge, demand, service-capacity, and bandwidth-cost pipelines. `TransferNodeBandwidthConstraint` exists in source but is not returned by the current pipeline generator.

## 6. Algorithms and lifecycle

The application initializes Route, passes it to Bandwidth, registers both aggregates, constructs the meta-model, solves with SCIP, and analyzes selected assignments and bandwidth values. No independent flow-conservation equation is introduced by the current application.

## 7. Register → construct → solve → analyze

`init` creates contexts and data; `register` adds variables and pipelines; `construct` builds the solver model; `solve` invokes the configured solver; `analyze` binds results back to domain objects.

## 8. Source and verification

- [Kotlin Demo1 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1)
- [Rust Demo1 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo1.rs)

## 9. Kotlin/Rust comparison and design decisions

Both implementations use the Route/Bandwidth split. Variable domains and active pipeline lists must be checked against each language's source before interpreting a conceptual equation as a registered constraint.

## 10. Context model pages

- [Route context](framework-example1/domain-route/domain-model)
- [Bandwidth context](framework-example1/domain-bandwidth/domain-model)

## 11. Change log

| Version | Change | Reason |
|---|---|---|
| 1.0 | Standardized overview structure and active-pipeline notes | Align overview pages with the context model template |
