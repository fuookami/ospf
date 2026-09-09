# Demo5 Route-generation context domain model

## Context and responsibility

Builds the resource graph and prices feasible routes with ESPPRC labels.

## Variables and domains

Label resources are non-negative distance, time, and load values; a generated route is a column candidate.

## Intermediate values

`reducedCost(r)=routeCost(r)-\sum_i dual_i\,cover_i(r)`.

## Assertions and constraints

Labels obey customer visitation, capacity, depot, and time-window extension rules.

## Objective and lifecycle

Return a negative reduced-cost route while pricing; terminate when no improving route exists.

## Source and verification

[Route generation context](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-route-generation-context)

