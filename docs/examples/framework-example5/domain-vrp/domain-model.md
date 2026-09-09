# Demo5 VRPTW domain context

## Context and responsibility

Owns customers, vehicles, depots, routes, service windows, units, and validators.

## Variables and domains

Route columns are feasible objects; customer demand and vehicle capacity remain typed input quantities.

## Intermediate values

`travelTime_{ij}=distance_{ij}/speed` and `routeCost(r)=fixedCost(r)+\sum_{(i,j)\in r} arcCost_{ij}`.

## Assertions and constraints

Every route starts and ends at a depot, visits each customer at most once, and respects capacity and time windows.

## Objective and lifecycle

Generation and compilation contexts consume this validated vocabulary.

## Source and verification

[VRP context](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-vrp-context)

