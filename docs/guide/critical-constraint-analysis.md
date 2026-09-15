# Automatic Critical Constraint Analysis

A solver answers “What is the best plan?” Critical constraint analysis asks “Why can it not be better?” It separates boundary features of a solution, benefits from changing business rules, and combinations of conditions that prevent a target. The approach applies to constraint programming (CP), mixed-integer linear programming (MILP), and models with an exact linear reformulation.

This page explains the mathematics, analysis workflow, and interpretation of reports. Workflows and reports are conceptual examples, not calls to specific Kotlin/Rust APIs.

## 1. Three levels of evidence

First review [solutions, bounds, and statuses](./solving-results). If the original model has no solution, start with [Infeasibility Analysis and Constraint Relaxation](./infeasibility-analysis); this chapter focuses on limits to better targets in a feasible model.

| Level | Question | Methods | Scope of the conclusion |
|---|---|---|---|
| Activity | Which constraints are at or near a boundary at this solution? | Slack, boundary distance, semantic evaluation | A feature of this solution, not proof of objective impact |
| Effectiveness | Does relaxing a constraint produce a benefit? | Fixed-integer LP, finite perturbation, removal and reoptimization | Distinguishes a fixed integer pattern from the full model |
| Joint blocking | Which constraints jointly prevent a specified target? | Target feasibility, conflict cores, minimal unsatisfiable subsets | Combination-level evidence relative to an explicit target and background |

Use inexpensive analysis to prioritize candidates, then reoptimize for important candidates. Use joint analysis to explain target unreachability. The first two levels are not prerequisites for the third: CP constraints without a natural slack or LP dual can go directly to target feasibility.

```text
Freeze the baseline model and objective
    → Solution activity
    → Local sensitivity / single-constraint perturbation
    → Feasibility of a better objective target
    → Conflict extraction and shrinking
    → Explanation grouped by original business rules
```

## 2. Establish the baseline and objective direction

Let $X$ be the original feasible set, $f(x)$ the objective, and $x^0$ a baseline feasible solution with value $z_0=f(x^0)$. If optimality is proven, write $z^*=z_0$; otherwise $z_0$ is only an incumbent value.

Activity requires a trusted feasible solution, but a “cannot improve” explanation must be limited to a specific target proven unreachable. A better target may be reachable from an unproven incumbent. Even ruling out one improvement amount does not exclude smaller improvements.

An analysis session retains the model snapshot, objective identity and direction, baseline solution, solve status, bounds, and tolerances. Integer fixing, RHS changes, temporary removal, and target conditions belong to derived models, not the business model. Cache keys also distinguish model fingerprints, objectives, perturbations, background conditions, and solver settings to prevent conclusions from leaking across scenarios.

For multiple objectives, specify which objective is analyzed and whether the others are fixed as conditions, preserved lexicographically, or combined into a scalar objective. Do not mix benefits across objective levels without defining that policy.

## 3. Activity: is this solution on the boundary?

For $a_i^Tx\le b_i$, define signed slack:

$$
s_i=b_i-a_i^Tx^0.
$$

For $a_i^Tx\ge b_i$, use $s_i=a_i^Tx^0-b_i$ so that feasible-side slack is positive. For an equality, record the residual $a_i^Tx^0-b_i$; satisfying an equality does not establish objective impact.

With tolerance $\varepsilon_i$ appropriate to units and numerical scale:

$$
\begin{aligned}
s_i < -\varepsilon_i &\quad\Rightarrow\quad \text{violated},\\
|s_i|\le\varepsilon_i &\quad\Rightarrow\quad \text{active},\\
s_i>\varepsilon_i &\quad\Rightarrow\quad \text{positive slack}.
\end{aligned}
$$

A separate threshold can identify nearly active constraints with positive slack. Record its definition and normalization. If the baseline violates a constraint, resolve feasibility or numerical consistency first; a violation is not activity.

For example, $x+y\le10$ has zero slack at $(4,6)$, whereas $x\le100$ has slack 96. These facts describe the location of that point, not the change in the optimum after relaxation.

Global constraints such as `AllDifferent`, `NoOverlap`, and `Cumulative` may not have a unique natural slack. Retain satisfied, violated, or unknown status, or use an explicitly defined semantic metric. Do not present the slack of internal linearization rows as a universal slack for the original global constraint.

## 4. Effectiveness: does relaxation improve the objective?

### 4.1 Local sensitivity with a fixed integer pattern

For a MILP, fix the integer variables at their current values:

$$
x_I=x_I^0.
$$

