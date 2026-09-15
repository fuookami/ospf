# Using DDD Architecture with Column Generation

Column generation separates “choose from known columns” from “discover valuable new columns.” DDD adds another requirement: express that algorithmic boundary in the domain language. The master must not receive anonymous coefficient arrays, and pricing must not read solver row numbers. They communicate through a **shadow-price protocol** and **domain column objects**.

> Column generation solves the LP relaxation over the complete column set. It becomes Branch-and-Price only when it is embedded in a branch-and-bound tree to establish integer optimality.

## 1. Mathematical structure

Let $I$ be the master-row set and $\mathcal P$ a huge set of feasible columns that cannot be enumerated in advance. Column $p$ has cost $c_p$ and coefficient $a_{ip}$ in row $i$; $\lambda_p$ selects the column. The following is a minimization covering model:

$$
\begin{aligned}
\min_{\lambda}\quad
& \sum_{p \in \mathcal P} c_p \lambda_p \\
\text{s.t.}\quad
& \sum_{p \in \mathcal P} a_{ip}\lambda_p \ge b_i,
&& \forall i \in I, \\
& \lambda_p \ge 0,
&& \forall p \in \mathcal P.
\end{aligned}
$$

The restricted master problem (RMP) contains only $\mathcal P' \subset \mathcal P$. If $\pi_i$ is the dual value of covering row $i$, the reduced cost of a candidate column is:

$$
\bar c_p
= c_p - \sum_{i \in I}\pi_i a_{ip}.
$$

The pricing problem computes:

$$
\bar c^\star
= \min_{p \in \mathcal P}
\left(c_p-\sum_{i \in I}\pi_i a_{ip}\right).
$$

Under this convention, add a column when $\bar c^\star < -\varepsilon_{\mathrm{rc}}$. If exact pricing proves that no such column exists, the current RMP has reached the optimum of the complete LP relaxation. Different row senses change the sign domain of dual variables, not the definition of reduced cost.

## 2. Context boundaries

Split column generation into three responsibilities:

| Context/service | Owns | Receives | Publishes |
|---|---|---|---|
| Generation / Pricing | Domain definition of a feasible column, resource state, search graph, labels, and dominance | `ShadowPriceMap` and iteration data | New domain columns and their cost |
| Compilation / Master | Column variables, covering/balance rows, objective terms, and dual mapping | Initial and incremental columns | Master solution, bound, and `ShadowPriceMap` |
| Selection / Application | Iteration, stopping, deduplication, integerization, and diagnostics | Results from both sides | Final domain solution and solve trace |

In flight recovery, for example:

- `FlightTaskBunch` is a column, not a raw vector;
- `bunch_generation` creates feasible flight strings from the flight graph, rules, and resources;
- `bunch_compilation` registers flight coverage, fleet balance, and capacity rows;
- `bunch_selection` or the Application orchestrates the loop.

The same skeleton applies to cutting patterns in one-dimensional cutting stock, layers in three-dimensional packing, and routes in VRPTW.

## 3. Two cross-context protocols

### 3.1 ShadowPriceMap

A `ShadowPriceMap` should expose dual values through domain keys:

~~~kotlin
interface ShadowPriceMap {
    operator fun invoke(task: FlightTask): Flt64
    operator fun invoke(aircraft: Aircraft): Flt64
}
~~~

It isolates three moving parts:

1. the solver row number of a master constraint;
2. the backend API and sign convention for dual values;
3. the domain objects used by pricing.

Compilation owns the “row → domain key → dual value” mapping. Generation uses domain keys only and holds neither the master model nor solver handles.

### 3.2 Column

A column returned by pricing should contain at least:

| Field | Purpose |
|---|---|
| Stable identity/signature | Deduplication and tracing |
| Domain content | Flight string, route, cutting pattern, and so on |
| Original cost $c_p$ | Objective coefficient |
| Master coefficients $a_{ip}$ or enough data to derive them | Incremental column construction |
| Reduced cost and generation iteration | Diagnostics and verification |

The master constructs $\lambda_p$ and its coefficients from the domain column. Pricing must not mutate the RMP directly.

## 4. Lifecycle

A robust column-generation solve follows this sequence:

1. initialize every context and domain object;
2. generate initial columns that make the RMP feasible, or add expensive artificial columns;
3. let Compilation register master variables, objectives, and rows;
4. solve the LP relaxation of the RMP;
5. extract duals by domain key and build the `ShadowPriceMap`;
6. solve pricing for every decomposable entity;
7. validate feasibility, recompute reduced cost, deduplicate, and batch-add columns;
8. return to step 4 while improving columns exist;
9. solve an integer problem over the current columns or enter Branch-and-Price, according to the quality requirement;
10. map column values back to a domain solution and report the solve trace.

