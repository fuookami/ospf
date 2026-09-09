# Demo5 Route-compilation context domain model

## Context and responsibility

Compiles route columns into the master coverage and fleet model.

## Variables and domains

`x_r\in\mathbb Z_{\ge0}` selects route `r`; artificial coverage variables are non-negative phase-I variables.

## Intermediate values

`cover_{i,r}\in\{0,1\}` records customer coverage and `fleet_v=\sum_r use_{v,r}x_r` counts vehicle usage.

## Assertions and constraints

`\sum_r cover_{i,r}x_r=1` for each customer and fleet usage stays within configured limits.

## Objective and lifecycle

Minimizes route cost plus phase-I penalties and receives columns from route generation.

## Source and verification

[Route compilation context](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-route-compilation-context)

