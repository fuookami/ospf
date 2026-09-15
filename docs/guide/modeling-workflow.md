# The Modeling and Solving Workflow

[Getting Started](./getting-started) provides an executable entry point. This chapter explains objects, dependencies, and results within one solve; a small model need not start with multiple business contexts.

## 1. From input to a business plan

A solve includes freezing inputs, checking data, building expressions, assembling a model, executing a backend, and mapping results. Finishing model construction does not mean solving is complete, and a returned solver call does not imply a feasible solution exists.

| Stage | Input | Output |
|---|---|---|
| Input preparation | Business records and scenario parameters | Determinate data for this run |
| Expression construction | Data and business indices | Variables and intermediate values |
| Model assembly | Expressions, rules, and objectives | Complete variable domains and constraints |
| Transformation and execution | Model, backend capabilities, settings | Status, solutions, bounds, and diagnostics |
| Result mapping | A confirmed solution and business indices | Business plan and indicators |

## 2. Example: two products sharing labor

### Overview, concepts, and sets

A production context allocates one shift's labor. Let $I=\{A,B\}$ and $H=8$ hours. Unit labor requirements are $a_A=2,a_B=3$ hours/item, and returns are $r_A=3,r_B=4$ currency units/item. Demand limits, inventory, and carryover are outside this example.

### Variables

$x_i\in\mathbb Z_{\ge0}$ is production in items for every $i\in I$. Labor gives valid upper bounds $x_A\le4,x_B\le2$. No auxiliary variables or extra filtering predicates are needed.

### Intermediate values

Used labor $U$ and total return $R$ are business indicators:

$$
U=2x_A+3x_B,\qquad R=3x_A+4x_B.
$$

### Assertions, constraints, and objective

Positive unit labor and a nonnegative budget are data assertions. The resource constraint and return objective are:

$$
\text{s.t.}\quad U\le8,\qquad \max R.
$$

Enumerating $x_B=0,1,2$ gives best corresponding $x_A=4,2,1$ and returns 12, 10, 11. Thus $(4,0)$ is optimal.

## 3. Establish identity before dependencies

Create stable product indices and variables first, then $U,R$, then constraints and objectives that reference them. An expression existing in the host program does not mean its variables belong to the solved model; assembly must make dependencies complete.

Business identities such as “product A” differ from backend column numbers. Preserve the variable-to-entity mapping for this run rather than assuming the first returned number always belongs to A, especially when collection order changes.

Intermediate values can serve multiple constraints and reporting indicators. Matching names alone do not establish symbol identity. See [Compiler-Like Architecture](./compiler-architecture) for registration and transformation responsibilities.

## 4. Data assertions versus solve constraints

A missing $a_A$ must not silently become zero; it is incomplete input. With $H=0$, zero production is valid in this model. If the business also requires at least one item, $H=0$ yields a genuinely infeasible instance.

This distinction determines whether to report a preparation error, modeling error, or solve outcome. Data assertions check known facts; solver constraints restrict unknown decisions. Neither replaces the other.

## 5. Configuration, execution, and reading results

Choose a backend suitable for integer linear models and set this run's time limit and result requirements. Constructible expressions are not automatically supported by every backend. Keep backend settings separate from business constraints.

After execution, read the status, establish whether a feasible solution exists, map $x_A,x_B$, and calculate $U,R$ with the same parameters. An empty result or default zeros must not become a business plan when no feasible solution exists.

Reports retain both status and indicators. “Return 11, feasible, optimality unproven” differs from “optimal return 11.” See [Understanding Solver Results](./solving-results).

## 6. Lifetimes and reuse

Reuse rule definitions and components, not indiscriminately the variable instances from the previous run. New scenarios need their own inputs and bindings. Warm starts provide candidate information; they do not bypass feasibility checks against the new model.

As the model grows, a resource context can own labor usage and a commercial context return, with application-level assembly. See [DDD Architecture](./use-ddd-architecture) for responsibilities and [Rolling Optimization](./rolling-optimization) for changing business data.
