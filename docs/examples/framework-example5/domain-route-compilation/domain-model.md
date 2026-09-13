# Route compilation context model

## 1. Overview

Route compilation turns feasible routes into a restricted master problem (RMP). It owns route-usage variables, artificial coverage, customer rows, fleet rows, and phase objectives.

### 1. Dependent Contexts

The [VRP context](../domain-vrp/domain-model) supplies customer, vehicle-type, and cost semantics. [Route generation](../domain-route-generation/domain-model) supplies feasible columns. The application controls branch nodes, column-generation iterations, and phase transitions.

## 2. Concepts / Entities

### 1. Customers and Vehicle Types

**$m_v$**: Available vehicles of type $v$; each route uses one vehicle of its type.

### 2. Route Columns

**$a_{ir}\in\{0,1\}$**: Whether route $r$ covers customer $i$.

**$c_r$**: Route cost, converted to a common cost unit before compilation.

### 3. Branch Nodes and Phases

Branch node $b$ restricts permissible routes; iteration $t$ restricts the generated pool. Phase I eliminates artificial coverage; phase II optimizes route cost.

## 3. Variables

### 1. Decision Variables

**$x_r$**: Dimensionless route usage. The node LP uses $x_r\in\mathbb R_{\ge0}$ for every $r\in R_{b,t}$; an integer solution requires $x_r\in\mathbb Z_{\ge0}$. Not every RMP solve is an integer program.

### 2. Auxiliary Variables

**$u_i$**: Dimensionless artificial coverage, $u_i\in\mathbb R_{\ge0}$ for every $i\in C$. It initializes solvable phase-I coverage rows, not a business decision to omit customers in the final solution. Phase II fixes $u_i=0$.

### 3. Kotlin/Rust Registration Forms

Kotlin registers routes with `UIntVariable1`; the column-generation backend explicitly applies `linearRelax()` for LP solving. Artificial coverage uses `URealVariable1`. Rust directly registers route and artificial variables as continuous values in $[0,1]$. The nonnegative mathematical domains above therefore do not assert identical registration types or default numerical upper bounds.

Coverage equalities and nonnegativity imply $u_i\le1$ and, for any route covering at least one customer, $x_r\le1$. These are consequences of model rows, not Kotlin's declared default upper bounds. Branch-and-price checks integer route usage in final solutions.

## 4. Predicates

**$\operatorname{covers}(r,i)$**: Route $r$ covers customer $i$.

**$\operatorname{type}(r,v)$**: Route $r$ uses vehicle type $v$.

**$\operatorname{compatible}(r,b)$**: Route $r$ respects node $b$'s required and forbidden decisions.

**$\operatorname{active}(r,t)$**: Route $r$ was inserted before master solve $t$.

## 5. Sets

**$C$**: Customers; **$V$**: vehicle types.

**$R_{b,t}$**: Inserted, feasible, branch-compatible routes at node $b$, iteration $t$.

**$R_{b,t}^{v}$**: Routes in $R_{b,t}$ using vehicle type $v$.

All following equations fix $b,t$. Adding columns updates these summation domains.

## 6. Intermediate Values

### 1. Customer Coverage

**Description**: Coverage is route usage weighted by customer-coverage coefficients.

$$
Y_i=\sum_{r\in R_{b,t}}a_{ir}x_r,\qquad \forall i\in C.
$$

### 2. Fleet Usage

**Description**: Each route consumes one vehicle of its type.

$$
F_v=\sum_{r\in R_{b,t}^{v}}x_r,\qquad \forall v\in V.
$$

### 3. Total Route Cost

$$
Z=\sum_{r\in R_{b,t}}c_rx_r.
$$

$Y_i,F_v,Z$ are linear expressions, not independent decision variables.

## 7. Assertions

Every column must pass route validation and respect the branch node:

$$
\forall r\in R_{b,t}:\quad
\operatorname{feasible}(r)\wedge\operatorname{compatible}(r,b).
$$

Each available route has one vehicle type and visits no customer twice:

$$
\forall r\in R_{b,t}:\quad
\exists!v\in V:\operatorname{type}(r,v),\qquad
\forall i\in C:\ a_{ir}\in\{0,1\}.
$$

## 8. Constraints

### 1. Customer Coverage [客户覆盖]

**Description**: Real routes or phase-I artificial coverage must cover each customer exactly once.

$$
s.t.\quad Y_i+u_i=1,\qquad \forall i\in C.
$$

### 2. Fleet Limit [车队数量]

**Description**: Selected routes must not consume more vehicles than are available.

$$
s.t.\quad F_v\le m_v,\qquad \forall v\in V.
$$

### 3. Phase-II Artificial Fixing [第二阶段人工量固定]

**Description**: Artificial coverage cannot substitute for real service during cost optimization.

$$
s.t.\quad u_i=0,\qquad \forall i\in C.
$$

**Corollary**: Coverage becomes $Y_i=1$. Capacity, time windows, and elementarity are already guaranteed by column feasibility; they must not be invented as additional arc-level rows in this context.

## 9. Objective Function (if applicable)

### 1. Phase I

**Description**: Minimize artificial coverage not supplied by real routes.

$$
\min\quad Z^{I}=\sum_{i\in C}u_i.
$$

### 2. Phase II

**Description**: Fix all artificial coverage to zero, then minimize real route cost.

$$
\min\quad Z^{II}=\sum_{r\in R_{b,t}}c_rx_r.
$$

These are two phases, not a single objective $M\sum_i u_i+Z$ using an unspecified large constant $M$.

## 10. Algorithm References

| Algorithm | Referenced In | Description |
|---|---|---|
| Restricted master solve | Sections 3, 8, 9 | Solves a node LP and extracts customer and fleet duals |
| Column insertion | Sections 5, 6 | Updates variables and coverage, fleet, and objective coefficients |
| Phase transition | Sections 8, 9 | Fixes eliminated artificial coverage and switches objectives |
| Branch-and-price | Application | Handles fractional solutions, node bounds, and integer incumbents |

[Kotlin route-compilation source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-route-compilation-context); [Kotlin/Rust example entry points](/examples/framework-example5).

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|---|---|---|
| Route usage | $x_r$ | Value of a route column in the master |
| Artificial coverage | $u_i$ | Temporary phase-I customer coverage |
| Coverage | $Y_i$ | Customer coverage supplied by real routes |
| Fleet usage | $F_v$ | Vehicles used for a specified type |
| Restricted master | $R_{b,t}$ | Master containing only the node's available routes |

## 12. Design Decisions

| Decision | Alternative | Rationale |
|---|---|---|
| Distinguish LP and integer domains | Declare integer variables in every phase | Dual pricing requires the node LP |
| Separate feasibility and cost phases | Mix objectives with an unspecified large constant | Makes feasibility restoration distinct from cost optimization |
| Filter routes by branch node | Share unfiltered columns across nodes | Enforces branch rules in both master and pricing |

## 13. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual domains, coverage rows, and phase objectives | Removed contradictions between the split page and the overall model |
