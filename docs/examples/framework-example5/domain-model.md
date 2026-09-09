# Framework example 5: VRPTW domain model

[中文](../../zh-cn/examples/framework-example5/domain-model.md)

This page is the context-level model contract for the runnable Demo5
application. The full implementation audit is in the
[example overview](../framework-example5); this page keeps the long
branch-and-price model navigable by bounded context.

## 1. Overview and context dependencies

| Context | Responsibility | Dependency |
| --- | --- | --- |
| input infrastructure | Demo17/Solomon parsing, unit conversion, validation, and `VrptwInstance` assembly | external fixture/text |
| VRPTW domain | customers, depots, vehicle types, routes/stops, units, tolerances | none after input normalization |
| policy/calculation | distance, travel time, arc cost, route cost, and arc feasibility | VRPTW domain |
| route compilation | route-column RMP, artificial coverage, customer/fleet expressions, phase pipelines | VRPTW domain and route columns |
| route generation | initial routes, branch-aware graph, ESPPRC labels and pricing result | VRPTW domain, policies, RMP duals |
| application | node solving, phase transitions, branching, bounds, output | compilation and generation |
| solver infrastructure | SCIP/Gurobi column-generation backend selection | application configuration |

The production path is a route-based RMP plus ESPPRC pricing. The direct
two-index MIP in `Demo5DirectMipOracle` is test-only and is not a domain
alternative to route columns.

## 2. Concepts, predicates, and sets

Let `D` be customers, `K` vehicle types, `R_k` routes of type `k`, `N` route
nodes (depots plus customers), and `A` directed feasible arcs. A customer `i`
has demand `q_i`, ready time `ready_i`, due time `due_i`, and service time
`service_i`. A vehicle type `k` has capacity `Q_k`, fixed cost `f_k`, and
available amount `amount_k`.

Predicates include `customer(n)`, `depot(n)`, `feasibleArc(p,q)`,
`contains(i,r)`, `elementary(r)`, and `branchCompatible(r,b)`. Therefore:

$$
R_k^{feas}=\{r\in R_k\mid elementary(r)\wedge capacity(r)\le Q_k
\wedge timeWindows(r)\},
$$

and the branch node's pool is `R_{k,b}^{compatible}` after require/forbid masks
are applied.

## 3. Variables and intermediate values

For a current branch node and column pool:

$$
x_r\ge0\quad(r\in R_k^{compatible}),\qquad
a_i\ge0\quad(i\in D).
$$

`x_r` is route usage in the RMP; integer route selection is checked by the
branch-and-price framework at integral incumbents. `a_i` is artificial phase-I
coverage. Route compilation exposes:

$$
cover_i=\sum_{r\ni i}x_r,
\qquad
fleet_k=\sum_{r\in R_k}x_r,
\qquad
routeCost_r=\sum_{(p,q)\in r}arcCost_{pq}+f_k.
$$

For each route stop `q` after predecessor `p`, the pricing intermediates are:

$$
arrival_q=departure_p+travelTime_{pq},
\qquad
start_q=\max(arrival_q,ready_q),
\qquad
departure_q=start_q+service_q.
$$

Load is accumulated from customer demand and is bounded by `Q_k`.

## 4. Assertions and constraints

The active phase-I/phase-II master constraints are:

$$
s.t.\quad a_i+\sum_{r\ni i}x_r=1\qquad(i\in D),
$$

$$
s.t.\quad \sum_{r\in R_k}x_r\le amount_k\qquad(k\in K).
$$

Every priced route must satisfy:

$$
s.t.\quad start_i\le due_i,
\qquad
s.t.\quad load_r\le Q_k,
\qquad
s.t.\quad r\text{ visits each customer at most once and respects branch masks}.
$$

These are route-generation feasibility assertions rather than extra RMP rows.
Pricing is complete only when the ESPPRC reports no improving route; reaching a
per-call column limit is incomplete pricing and must not be reported as
convergence.

## 5. Objective functions

Phase I minimizes artificial coverage:

$$
\min\sum_{i\in D}a_i.
$$

After all `a_i` are fixed to zero, phase II minimizes route cost:

$$
\min\sum_{k\in K}\sum_{r\in R_k}routeCost_r x_r.
$$

For a route `r` of type `k`, the reduced-cost convention consumed by pricing is:

$$
rc(r)=c^{phase}(r)-\sum_{i\in r\cap D}\pi_i-\mu_k,
$$

where `c^{phase}` is zero for real routes in phase I and normalized route cost
in phase II; `π_i` and `μ_k` are customer and fleet duals.

## 6. Branch-and-price lifecycle

1. Validate and normalize the input instance.
2. Build a branch-node RMP with initial one-customer routes and artificial
   coverage, then solve phase I.
3. Extract duals, build a branch-aware ESPPRC graph, and add deduplicated
   negative-reduced-cost columns.
4. Switch to phase II when artificial coverage converges, fix `a_i=0`, and
   continue column generation.
5. Branch fractional nodes on vehicle/customer assignment and vehicle/edge
   decisions, filter inherited/new routes by branch masks, and validate integer
   routes before assembling the solution.

The application uses a best-bound queue and configured time, node, gap,
pricing-column, and per-node iteration limits. These are termination policies,
not mathematical constraints.

## 7. Ubiquitous language and design decisions

| Term | Symbol | Meaning |
| --- | --- | --- |
| customer | `i` | demand-bearing service stop |
| vehicle type | `k` | capacity, fixed cost, and fleet count |
| route/column | `r` | feasible depot-to-depot customer sequence |
| artificial coverage | `a_i` | phase-I variable used to seed coverage |
| RMP | `x_r` | current route master |
| pricing | `rc(r)` | ESPPRC search for an improving route |
| branch mask | `b` | require/forbid decisions inherited by a node |

| Decision | Alternative | Rationale |
| --- | --- | --- |
| route columns in production | direct arc-indexed MIP | matches the framework's decomposition and scalable pricing |
| artificial phase I | assume initial routes cover all customers | permits controlled infeasibility while columns are generated |
| exact pricing completion flag | treat column-limit return as convergence | prevents false optimality when pricing is truncated |
| typed units and validators | raw doubles throughout | preserves coordinate, time, and load compatibility |

## 8. Implementation boundary

The source-of-truth is `RouteCompilationContext`, `BranchNodeSolver`,
`EspprcPricer`, `BranchAndPriceAlgorithm`, and Demo5 `Application`. The direct
MIP oracle and Rust counterpart are comparison artifacts; they do not change
the Kotlin production model documented here.

