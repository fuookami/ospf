# Numerical Stability and Modeling Quality

Numerical modeling is the discipline of making the mathematical meaning of a model survive finite precision, finite domains, and solver encodings. Four declarations matter repeatedly: variable bounds, units and scaling, the derivation of any Big-M constant, and the interpretation of tolerances at strict or equality boundaries.

This page develops those ideas through a small production model. It also shows when a reformulation is exact, when it is an intentional approximation, and how a symmetry-breaking rule can remove duplicate witnesses without changing the business optimum. The discussion is solver-neutral and does not invent a current OSPF API.

## 1. Why bounds and units are model semantics

A finite bound is more than a performance hint. It determines which plans the model permits and often supplies the proof needed for a conditional or disjunctive formulation. If $x\in[L,U]$, then an implication can be relaxed only as far as $L$ and $U$ allow.

Units are equally important. A residual of $0.1$ may mean $0.1$ kilograms, minutes, hours, or dollars. A feasibility tolerance, a strict-boundary margin, an objective weight, and a Big-M constant all inherit the units of the expressions where they occur. Changing units without changing these quantities changes the model.

Scaling is an algebraic change of variables intended to keep coefficients and residuals at comparable magnitudes. It should be recorded as part of the model transformation, not applied only to the matrix printed for a solver.

## 2. A complete bounded example: two identical plants

### 2.1 Business statement and data

There are two identical plants. Total production must be five integer units. An open plant can produce at most six units, costs 4 USD to open, and costs 1 USD per produced unit. A “large batch” means strictly more than three units at a plant. The large-batch indicator is useful for a downstream rule, so the model must classify it correctly.

| plant $i$ | production bound $U_i$ | fixed opening cost | unit cost |
|---|---:|---:|---:|
| $1$ | $6$ | $4 | $1 |
| $2$ | $6$ | $4 | $1 |

The plants are identical in every datum and rule. Therefore swapping plant labels must not change feasibility or cost.

### 2.2 Variables, intermediates, and constraints

Let $x_i$ be integer production, $y_i$ the open indicator, and $h_i$ the large-batch indicator:

$$
x_i\in\{0,1,\ldots,6\},\qquad
y_i,h_i\in\{0,1\}
\qquad(i=1,2).
$$

Define the total-quantity and total-cost intermediates

$$
Q=x_1+x_2,
\qquad
\operatorname{Cost}=4(y_1+y_2)+x_1+x_2.
$$

The demand and activation rules are

$$
Q=5,
$$

$$
x_i\le6y_i\qquad(i=1,2).
$$

The second row is exact because $x_i\ge0$ and $x_i\le6$: if $y_i=0$, it forces $x_i=0$; if $y_i=1$, it does not remove any value already allowed by the production bound.

For integer production, “strictly more than three” is exactly $x_i\ge4$. We encode the two branches with separate, derived Big-M values:

$$
x_i\ge4-4(1-h_i),
$$

$$
x_i\le3+3h_i
\qquad(i=1,2).
$$

Finally, choose one representative from the two plant-label symmetries:

$$
y_1\ge y_2.
$$

The cost objective is

$$
\min\;\operatorname{Cost}.
$$

The cost definition is written explicitly even though $Q=5$ fixes its variable-cost part; retaining the intermediate makes the cost meaning visible and generalizes when demand is an inequality.

### 2.3 Deriving the Big-M values

For the lower branch, when $h_i=0$ the row must allow the smallest declared production $x_i=0$:

$$
0\ge4-M_i^-\quad\Longrightarrow\quad M_i^-\ge4.
$$

Thus $M_i^-=4$ is tight. When $h_i=1$, the row becomes $x_i\ge4$.

For the upper branch, when $h_i=1$ the row must allow the largest declared production $x_i=6$:

$$
6\le3+M_i^+\quad\Longrightarrow\quad M_i^+\ge3.
$$

Thus $M_i^+=3$ is tight. When $h_i=0$, the row becomes $x_i\le3$.

Using one generic value $M=10^6$ would be logically valid only if it still respected all other bounds, but it would make the inactive branch unnecessarily weak and can hurt numerical conditioning. Using $M=2$ in the lower row would be invalid: it would incorrectly exclude $x_i=0$ when $h_i=0$.

More generally, for an upper conditional row

$$
g(x)\le b+My,
$$

the smallest valid non-negative value is determined by

$$
M\ge\max_{x\in D}\bigl(g(x)-b\bigr).
$$

For a lower conditional row

$$
g(x)\ge b-M(1-y),
$$

use

$$
M\ge\max_{x\in D}\bigl(b-g(x)\bigr).
$$

The maximum is taken over the declared domain and background constraints, not over an unbounded mathematical variable. For an affine $g$, valid variable bounds or a small auxiliary optimization problem can provide this maximum. Different rows often need different M values.

