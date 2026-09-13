# Understanding Solver Results

A solve returns status, candidate plans, and proof information—not just an objective value. Business systems should establish what evidence exists before displaying a result, continuing computation, or adopting a plan.

## 1. Feasible, optimal, and unknown

A feasible solution satisfies all modeled conditions. Optimality additionally establishes that no better feasible solution exists. An incumbent is the best feasible solution retained so far, not automatically a proven optimum.

| Outcome | Available information | Unsupported conclusion |
|---|---|---|
| Proven optimal | Solution and objective, with optimality under stated tolerances | Zero error in exact real arithmetic |
| Feasible, optimality unproven | Candidate, value, and available bounds | No better plan exists |
| Proven infeasible | This model has no feasible plan | The real business can never be served |
| Proven unbounded | Objective improvement has no finite limit along feasible choices | An infinite-profit business plan should be returned |
| Unknown or interrupted without a solution | Termination reason and possibly bounds | Infeasibility, or feasibility of a zero plan |

An “infeasible or unbounded” outcome must not be arbitrarily resolved to either. Execution failures, unsupported models, and license errors are not mathematical infeasibility.

## 2. Bounds and gaps

For minimization, a feasible objective gives an upper bound $U$ and proof procedures give a lower bound $L$. For maximization, feasible objectives give $L$ and proof procedures give $U$. With consistent semantics:

$$
L\le z^*\le U,\qquad g_{\mathrm{abs}}=U-L.
$$

Relative-gap definitions depend on the backend. One reporting convention could be:

$$
g_{\mathrm{rel}}=\frac{U-L}{\max(s,|z_{\mathrm{inc}}|)},\qquad s>0.
$$

Here $s$ uses the same units as the objective. This illustrative convention does not replace a backend's native definition. Near-zero or negative objectives and constant objective shifts need particular care. Missing bounds are not zero, and missing gaps are not 0%.

## 3. Example: an intermediate maximization result

### Model

Products are $\{A,B\}$, with nonnegative integer production variables $x_A,x_B$ in items. Labor usage and return are:

$$
U_{\mathrm{labor}}=2x_A+3x_B,\qquad R=3x_A+4x_B.
$$

Positive unit labor is a data assertion. No further predicates or auxiliary variables are needed. The constraint and objective are:

$$
\text{s.t.}\quad U_{\mathrm{labor}}\le8,\qquad \max R.
$$

### Interpretation

Suppose the incumbent is $(x_A,x_B)=(1,2)$ with return 11 and a valid upper bound is 12. Then $11\le z^*\le12$, with absolute gap 1. Finding $(4,0)$ gives return 12 and, together with the bound, proves optimality.

The return-11 plan is a candidate, not proof that maximum return is 11. If no upper bound was supplied, report only the known return and status rather than inventing a bound of 12.

## 4. Time limits and business acceptability

A time limit is a termination reason, not a feasibility status. A feasible plan may exist at termination, or there may be bounds without a plan. Adoption depends on business quality thresholds, timeliness, and safety requirements; default values must not make that decision implicitly.

After rounding floating-point results to integers, recheck constraints and objectives. Independent rounding can break resource limits, flow balances, or logical conditions. Model feasibility also says nothing about business conditions omitted from the model.

## 5. Keep context when comparing plans

Comparisons need consistent objective direction, units, input snapshots, and business scope. Equal-objective solutions may differ in assignments, stability, or fairness. Important differences need explicit indicators or secondary objectives.

Across different inputs, objective changes may reflect demand or capacity rather than a better algorithm. Two unproven incumbents do not establish the global benefit of changing a rule. See [Scenario Analysis](./scenario-analysis) and [Critical Constraint Analysis](./critical-constraint-analysis).

## 6. From solver results to business reports

Retain model identity, status, solution availability, objective and units, bounds, gap convention, termination reason, and business indicators. Fields without supporting evidence remain absent or unknown.

[Infeasibility Analysis](./infeasibility-analysis) explains why a model has no solution; [Multi-Objective Optimization](./multi-objective-optimization) explains results under different business priorities.
