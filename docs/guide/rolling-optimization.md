# Rolling Optimization and Reoptimization

Rolling optimization addresses changing business state: new orders, unavailable equipment, or execution that differs from a plan. It is neither repeated solving of an unchanged model nor [time-slice scheduling](./time-slice-cycle-solver) of compute resources.

## 1. Three time ranges

The planning horizon covers the future considered now. The execution window selects actions actually released. The frozen window protects commitments that cannot freely change. These windows can differ; long-term recommendations are not executed facts.

Each roll first reads actual state, then constructs new inputs. Unfinished tasks continue, completed tasks leave, and inventory or equipment state becomes the initial condition. The previous plan is not the sole source of reality.

## 2. Example: two-period production and stability

### Overview, entities, and sets

A production context meets cumulative demand over $T=\{1,2\}$. Capacity is $K_t=5$ items per period, with cumulative demands $D_1=3,D_2=8$. The old plan is $\bar x=(4,4)$, and period-one production of four items is committed but not executed. Production costs 1 currency unit/item; plan changes cost 2 currency units/item.

### Variables and intermediate values

$x_t\in\mathbb Z_{\ge0}$ is planned production in items. Auxiliary $v_t\ge0$ measures absolute change for the penalty. Define cumulative production and cost:

$$
P_t=\sum_{k=1}^{t}x_k,\qquad C=\sum_{t\in T}x_t+2\sum_{t\in T}v_t.
$$

The frozen set is $F=\{1\}$; $T\setminus F$ is adjustable. Data assertions require nondecreasing cumulative demand and nonnegative capacities.

### Constraints and objective

$$
\begin{aligned}
\text{s.t.}\quad &0\le x_t\le K_t &&\forall t\in T,\\
&P_t\ge D_t &&\forall t\in T,\\
&x_t=\bar x_t &&\forall t\in F,\\
&v_t\ge x_t-\bar x_t,\quad v_t\ge\bar x_t-x_t &&\forall t\in T.
\end{aligned}
$$

Minimize $C$. The positive penalty and absence of other constraints forcing larger $v_t$ ensure $v_t=|x_t-\bar x_t|$ at an optimum. This relies on the objective, not an unconditional graph formulation.

### New demand and result

If cumulative demand becomes $D_2=9$, the optimal plan with period one frozen is $(4,5)$: production 9, change 1, and cost $C=11$. The old $(4,4)$ plan no longer meets demand and is not automatically a valid fallback.

If period-two capacity also drops to 4, demand 9 cannot be met under the freeze. Report the conflict or obtain a business-approved commitment change; do not silently delete the freeze.

## 3. Rebuilding, incremental changes, and warm starts

Semantically, each run is a new versioned scenario. Implementations may rebuild or use backend-supported incremental changes, provided stale state cannot contaminate the new model.

Warm starts supply candidate values or supported recovery information. They do not establish feasibility under new data, guarantee tree reuse, or guarantee speed. Initialize new variables and map removed tasks through business identities.

## 4. Executed versus committed decisions

Executed quantities become facts. If period one actually produced only three items, compute remaining demand from three, not the planned four. Committed but unexecuted actions can instead be represented by frozen constraints.

Freeze rules are business policy, not a solver default. When unfreezing is authorized, retain what commitment changed and why. Moving windows must also avoid counting demand twice.

## 5. Time limits, stability, and adoption

If a feasible plan exists at timeout, business quality rules decide whether to release its execution window. Without one, use an old plan only after checking it against new inputs, or use an explicitly feasible contingency.

Stability may measure production changes, reassignment count, or start-time shifts. Different units cannot be added without interpretation, and weights cannot replace mandatory commitments. Separate hard freezes from soft change penalties.

## 6. Responsibilities in OSPF

Business contexts expose actual state, variables, and stability indicators. The application selects triggers, horizons, freeze policies, and plan publication. The backend solves. Retain input versions so late results from older runs cannot overwrite newer plans.

Continue with [Multi-Objective Optimization](./multi-objective-optimization), [Infeasibility Analysis](./infeasibility-analysis), and [Scenario Analysis](./scenario-analysis).
