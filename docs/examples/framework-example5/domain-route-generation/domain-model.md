# Route generation context model

## 1. Overview

Route generation searches elementary routes satisfying resource and branch rules, then returns improving columns to [route compilation](../domain-route-compilation/domain-model). Pricing is an elementary shortest-path problem with resource constraints (ESPPRC).

### 1. Dependent Contexts

The [VRP context](../domain-vrp/domain-model) supplies customers, vehicle types, resources, and calculation policies. Compilation supplies the phase and customer/fleet duals. The application supplies branch rules.

## 2. Concepts / Entities

### 1. Labels

A label records the current node, visited customers, accumulated load, service time, and pricing cost. It is search state, not a solver variable.

### 2. Routes and Prices

**$c_r$**: Actual cost of route $r$.

**$\pi_i$**: Dual price of customer $i$'s coverage equality.

**$\mu_v$**: Dual price of the fleet upper-bound row for type $v$. The equations use the signed dual of the minimization master row, not its absolute value.

**$\varepsilon_{rc}$**: Nonnegative tolerance used to identify negative reduced costs.

## 3. Variables

### 1. Decision Variables

This context does not duplicate master route usage $x_r$. Pricing returns a candidate route $r$.

### 2. Auxiliary Variables

Label state $(n,S,L,t,\gamma)$ records the current node, visited customers, load, service-start time, and accumulated pricing cost. These are algorithm states, not auxiliary master variables.

## 4. Predicates

**$\operatorname{extendable}(\ell,j)$**: Label $\ell$ can extend to node $j$.

**$\operatorname{compatible}(r,b)$**: Route $r$ satisfies required and forbidden decisions at branch node $b$.

**$\operatorname{improving}(r)$**: Reduced cost is below $-\varepsilon_{rc}$.

**$\operatorname{complete}$**: Pricing has searched sufficiently to certify that no improving route remains. Merely reaching a returned-column limit does not establish this.

## 5. Sets

**$C$**: Customers; **$V$**: vehicle types; **$N$**: customer and depot nodes.

**$A_v$**: Feasible arcs for type $v$; **$\mathcal L_v$**: its search labels.

**$\mathcal R_b^v$**: All routes of type $v$ satisfying resource and branch-node $b$ rules, distinct from the finite pool already inserted into the master.

**$\mathcal R_b^{-}$**: Negative-reduced-cost routes found by pricing.

## 6. Intermediate Values

### 1. Resource Recurrence

For an extension from label $\ell=(i,S,L,t,\gamma)$ to unvisited customer $j$, demand, service duration, and travel time are $q_j,s_i,\tau^v_{ij}$:

$$
L'=L+q_j,\qquad
t'=\max\{e_j,t+s_i+\tau^v_{ij}\},\qquad
S'=S\cup\{j\}.
$$

$t'$ is service-start time, not arrival before waiting.

### 2. Reduced Cost

The route's phase-dependent objective coefficient is:

$$
c_r^{phase}=
\begin{cases}
0,&\text{phase I},\\
c_r,&\text{phase II}.
\end{cases}
$$

A route of vehicle type $v$ must include both customer and fleet dual contributions:

$$
\bar c_r=c_r^{phase}-\sum_{i\in C_r}\pi_i-\mu_v,
\qquad r\in\mathcal R_b^v.
$$

Costs and duals must use the same numerical normalization. Branch decisions restrict the search through route compatibility.

## 7. Assertions

A successful extension must preserve elementarity and resource feasibility:

$$
\operatorname{extendable}(\ell,j)\Rightarrow
j\notin S\ \wedge\ (i,j)\in A_v\ \wedge\
L'\le Q_v\ \wedge\ t'\le l_j.
$$

This implication states what a permitted extension must satisfy. It does not require every label to extend to every customer.

## 8. Constraints

### 1. Route Feasibility [路线可行性]

**Description**: Pricing searches depot-to-depot routes without repeated customers, subject to capacity, time windows, and branch rules.

$$
r\in\mathcal R_b^v\Rightarrow
\operatorname{elementary}(r)\wedge
L_r\le Q_v\wedge
\operatorname{timeFeasible}(r)\wedge
\operatorname{compatible}(r,b).
$$

These are label-extension, filtering, and completed-route validation conditions, not a second family of arc variables and master rows.

### 2. Improving Column Filter [改进列筛选]

$$
\mathcal R_b^{-}=
\{r\text{ found by pricing}:\bar c_r<-\varepsilon_{rc}\}.
$$

## 9. Objective Function (if applicable)

**Description**: Search permitted vehicle types for a route with minimum phase-dependent reduced cost.

$$
\min_{v\in V,\ r\in\mathcal R_b^v}\bar c_r.
$$

Only complete pricing that finds no negative-reduced-cost route supports a column-generation convergence claim. Early return due to time, column, or search limits does not establish that optimality conclusion.

## 10. Algorithm References

| Algorithm | Referenced In | Description |
|---|---|---|
| Initial route generation | Master initialization | Supplies seed routes; artificial coverage initializes feasibility |
| ESPPRC label extension and dominance | Sections 6–8 | Maintains resources, visited sets, and reduced costs; prunes dominated states |
| Branch-compatibility filtering | Sections 4, 8 | Applies required/forbidden decisions to the graph and candidates |
| Pricing completion reporting | Section 9 | Distinguishes complete search from a limit-triggered return |

[Kotlin route-generation source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-route-generation-context); [Kotlin/Rust example entry points](/examples/framework-example5).

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|---|---|---|
| Label | $\ell$ | Search state for a partial route |
| Visited set | $S$ | Customers already served by a label |
| Reduced cost | $\bar c_r$ | Phase coefficient minus corresponding row-dual contributions |
| Fleet dual | $\mu_v$ | Dual value of the vehicle-type availability row |
| Pricing complete | $\operatorname{complete}$ | Search status allowing a reliable improving-column conclusion |

## 12. Design Decisions

| Decision | Alternative | Rationale |
|---|---|---|
| ESPPRC label search | Enumerate all routes | Searches incrementally under resources and elementarity |
| Include customer and fleet duals | Subtract customer duals only | Matches all base restricted-master rows |
| Explicit completion status | Treat every return as completed search | Avoids reporting truncated search as convergence |

## 13. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual resource recurrences, phase prices, and pricing boundaries | Restored fleet duals and separated algorithm state from model variables |
