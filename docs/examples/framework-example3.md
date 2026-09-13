# Framework Example 3: One-Dimensional Cutting Stock — Overview

[中文](/zh-cn/examples/framework-example3)

## 1. Overview

This example uses the one-dimensional cutting-stock (CSP1D) framework to separate product/material data, the cutting-plan master, and pricing. The overview explains column generation; the material and produce pages define the data contract and master model separately.

## 2. Contexts and Dependencies

| Context | Responsibility | Dependency |
|---|---|---|
| Material | Products, demand, materials, and cutting-plan data | Input configuration |
| Produce | Plan-usage variables, yield/resource expressions, and master pipelines | Material, generated plans |
| Cutting-plan generation | Initial plans and improving-column search | Material, master duals |
| Length assignment and wasting minimization | Optional length rules and loss-related objectives | Material, produce, configuration |

The application assembles contexts and controls column generation. Algorithm roles must not be treated as a second independent production-variable family.

## 3. Concepts, Sets, and Predicates

$P$ is the product set, $M$ the material set, and $J_t$ the cutting plans available at master solve $t$. Product demand is $d_p$ and plan yield contribution is $a_{pj}$. Predicates distinguish feasible plans, inserted columns, and products with optional length rules.

## 4. Variables and Intermediate Values

$x_j$ is the usage count of plan $j$, not the production quantity of product $p$. Product yield is $q_p=\sum_{j\in J_t}a_{pj}x_j$. Each plan's remaining width is a plan coefficient; total remaining width is its usage-weighted sum. Global product output must not be substituted into every plan's waste calculation.

## 5. Assertions, Constraints, and Objectives

Demand is imposed per product as $q_p\ge d_p$. Configured slack and resource limits follow their owning pipelines; optional rules are not automatically active in the small example. Master variables are continuous during column-generation LP solves and nonnegative integers during integer solving.

## 6. Algorithms and Lifecycle

Generate initial plans, solve the restricted master, extract dual prices, search for negative-reduced-cost plans, deduplicate and insert columns, and solve again. Termination must distinguish complete pricing with no improving columns from an early return caused by time or iteration limits.

## 7. Register → Construct → Solve → Analyze

Registration creates the produce aggregate and its pipelines. Construction compiles available plans into master columns. Solving alternates the master and pricing. Analysis converts plan usage into product yields and material usage.

## 8. Source Entry Points

- [Kotlin Demo3 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo3)
- [Rust Demo3 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo3)

## 9. Kotlin/Rust Comparison and Design Decisions

Both language entry points call their CSP1D frameworks rather than reimplement every context inside the Demo directory. Mathematical notation consistently indexes variables by plan $j$, so language-level naming differences do not imply different models.

## 10. Context Model Pages

- [Material context](framework-example3/domain-material/domain-model)
- [Produce context](framework-example3/domain-produce/domain-model)

## 11. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual overviews, notation, and source entry points | Keep the overview consistent with its context models |
