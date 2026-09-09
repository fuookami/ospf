# Demo3 Produce context domain model

## Context and responsibility

Owns production quantities, machine batches, capacity, waste, and the master objective.

## Variables and domains

`x_p\in\mathbb Z_{\ge0}` is production quantity; generated cutting plans use pattern variables in the restricted master.

## Intermediate values

`capacity_e=amount_e\,hours^{max}_e` and `waste_k=capacity_k-\sum_p width_{k,p}x_p`.

## Assertions and constraints

Demand coverage, machine capacity, and non-negative waste are enforced for each active plan.

## Objective and lifecycle

The context builds the RMP, accepts columns from pricing, and minimizes production plus waste cost.

## Source and verification

[CSP1D produce context](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-csp1d/csp1d-domain-produce-context)

