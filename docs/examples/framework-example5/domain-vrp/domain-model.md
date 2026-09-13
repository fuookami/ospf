# VRP context model

## 1. Overview

The VRP context defines customers, depots, vehicle types, routes, and resource semantics shared by [route generation](../domain-route-generation/domain-model) and [route compilation](../domain-route-compilation/domain-model). It validates data and routes; it does not build the restricted master problem.

### 1. Dependent Contexts

Input adapters supply a unit-normalized instance. Calculation policies supply distance, travel time, and cost.

## 2. Concepts / Entities

### 1. Customers and Depots

**$q_i$**: Demand of customer $i$, measured in a common load unit.

**$[e_i,l_i]$**: Permitted service-start window of customer $i$; **$s_i$** is its service duration.

Depots define route endpoints and applicable time ranges. They are not customers requiring coverage.

### 2. Vehicle Types and Routes

**$Q_v$**: Capacity of vehicle type $v$; **$m_v$** is the available vehicle count; **$f_v$** is the fixed cost of using one vehicle.

Route $r$ is a depot-to-depot visit sequence for a specified vehicle type. **$a_{ir}$** records whether it visits customer $i$.

## 3. Variables

### 1. Decision Variables

This context declares no solver decision variables. Route usage $x_r$ belongs to route compilation.

### 2. Auxiliary Variables

This context declares no auxiliary solver variables. Route load, service times, and cost are domain calculations, not additional master variables.

## 4. Predicates

**$\operatorname{customer}(n)$**: Node $n$ is a customer.

**$\operatorname{depot}(n)$**: Node $n$ is a depot.

**$\operatorname{visits}(r,i)$**: Route $r$ visits customer $i$.

**$\operatorname{elementary}(r)$**: Route $r$ visits no customer more than once.

## 5. Sets

**$C$**: Customers; **$V$**: vehicle types; **$N$**: customer and depot nodes.

**$A_v$**: Directed arcs permitted for vehicle type $v$.

**$R_v$**: Feasible routes of type $v$; **$R$** is their union.

**$C_r$**: Customers visited by route $r$; **$A_r$**: its ordered traversed arcs.

## 6. Intermediate Values

### 1. Load and Coverage Coefficients

Route load is the total demand of its visited customers. Coverage coefficients are route data, not binary master variables.

$$
L_r=\sum_{i\in C_r}q_i,\qquad
a_{ir}=
\begin{cases}
1,&i\in C_r,\\
0,&i\notin C_r.
\end{cases}
$$

### 2. Arrival, Waiting, and Service

Given travel time $\tau^v_{ij}$ for vehicle type $v$, arrival, service start, and departure at node $j$ after node $i$ are:

$$
t^{arr}_j=t^{dep}_i+\tau^v_{ij},\qquad
t^{start}_j=\max\{t^{arr}_j,e_j\},\qquad
t^{dep}_j=t^{start}_j+s_j.
$$

Early arrival permits waiting. A service-start window must not be interpreted as a prohibition on early arrival.

### 3. Route Cost

Route cost combines fixed cost and arc costs. Distance need not equal travel time or cost.

$$
c_r=f_v+\sum_{(i,j)\in A_r}c^v_{ij},\qquad r\in R_v.
$$

## 7. Assertions

Inputs and policies must preserve compatible units, nonnegative demand, and ordered time windows:

$$
\forall i\in C:\quad q_i\ge0\ \wedge\ e_i\le l_i\ \wedge\ s_i\ge0.
$$

Let $n_k(r)$ denote the $k$th node of route $r$. An elementary route visits each customer at most once:

$$
\forall r\in R,\ \forall i\in C:\quad
\#\{k:n_k(r)=i\}\le1.
$$

## 8. Constraints

### 1. Route Resource Feasibility [路线资源可行性]

**Description**: Routes must satisfy capacity and service-start windows, departing from and returning to permitted depots.

$$
L_r\le Q_v,\qquad
e_i\le t^{start}_i\le l_i,\qquad
\forall r\in R_v,\ \forall i\in C_r.
$$

These are route-construction and validation rules, not additional rows registered in the restricted master by this context. Cross-route customer coverage and fleet limits belong to route compilation.

## 9. Objective Function (if applicable)

This context has no independent optimization objective. It supplies route cost $c_r$ for phase II of route compilation. Route generation uses the phase objective and dual prices to calculate reduced cost.

## 10. Algorithm References

| Algorithm | Referenced In | Description |
|---|---|---|
| Route validation | Input and result analysis | Checks depots, visits, capacity, service times, and units |
| Route resource recurrence | Section 6 | Accumulates load and computes waiting and service in visit order |
| Calculation policies | Section 6 | Supply distance, travel time, and cost |

[Kotlin VRP context source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-vrp-context); [Kotlin/Rust example entry points](/examples/framework-example5).

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|---|---|---|
| Customer | $i\in C$ | Demand node requiring service |
| Vehicle type | $v\in V$ | Capacity, fixed cost, and available count |
| Route | $r\in R_v$ | Feasible visit sequence for a vehicle type |
| Service start | $t^{start}_i$ | Time service starts after arrival and waiting |
| Coverage coefficient | $a_{ir}$ | Whether a route visits a customer |

## 12. Design Decisions

| Decision | Alternative | Rationale |
|---|---|---|
| Treat routes as domain objects | Redefine arc variables in every context | Shares semantics across generation, compilation, and validation |
| Separate arrival from service start | Disallow arrival before the ready time | Models waiting correctly |
| Separate resource validation from master rows | Expand route resources again in the master | Preserves route-column decomposition |

## 13. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual structure, time semantics, and variable ownership | Avoid treating domain attributes as solver variables |
