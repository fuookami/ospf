# Multi-objective Optimization and Soft Constraints

Many decisions have more than one legitimate measure of quality. A delivery plan may reduce operating cost by arriving later; a production plan may protect service quality while using more capacity. Multi-objective optimization makes that policy explicit instead of hiding it in an unexplained scalar score.

This page uses a delivery-delay and operating-cost example to distinguish hard constraints from soft violations, weighted sums from lexicographic priorities, and business tolerances from numerical solver tolerances. The equations are solver-neutral; they do not prescribe a particular OSPF or backend API.

## 1. Hard rules, soft rules, and objective dimensions

Let $X$ be the set defined by the hard rules. A hard rule is part of feasibility: a plan outside $X$ is not a candidate, even if it would score well elsewhere. Examples include serving each order exactly once, respecting vehicle capacity, and obeying a legal working-time limit.

A soft rule is measured rather than required. If $g(x)\le b$ is a soft upper bound, its non-negative violation is

$$
v(x)=\max\{0,g(x)-b\}.
$$

For a soft lower bound $g(x)\ge b$ and a soft equality $g(x)=b$, use

$$
v(x)=\max\{0,b-g(x)\},
\qquad
v(x)=|g(x)-b|,
$$

respectively. The unit of $v$ is inherited from $g$: a delivery delay is measured in days or hours, while a budget violation is measured in dollars. A solver feasibility tolerance is not a business violation measure.

After the hard rules define $X$, an objective vector might be

$$
f(x)=\bigl(f_1(x),f_2(x),\ldots,f_K(x)\bigr),
$$

where each component has a direction, unit, and policy meaning. For minimization, $x$ Pareto-dominates $x'$ when it is no worse in every component and strictly better in at least one. A weighted sum selects one point according to an exchange-rate policy; a lexicographic objective declares an order of importance.

## 2. Weighted sums and lexicographic order

### 2.1 Weighted sum

For minimization criteria with non-negative weights,

$$
F_w(x)=\sum_{k=1}^K w_k\,\bar f_k(x),
$$

where $\bar f_k$ should be dimensionless or explicitly converted to a common business unit. A useful normalization is

$$
\bar f_k(x)=\frac{f_k(x)-\ell_k}{u_k-\ell_k},
$$

using declared lower and upper reference values $\ell_k,u_k$ with $u_k>\ell_k$. Handle constant criteria separately rather than dividing by zero. A weight is meaningful only relative to that normalization. Adding `days + dollars` without a conversion silently assigns an arbitrary exchange rate.

Weighted sums are useful when trade-offs are genuinely compensatory. They can also miss unsupported points of a non-convex Pareto frontier and can accept a large violation of one soft rule when another term has a sufficiently favorable weight.

### 2.2 Lexicographic objective

For two minimization criteria, lexicographic minimization of $(f_1,f_2)$ means:

1. find the smallest attainable $f_1$;
2. among solutions attaining that value, find the smallest $f_2$.

The first criterion is not traded for an improvement in the second. This is appropriate when any one day of delay is more important than any permitted cost saving, or when a regulatory rule must be optimized before a secondary preference.

A single scalar can reproduce a lexicographic policy only when objective ranges and attainable resolution are known. If $f_1$ has minimum separation $\delta_1>0$ and the possible variation of $f_2$ is at most $\Delta_2$, choose:

$$
M>\frac{\Delta_2}{\delta_1}
$$

and minimize $M f_1+f_2$. Without a positive resolution or a finite secondary bound, a guessed large weight is not a proof of lexicographic behavior.

### 2.3 Tolerance around a priority level

Sometimes the first criterion may be relaxed slightly for a better second criterion. Let $f_1^*$ be the proven optimum of the first level and let $\varepsilon_1$ be a business tolerance in the same unit as $f_1$. The second level may then be solved subject to

$$
f_1(x)\le f_1^*+\varepsilon_1
$$

for minimization. This is a policy tolerance, not a floating-point feasibility tolerance. State whether it is absolute or relative, and keep its unit in the result evidence.

## 3. Worked example: delivery delay versus route cost

### 3.1 Business statement

A single vehicle delivers orders $A$ and $B$ in two consecutive one-day slots, $0$ and $1$. Order $A$ is due at the beginning of day $0$; order $B$ is due at the beginning of day $1$. The route that visits $A$ first costs 10 USD, while the route that visits $B$ first costs 2 USD. The due dates are soft: lateness is allowed but measured in days. Serving each order exactly once and using one slot per order are hard rules.

