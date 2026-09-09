# Demo3 Material context domain model

## Context and responsibility

Owns product demand, material availability, and material usage coefficients.

## Variables and domains

Consumes production variables `x_p\ge0`; the context does not create a duplicate master variable.

## Intermediate values

`use_m=\sum_p usage_{m,p}x_p` is material consumption for material `m`.

## Assertions and constraints

`use_m\le available_m` and every demand predicate is evaluated over the active product set.

## Objective and lifecycle

Material pipelines are registered by the produce context and participate in the restricted master.

## Source and verification

[CSP1D material context](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-csp1d/csp1d-domain-material-context)

