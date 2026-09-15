# Framework Example 1: Service Placement — Overview

[中文](/zh-cn/examples/framework-example1)

## 1. Overview

This example uses Route and Bandwidth bounded contexts to model service placement and network bandwidth allocation. The overview explains collaboration; the child pages define the variables, intermediate expressions, and constraints registered by the Kotlin example.

## 2. Contexts and Dependencies

| Context | Responsibility | Dependency |
|---|---|---|
| Route | Graph, clients, services, assignments, assignment constraints, and service-cost objective | — |
| Bandwidth | Edge/service/node bandwidth expressions and active bandwidth limits | Route |

## 3. Concepts, Sets, and Predicates

$N$ is the node set, $E$ the directed edge set, and $S$ the service set. $N^{normal}$ and $N^{client}$ denote normal and client nodes; $E^{normal}$ contains edges with a normal source. These symbols match the context pages.

## 4. Variables and Intermediate Values

$x_{n,s}$ is binary service assignment, with client rows fixed to zero. $y_{e,s}$ is bounded nonnegative integer bandwidth. Service assignment count $A_s$, node assignment count $A_n$, edge bandwidth $B_e$, and node incoming bandwidth $I_n$ are linear intermediate expressions; customer demand is input data.

## 5. Assertions, Constraints, and Objectives

Route limits each normal node to at most one service and each service to at most one placement, and registers service cost. Bandwidth registers edge bandwidth, customer demand, service capacity, and bandwidth cost. The omitted `TransferNodeBandwidthConstraint` is not part of the registered model. Net-outflow definitions do not imply flow-conservation constraints.

## 6. Algorithms and Lifecycle

The application initializes Route, passes it to Bandwidth, registers both aggregates, constructs the meta-model, solves with SCIP, and analyzes selected assignments and bandwidth values. No independent flow-conservation equation is introduced by the current application.

## 7. Register → Construct → Solve → Analyze

`init` creates contexts and data; `register` adds variables and pipelines; `construct` builds the solver model; `solve` invokes the configured solver; `analyze` binds results back to domain objects.

## 8. Source Entry Points

- [Kotlin Demo1 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1)
- [Rust Demo1 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo1)

## 9. Kotlin/Rust Comparison and Design Decisions

Both Kotlin and Rust framework examples have Route and Bandwidth contexts. The links below target framework examples, not the same-numbered simple examples. The context pages use Kotlin registration as their mathematical reference and do not infer additional constraints from the problem name.

## 10. Context Model Pages

- [Route context](framework-example1/domain-route/domain-model)
- [Bandwidth context](framework-example1/domain-bandwidth/domain-model)

## 11. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual overviews, notation, and source entry points | Keep the overview consistent with its context models |