### 2.4 Hand-checking an optimum

Consider

$$
(x_1,x_2,y_1,y_2,h_1,h_2)=(5,0,1,0,1,0).
$$

The total quantity is $Q=5+0=5$. The activation rows give $5\le6$ and $0\le0$. For plant $1$, $h_1=1$ gives $x_1\ge4$ and $x_1\le6$; for plant $2$, $h_2=0$ gives $x_2\le3$. The symmetry row is $1\ge0$. The cost intermediate is

$$
\operatorname{Cost}=4(1+0)+5+0=9\ \text{USD}.
$$

Any feasible plan must open at least one plant because $Q=5$. Opening one plant costs 4 USD and producing all five units costs 5 USD, so 9 USD is a lower bound and the displayed plan is optimal.

Without the symmetry row, $(0,5,0,1,0,1)$ is an equally good second witness. The row $y_1\ge y_2$ retains the first representative without changing the attainable objective value.

## 3. Scaling and tolerance in declared units

Suppose a related time variable is originally measured in minutes:

$$
0\le t_{\mathrm{min}}\le1440.
$$

Define hours by $t=t_{\mathrm{min}}/60$. The same bound is

$$
0\le t\le24.
$$

For example, the minute equation $t_{\mathrm{min}}+s_{\mathrm{min}}\le1440$ becomes $t+s\le24$ when both variables are converted to hours. This does not change the feasible plans; it changes their coordinates and keeps coefficients near a useful scale. Converting cents to dollars similarly changes a cost of $350$ cents to $3.50$ dollars.

Every related quantity must be converted too. A residual tolerance of $0.1$ minute is

$$
\varepsilon_{\mathrm{hour}}=\frac{0.1}{60}\ \text{hour}\approx0.001667\ \text{hour}.
$$

A Big-M of $1440$ minutes becomes $24$ hours when it bounds that same time expression. A weight that values one hour at 20 USD must be changed consistently if the time objective is expressed in minutes. Scaling only coefficients while leaving objective weights or reported tolerances in the old unit silently changes policy.

## 4. Strict inequalities and the exact/approximate boundary

### 4.1 Integer variables: exact conversion

For integer $x$,

$$
x>3\quad\Longleftrightarrow\quad x\ge4.
$$

The two Big-M rows in the plant example therefore classify every integer value in $\{0,\ldots,6\}$ exactly:

| $x_i$ | valid $h_i$ |
|---:|---:|
| $0,1,2,3$ | $0$ |
| $4,5,6$ | $1$ |

There is no reason to use a tiny numeric epsilon such as $10^{-10}$ for this discrete boundary. The natural step is one production unit, expressed in the model's unit.

### 4.2 Continuous variables: a declared margin is a policy choice

For a real $x\in[0,6]$, the strict set $x>3$ is open at the boundary. A conventional closed mixed-integer formulation cannot represent that strict relation literally as an ordinary non-strict row. An operational policy may choose a margin $\delta=0.1$ and use

$$
x\ge3.1-(3.1-0)(1-h),
$$

$$
x\le3+(6-3)h.
$$

Then $h=1$ means $x\ge3.1$, while $h=0$ means $x\le3$. Values in $(3,3.1)$ have no branch. This is an intentional approximation or a conservative acceptance policy, not an exact encoding of $x>3$.

The margin should come from measurement resolution, contractual policy, or a declared separation requirement. It should not be chosen merely because it is smaller than the solver's feasibility tolerance. If values in the open gap are meaningful, model that third state explicitly or use a formulation whose semantics allow it; do not silently label the two-branch approximation exact.

### 4.3 Numerical tolerance is not a strict-boundary margin

For an equality $a(x)=b$, a numerical check may accept

$$
|a(x)-b|\le\varepsilon_{\mathrm{num}}.
$$

That band describes how a computed solution is classified. A business margin $\delta$ changes the feasible set or the meaning of a branch. Both quantities need a unit, scale, and direction. A result that is within numeric tolerance is not automatically within a contractual service tolerance.

## 5. Symmetry breaking

Two interchangeable plants create two label-swapped witnesses. The condition $y_1\ge y_2$ is valid here because both plants have the same domain, capacity, costs, and all other rules. Since demand forces at least one plant open, it chooses plant $1$ whenever exactly one is open.

Symmetry breaking is exact with respect to objective values: every solution can be permuted into one satisfying the representative rule, with the same objective and business interpretation. It does not improve the business plan by itself; it reduces duplicate search branches and can make the representative selection more predictable, but it does not guarantee a unique full plan when other symmetries remain.

The rule becomes invalid if plant $1$ and plant $2$ differ in cost, capacity, availability, quality, or any constraint. In that case swapping labels may change feasibility or value. A safe model should establish the equivalence class before adding a symmetry rule. Similar lexicographic representative rules can be used for interchangeable vehicles, rooms, or identical machines.