If the remaining problem is an LP, solve it and obtain dual values $\pi_i$. Within the applicable sensitivity range, objective sign convention, and basis conditions, a local RHS change can be interpreted through $\pi_i\Delta b_i$.

This concerns continuous adjustments with the integer pattern unchanged, not a global MILP shadow price. A zero dual does not exclude improvement through a different integer pattern after a larger perturbation. Degeneracy can also make dual solutions nonunique.

When the LP comes from an exact MIP reformulation, state which integer variables are fixed, including introduced discrete choices. Interpret internal row duals through their dependence on original parameters; do not arbitrarily sum them into a shadow price for a CP global constraint. Skip this level when no suitable LP representation exists.

### 4.2 Finite perturbation and full-model reoptimization

Relax an upper-bound constraint by $b_i\mapsto b_i+\delta$ and a lower-bound constraint by $b_i\mapsto b_i-\delta$, with $\delta\ge0$. Equalities and global constraints need separately defined business changes, such as more capacity or a wider time window, rather than a universal “add to the RHS” operation.

Let $z_i(\delta)$ be the perturbed optimum. Define positive gain consistently:

$$
G_i(\delta)=
\begin{cases}
z_i(\delta)-z^*, & \text{maximize},\\
z^*-z_i(\delta), & \text{minimize}.
\end{cases}
$$

Integer variables may change during reoptimization, so the result is not limited to the original pattern. The ratio $G_i(\delta)/\delta$ is an average gain over that perturbation, not a global derivative.

Call the difference a change in the optimum only when both optima are proven. With only a perturbed feasible solution, report observed gain and solver bounds; failure to observe a gain does not prove ineffectiveness. If the baseline is itself unproven, a better solution than its incumbent is not necessarily caused by relaxation.

### 4.3 Adaptive perturbation and removal

Try $\delta_0,2\delta_0,4\delta_0,\ldots$ until a benefit is found or the permitted business range or analysis budget is reached. For nested expanding feasible sets, reliable solve conclusions, and an explicit gain tolerance, refine an effective-threshold interval:

$$
\delta_i^*=\inf\{\delta\ge0:G_i(\delta)>\varepsilon_z\}.
$$

Search discrete parameters over their allowed values. For continuous parameters, report an interval at the chosen precision. A timed-out solve is not evidence of “no gain” on one side of a binary search, and the gain function need not be smooth.

Removing a constraint entirely is a stronger comparison, but not necessarily an authorized business action. No gain after removal means only that removing it alone is insufficient. An unbounded result must be reported separately, not as an ordinary finite gain.

### 4.4 Example: an inactive constraint can still prevent improvement

Consider integer production quantity $q$:

$$
\begin{aligned}
\max\quad &q\\
\text{s.t.}\quad &q\in\mathbb Z_{\ge0},\\
c_1:\quad &q\le10,\\
c_2:\quad &2q\le21.
\end{aligned}
$$

The optimum is 10. Constraint $c_1$ is active, while $c_2$ has slack 1 and is inactive. Removing only $c_1$ still leaves integer production at most 10; removing only $c_2$ does the same. Relaxing both bounds to $q\le11$ and $2q\le22$ allows production of 11.

Neither inactivity nor no gain from individual removal implies irrelevance to a bottleneck. Activity and local duals can prioritize analysis, but must not permanently exclude inactive constraints.

## 5. Turn objective improvement into feasibility

Instead of arbitrarily perturbing a multivariable solution by $x^0+\epsilon$, specify the desired improvement $\Delta>0$:

$$
\tau_\Delta(x)=
\begin{cases}
f(x)\ge z_0+\Delta, & \text{maximize},\\
f(x)\le z_0-\Delta, & \text{minimize}.
\end{cases}
$$

Then solve the satisfaction problem $x\in X\land\tau_\Delta(x)$. “Transport at least 100 additional kilograms” is easier to interpret than “improve slightly.” A relative amount can use $r|z_0|$, but a zero baseline requires another absolute scale. Integer objectives also require attention to attainable increments and tolerances.

| Result | Supported conclusion |
|---|---|
| Reachable (SAT) | A feasible plan meets the target and can be returned |
| Unreachable (UNSAT / Infeasible) | The target is proven infeasible under the specified model and background |
| Unknown | Time or resource limits, or another uncertainty, leave the question unresolved |
| Unsupported | The selected path cannot express or handle the required semantics |

Not finding a solution is not proof of unreachability. If the baseline model is already infeasible, diagnose infeasibility first rather than explain optimality.

## 6. From a conflict core to a minimal blocking set

Let $B$ contain always-retained variable types and non-relaxable rules, $C$ the diagnostic constraints, and $\tau$ the target. For an unreachable target, extract a conflict from $B\land C\land\tau$ and shrink it to $M\subseteq C$.

