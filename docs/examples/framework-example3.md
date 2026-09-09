# Framework Example 3: One-Dimensional Cutting Stock — Overview

[中文](../zh-cn/examples/framework-example3)

## 1. Overview

This example uses the csp1d framework API for a restricted master problem and column generation. Material and Produce are the core domain contexts; pricing and objective policies extend them.

## 2. Context map and dependencies

`material → produce → cutting-plan-generation`; length assignment and wasting minimization contribute optional constraints and objective terms. The application client coordinates registration and the column-generation loop.

## 3. Concepts, sets, and predicates

`P` is the product set, `M` the material set, `E` the machine/resource set, and `K` the active cutting-plan columns. Predicates identify demanded products, feasible plans, active columns, and products using a material.

## 4. Variables and intermediate values

Production quantities are integer non-negative `x_p`; RMP column variables are `lambda_k`. Intermediate values include material usage `u_m`, machine hours `H_e`, capacity `C_e`, demand contribution `a_{pk}`, and waste `w_k`.

## 5. Assertions, constraints, and objective

Active columns cover demand, machine hours stay within capacity, plan waste is non-negative, and optional length rules are respected. The RMP minimizes production/material/waste costs registered by the current Produce and wasting-minimization pipelines.

## 6. Algorithms and lifecycle

The client creates initial feasible plans, registers the RMP, solves it, reads dual prices, prices new plans, adds improving columns, and stops when no negative reduced-cost plan remains.

## 7. Register → construct → solve → analyze

Context builders register variables and pipelines; the framework constructs the RMP and pricing model; the solver alternates restricted-master and pricing iterations; analysis reports production and waste.

## 8. Source and verification

- [Kotlin Demo3 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo3)
- [Rust Demo3 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo3.rs)

## 9. Kotlin/Rust comparison and design decisions

Both versions expose the same column-generation vocabulary, while coefficient names, numeric domains, and active objective wrappers must be verified from language-specific source.

## 10. Context model pages

- [Material context](framework-example3/domain-material/domain-model)
- [Produce context](framework-example3/domain-produce/domain-model)

## 11. Change log

| Version | Change | Reason |
|---|---|---|
| 1.0 | Standardized overview structure and RMP lifecycle | Align the overview with context pages |