The deliberately small data set makes every candidate hand-checkable. It also illustrates why the choice between weighted and lexicographic policy is a business decision, not an algebraic afterthought.

### 3.2 Variables and hard constraints

Let

$$
y_{AB},y_{BA}\in\{0,1\}
$$

indicate the chosen route order. Let $d_A,d_B\in\{0,1\}$ be the delivery slots, where $0$ is the first slot and $1$ is the second. The hard route and slot rules are

$$
y_{AB}+y_{BA}=1,
$$

$$
d_A+d_B=1,
$$

$$
d_A=y_{BA},\qquad d_B=y_{AB}.
$$

The last two equations connect the route choice to the slot assignment. They are not soft: a plan that delivers twice in one slot or does not serve an order is infeasible.

### 3.3 Soft violations, intermediates, and cost

Let the due slots be $\tau_A=0$ and $\tau_B=1$. In a general model, delay is

$$
\ell_i=\max\{0,d_i-\tau_i\}.
$$

For the two-slot domains here this reduces exactly to

$$
\ell_A=d_A,\qquad \ell_B=0,qquad L=\ell_A+\ell_B.
$$

Thus $L$ is total delay in days. The route cost intermediate is

$$
C=10y_{AB}+2y_{BA}\quad\text{USD}.
$$

The two objective dimensions are $(L,C)$, both minimized. With only lower bounds on delay, establish that the objective drives it to the positive-part value without other coupling preventing this, or define it using an exact function-graph formulation. Adding an upper bound alone does not establish equality to the positive part.

### 3.4 Enumerate and check every route

There are only two feasible route choices:

| route | $y_{AB}$ | $y_{BA}$ | $d_A$ | $d_B$ | $\ell_A$ | $\ell_B$ | $L$ (days) | $C$ (dollars) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| $A\rightarrow B$ | $1$ | $0$ | $0$ | $1$ | $0$ | $0$ | $0$ | $10$ |
| $B\rightarrow A$ | $0$ | $1$ | $1$ | $0$ | $1$ | $0$ | $1$ | $2$ |

Both rows satisfy every hard constraint. The second row saves 8 USD but delays order $A$ by one day.

### 3.5 Weighted policy

To make the units explicit, define the dimensionless scalar

$$
F_{5}(x)=5\frac{L}{1\ \text{day}}+1\frac{C}{1\ \text{USD}}.
$$

The values are

$$
F_5(A\rightarrow B)=5(0)+10=10,
$$

$$
F_5(B\rightarrow A)=5(1)+2=7.
$$

With this exchange-rate policy, the cost saving wins and the selected route is $B\rightarrow A$. If delay receives weight $9$ instead,

$$
F_9(A\rightarrow B)=10,
\qquad
F_9(B\rightarrow A)=9+2=11,
$$

so $A\rightarrow B$ wins. Neither weight is universally correct; each states how the business values one day relative to one dollar after unit conversion.

If several soft rules exist, the weighted form can include their violation measures, for example

$$
F_w=w_A\frac{\ell_A}{1\ \text{day}}+w_B\frac{\ell_B}{1\ \text{day}}+w_C\frac{C}{1\ \text{USD}}.
$$

Keep a hard rule as a constraint defining $X$; only an explicitly authorized exception may be represented by a soft violation.

### 3.6 Lexicographic policy

Under lexicographic minimization of $(L,C)$, the first level gives

$$
L^*=0\ \text{days}.
$$

Only $A\rightarrow B$ attains that value, so the second level has no remaining choice and returns $C=10\ \text{USD}$. The 8 USD saving cannot justify a one-day violation because cost is a lower-priority criterion.

For this finite example, lexicographic order can also be encoded with a single coefficient. Cost ranges from 2 USD to 10 USD, so its maximum difference is 8 USD. Since delay changes in whole days, $M=9$ is sufficient:

$$
\min\;9L+C.
$$

The two values are $10$ and $11$, respectively. The derivation matters: merely choosing a large-looking coefficient without a cost bound and delay resolution does not establish the same policy.

### 3.7 Tolerance policy

Suppose the company accepts up to one day of total delay when that produces a cheaper route. First solve the primary level and obtain $L^*=0$. With a business tolerance $\varepsilon_L=1$ day, the second level is