~~~kotlin
while (iteration < maxIterations) {
    val masterResult = compilation.solveRelaxation()
    val shadowPrices = compilation.shadowPriceMap(masterResult)

    val candidates = generation.generate(iteration, shadowPrices)
    val accepted = candidates
        .filter { generation.isFeasible(it) }
        .filter { compilation.reducedCost(it, shadowPrices) < -reducedCostTolerance }
        .distinctBy { it.signature }

    if (accepted.isEmpty()) {
        break
    }
    compilation.addColumns(accepted)
    iteration += 1
}
~~~

This is a responsibility sketch, not a promise of one fixed API signature. Contexts expose registration and analysis capabilities; an algorithm service owns the loop.

## 5. Initial columns and feasibility

Initial columns are an often-overlooked engineering concern:

- every required master row should be covered by at least one initial column;
- when a feasible column cannot yet be constructed, add an expensive artificial column and verify that its value is zero before declaring convergence;
- locked tasks, resource availability, and indivisibility rules must use the same predicates in initial generation and pricing;
- after Pricing publishes a column, Compilation should still perform a lightweight contract check before insertion.

If the RMP is infeasible, do not continue by reading dual values. Distinguish “insufficient initial columns” from “the original problem is infeasible.”

## 6. Stopping and numerical policy

Configure and record at least:

| Setting | Intended meaning |
|---|---|
| $\varepsilon_{\mathrm{rc}}$ | Tolerance for accepting a negative-reduced-cost column |
| `maxIterations` | Iteration limit |
| `maxColumnsPerIteration` | Maximum columns added per round |
| `duplicateTolerance` | Coefficient or domain-signature deduplication |
| `stallLimit` | Limit for no bound or objective improvement |
| `pricingTimeLimit` | Time limit for one pricing problem or one round |

Report these outcomes separately:

- **proven convergence**: exact pricing found no improving column;
- **heuristic stop**: heuristic pricing found none but cannot prove nonexistence;
- **resource stop**: a time or iteration limit was reached;
- **failure**: the master solve, dual extraction, or pricing failed.

## 7. Additional obligations for Branch-and-Price

Integer optimality requires column generation at branch nodes, and every branching rule must be propagated into pricing. Prefer domain relations such as “two tasks belong to the same flight string” or “this edge is used” over fixing only a current column variable.

Each node must maintain:

- node-specific branching constraints;
- a Pricing feasible region consistent with those constraints;
- the node bound, incumbent, and pruning reason;
- the scope and reuse conditions of generated columns.

Running column generation only at the root and then turning the current RMP into an integer model is a useful heuristic, but it does not replace a global Branch-and-Price proof.

## 8. Verification checklist

### Pricing unit tests

- Enumerate every feasible column on a small graph or combination space and compare the minimum reduced cost with Pricing;
- verify that resource extension and dominance cannot remove a potentially optimal label;
- cover zero duals, positive/negative duals, and values exactly on the tolerance boundary;
- verify that every returned column satisfies domain rules and has stable cost and identity.

### Master unit tests

- after one column is added, check the variable count, objective coefficient, and every affected row coefficient;
- verify the one-to-one mapping among `ShadowPriceMap` domain keys, rows, and signs;
- reject duplicate columns;
- define behavior for artificial columns, locked columns, and uncovered rows.

### Integration and benchmark tests

- on an enumerable instance, add every column to a full model and compare its LP objective with column generation;
- recompute the reduced cost of generated columns and verify that no value below $-\varepsilon_{\mathrm{rc}}$ remains at termination;
- check bound monotonicity, column count, iteration count, and stop reason;
- for Branch-and-Price, compare the integer optimum with a complete small MILP.

## 9. Common mistakes

- using solver row numbers as public `ShadowPriceMap` keys;
- implementing column cost or feasibility separately in Pricing and Master;
- reporting every “no column found” result as proven optimality;
- omitting the reduced-cost contribution of equalities, bounds, or stabilization terms;
- treating column generation and Branch-and-Price as synonyms;
- expanding every domain-column coefficient in the Application and thereby erasing context boundaries.

## 10. Related examples

- [Complex Example 3: One-Dimensional Cutting Stock](/examples/framework-example3)
- [Complex Example 4: Flight Recovery](/examples/framework-example4)
- [Complex Example 5: VRPTW Branch-and-Price](/examples/framework-example5)
- [DDD architecture fundamentals](/guide/use-ddd-architecture)