## 6. Exact reformulations versus approximations

The following classification is useful when reviewing a numerical model:

| construction | status in the stated model |
|---|---|
| $x_i\le6y_i$ with $0\le x_i\le6$ | exact activation link |
| $x_i>3\iff x_i\ge4$ for integer $x_i$ | exact discrete strict conversion |
| $M_i^-=4$, $M_i^+=3$ from valid bounds | exact indicator branches |
| $t=t_{\mathrm{min}}/60$ with no rounding | exact change of coordinates |
| $x>3$ replaced by $x\ge3.1$ for real $x$ | declared conservative approximation |
| accepting $|a(x)-b|\le\varepsilon$ | tolerance-based interpretation, not exact equality |
| $y_1\ge y_2$ for genuinely identical plants | exact objective-preserving representative choice |

Exactness describes a mathematical transformation for the declared domains, bounds, and units. A numerical acceptance tolerance is a separate reporting or solver convention; it does not turn an approximation or an open-boundary policy into an exact equality. If a bound is only an estimate, a Big-M derived from it is not a proof. If input data are rounded during scaling, the transformation is no longer algebraically exact.

## 7. Conceptual organization in OSPF

Numerical semantics can be kept visible through the following conceptual responsibilities:

1. A data or domain context owns physical units, finite bounds, integrality, data precision, and any equivalence classes used for symmetry.
2. A model context owns the original business rules: activation, strict service thresholds, equalities, and permitted approximation margins.
3. A compiler boundary derives row-specific bounds and Big-M values, applies recorded scaling, and preserves whether each transformation is exact, tolerance-based, or approximate. It should retain the source rule that justified each generated row.
4. A solving-results context reports scaled and business-unit values, residuals, branch indicators, objective values, symmetry representative choices, and any unresolved numerical or optimality status.

This separation is the subject of [compiler architecture](./compiler-architecture). [Solving results](./solving-results) gives the reporting perspective: a solution value without its units, residual tolerances, or proof status is incomplete. When asking why a target cannot improve, [critical constraint analysis](./critical-constraint-analysis) should use the original bounded rule and its declared business semantics, not an unexplained internal Big-M row.

The organization above is conceptual and deliberately avoids implying a particular current class or function name.

## 8. Common pitfalls

### 8.1 Guessing a universal Big-M

An oversized constant can weaken the continuous relaxation and amplify round-off; an undersized one removes valid plans. Derive each M from finite bounds and state the derivation. If no reliable bound exists, first reformulate or obtain one rather than hiding the uncertainty in a larger number.

### 8.2 Letting a bound disagree with the data

If the declared capacity is six but an input row permits production eight, either the model is infeasible for valid data or the bound is false. Big-M derivations, scaling, and symmetry claims all depend on the same bound being authoritative.

### 8.3 Using machine epsilon for a business boundary

A strict service threshold is not made meaningful by adding $10^{-12}$. Choose a margin in kilograms, minutes, dollars, or the relevant discrete step. A tiny margin can be swallowed by solver tolerances; a large one can remove real plans.

### 8.4 Comparing residuals before converting units

A residual of $0.001$ hours is about $0.06$ minutes. Reports that compare it with a $0.001$-minute requirement without conversion draw the wrong conclusion. Store both the internal scale and the user-facing unit.

### 8.5 Applying symmetry breaking to non-identical objects

Ordering two machines by an arbitrary ID is not safe if their capacities or availability differ. Verify full interchangeability, including optional rules and costs, before imposing a representative order.

### 8.6 Treating an approximation as an equivalence

An epsilon margin creates a gap; a rounded coefficient changes a boundary; an approximate nonlinear envelope may omit feasible points. Label the choice, quantify the affected unit, and include it in the result and review evidence.

### 8.7 Forgetting objective and report scaling

Changing a variable from minutes to hours while leaving its objective coefficient unchanged changes the relative value of that objective. The same applies to weighted multi-objective terms. Transform constraints, objectives, tolerances, and displayed values consistently.

## 9. Related pages

- [Critical constraint analysis](./critical-constraint-analysis)
- [Inequality indicator](./linear-functional/inequality)
- [If](./linear-functional/if)
- [Slack](./linear-functional/slack)
- [Absolute value](./linear-functional/abs)
- [Solving results](./solving-results)
- [Compiler architecture](./compiler-architecture)
- [Operations-research language](./operations-research-language)
- [Modeling workflow](./modeling-workflow)
- [Symbolic expressions](./symbolic-expressions)
- [Rolling optimization](./rolling-optimization)
- [Infeasibility analysis](./infeasibility-analysis)
- [Scenario analysis](./scenario-analysis)