$$
\min C
\qquad\text{s.t.}\qquad
L\le L^*+\varepsilon_L=1\ \text{day}.
$$

Both rows are allowed, and the result is $B\rightarrow A$ with $C=2\ \text{USD}$. If $\varepsilon_L=0$ days, only $A\rightarrow B$ remains. A tolerance of $0.25$ day also selects $A\rightarrow B$ here because the attainable delays are integral days.

The tolerance is not the same as a solver's numeric feasibility band. A report should say “accepted within one day of the best delay,” not merely “within tolerance.”

## 4. Organizing objective semantics in OSPF

An OSPF model can keep multi-objective policy explicit through conceptual roles:

1. A domain context defines the delivery decisions and hard feasibility rules.
2. A violation context derives non-negative, unit-bearing measures such as delay, rejected quantity, or overtime. These are semantic intermediates, not arbitrary residuals from an internal row.
3. An objective context declares each objective's name, direction, unit, normalization reference, priority level, and business tolerance.
4. A compilation boundary chooses a weighted objective, a staged lexicographic solve, or another supported representation while preserving the declared vector and policy.
5. A result context records every objective component, each priority-level optimum or bound, the tolerances applied, and the final hard/soft status.

The [compiler architecture](./compiler-architecture) page describes why objective policy should survive translation into a solver mechanism. [Solving results](./solving-results) explains why one scalar score is not enough evidence for a multi-objective result: users need the component values and the status of each level. [Critical constraint analysis](./critical-constraint-analysis) can then analyze a stated target for one objective while documenting whether other objective levels were fixed, preserved lexicographically, or allowed to trade off.

This organization is conceptual. It does not assume a particular current class, factory, or backend capability.

## 5. Common pitfalls

### 5.1 Adding quantities with incompatible units

The expression $L+C$ has no natural meaning when $L$ is in days and $C$ is in dollars. Normalize each component or state a conversion such as “one day is valued at 9 USD.” Store the unit and scale alongside the objective definition.

### 5.2 Using weights as a substitute for a priority rule

A weight that works on today's data can fail after costs, capacities, or bounds change. If “never sacrifice service level” is the policy, use a lexicographic level or a proven dominance coefficient derived from bounds and resolution. Do not call a merely large coefficient lexicographic without that derivation.

### 5.3 Turning a hard rule into a penalty accidentally

An order that must be served, a safety limit, or a legal constraint belongs in $X$. Giving it a high penalty still permits violation when another term is sufficiently favorable. If controlled exceptions exist, model the exception explicitly and report it as a separate soft violation.

### 5.4 Leaving violation variables only lower-bounded

Rows such as $\ell_i\ge d_i-\tau_i$ and $\ell_i\ge0$ describe a lower envelope. They produce the intended violation only when the objective or another equality drives $\ell_i$ to its minimum. Otherwise the model may return an inflated violation and a misleading report.

### 5.5 Ignoring finite ranges when emulating priorities

The coefficient $M$ for $M f_1+f_2$ must dominate every possible change in $f_2$ for one attainable improvement in $f_1$. If the secondary objective is unbounded, or if a continuous primary objective has arbitrarily small improvements, a finite guessed $M$ cannot prove lexicographic behavior.

### 5.6 Mixing business tolerance with numerical tolerance

A one-day acceptance band and a $10^{-8}$ feasibility tolerance answer different questions. Business tolerance changes the set of acceptable plans; numerical tolerance describes how a solver interprets a computed residual. Record them separately, with units and directions.

### 5.7 Reporting only the scalar score

Two plans can have the same weighted score but very different delay and cost. A result should include the objective vector, normalized components, weights or priority order, tolerances, and the hard-feasibility status. If a solve is not proven optimal at one level, say so instead of presenting the incumbent scalar as a certified policy optimum.

## 6. Related pages

- [Critical constraint analysis](./critical-constraint-analysis)
- [Inequality indicator](./linear-functional/inequality)
- [Slack](./linear-functional/slack)
- [Absolute value](./linear-functional/abs)
- [Maximum](./linear-functional/max)
- [Solving results](./solving-results)
- [Compiler architecture](./compiler-architecture)
- [Operations-research language](./operations-research-language)
- [Modeling workflow](./modeling-workflow)
- [Symbolic expressions](./symbolic-expressions)
- [Rolling optimization](./rolling-optimization)
- [Infeasibility analysis](./infeasibility-analysis)
- [Scenario analysis](./scenario-analysis)