A target-relative minimal blocking set satisfies:

$$
B\land\bigwedge_{c\in M}c\land\tau\quad\text{is UNSAT},
$$

$$
\forall c\in M:\quad
B\land\bigwedge_{d\in M\setminus\{c\}}d\land\tau\quad\text{is SAT}.
$$

“Minimal” means inclusion-minimal: no member can be deleted. It does not mean globally minimum cardinality or uniqueness. If the background alone prevents the target, $M$ can be empty. Reports must show the background and target, not just constraint names.

A backend unsat core or conflict set is not necessarily minimal. For an IIS, also specify whether irreducibility is defined over original business constraints, variable bounds, or internal solver rows. Mapping names alone does not establish an original-constraint-level MUS.

Deletion-based shrinking holds the target and background fixed and tries removing each candidate:

```text
M = a confirmed blocking set
For each candidate c in M:
    Solve B ∧ (M without c) ∧ target
    If UNSAT is confirmed: remove c from M
    If SAT is confirmed: retain c and record the feasible witness
    If Unknown: retain c and mark minimality as unresolved
```

If uncertainty or budget ends the process, report a valid conflict without claiming proven minimality. Diagnostic activation must actually control the corresponding original constraint. Generated auxiliaries must not unintentionally keep restricting the model after their original rule is disabled.

### 6.1 Example: two capacities jointly block a target

Let $x,y\in\mathbb Z_{\ge0}$ be production on two lines, with capacities $c_A:x\le5$ and $c_B:y\le5$. Maximize $x+y$. The optimum is 10, and target $\tau:x+y\ge11$ is infeasible.

Relative to the background that $x,y$ are nonnegative integers, $M=\{c_A,c_B\}$ is a minimal blocking set. Without $c_A$, $(6,5)$ satisfies the remaining conditions; without $c_B$, $(5,6)$ does. The explanation is that the two capacities jointly limit total output, not that each constraint independently blocks the target.

## 7. Blocking explanations are not relaxation recommendations

Deleting any member of one minimal blocking set removes the obstruction from that set of conditions. It does not guarantee reachability in the full model: other constraints can form another conflict.

In the earlier integer-production example, target $q\ge11$ is blocked separately by $\{c_1\}$ and $\{c_2\}$. There are two singleton blocking sets, and removing just one is insufficient. This has a different business meaning from the two-member blocking set for the production lines.

A minimal correction set (MCS) instead identifies an inclusion-minimal set of permitted constraints whose removal from the full model restores target feasibility. It is a different object from a MUS. Minimum cardinality, minimum cost, and inclusion-minimality are also different objectives. Recommendations additionally need permitted relaxation ranges, costs, and approval conditions; diagnostic results must not directly execute rule changes.

## 8. Organizing analysis and business explanations in OSPF

Organize analysis around solver-neutral model semantics. Backend capabilities determine whether to use LP duals, full-model reoptimization, native conflict cores, or repeated satisfaction solving. Native CP global constraints without a suitable LP path can go directly to target feasibility and conflict analysis. Exact reformulations must also preserve original semantic identities.

Use stable `ConstraintId`, `ObjectiveId`, and constraint groups across derived models. Public evidence describes original constraints, variable bounds, sparse domains, and objective targets—not solver rows, auxiliary variables, or native handles. A global constraint expanded into many internal constraints still needs an explanation in terms of its original business rule.

Reports can start with domains or groups and expand into instances. However, grouping an instance-level minimal set does not automatically prove group-level minimality. Kotlin and Rust integrations follow the same objective-direction, status, tolerance, and evidence semantics without treating an optional backend capability as universal.

A conceptual report for the two production lines is:

```text
Baseline: total output 10, proven optimal
Target: total output at least 11
Background: both production quantities are nonnegative integers
Activity: line A capacity and line B capacity are active
Target result: unreachable
Blocking set: {line A capacity <= 5, line B capacity <= 5}
Minimality: proven inclusion-minimal relative to the stated background and target
Business explanation: the two capacities jointly prevent total output of 11
```

Also retain model and objective identities, perturbation parameters, solve statuses, local-sensitivity scope, and uncertainty from incomplete analysis. Repeating analysis at several improvement amounts can reveal recurring constraints, but a finite sample is not a structural proof for every target level.

This lets a report explain where a plan is constrained while distinguishing observed benefits, evidence that a specific target is unreachable, and adjustments that still require business decisions. See [Use Domain Driven Design Architecture](./use-ddd-architecture) for organizing the original constraints into business contexts.
