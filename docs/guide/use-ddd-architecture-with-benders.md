# Using DDD Architecture with Benders Decomposition

Benders decomposition keeps one group of decisions in a master problem and places the remaining decisions and difficult constraints in a subproblem. DDD preserves existing domain ownership across that split. The same Contexts can participate in a monolithic model, the master, or the subproblem through **selective registration**, while typed fixed-variable and cut protocols connect both sides.

## 1. Mathematical structure

Consider this minimization problem. $x$ normally contains master discrete decisions, while $y$ contains continuous recourse decisions after $x$ is fixed:

$$
\begin{aligned}
\min_{x,y}\quad
& c^\mathsf{T}x+d^\mathsf{T}y \\
\text{s.t.}\quad
& Ax \ge b,\\
& Tx+Wy \ge h,\\
& x\in X,\quad y\ge 0.
\end{aligned}
$$

After fixing $x=\bar x$, the subproblem is:

$$
Q(\bar x)=
\min_{y\ge0}
\left\{
d^\mathsf{T}y
\;\middle|\;
Wy\ge h-T\bar x
\right\}.
$$

Its dual is:

$$
Q(\bar x)=
\max_{\pi\ge0}
\left\{
\pi^\mathsf{T}(h-T\bar x)
\;\middle|\;
W^\mathsf{T}\pi\le d
\right\}.
$$

The master introduces $\theta$ to approximate the subproblem value function:

$$
\begin{aligned}
\min_{x,\theta}\quad
& c^\mathsf{T}x+\theta\\
\text{s.t.}\quad
& Ax\ge b,\quad x\in X,\\
& \theta\ge
\pi^\mathsf{T}(h-Tx),
&& \forall \pi\in\mathcal E,\\
& r^\mathsf{T}(h-Tx)\le0,
&& \forall r\in\mathcal R.
\end{aligned}
$$

$\mathcal E$ contains relevant extreme points of the dual feasible region and produces **optimality cuts**. $\mathcal R$ contains relevant extreme rays and produces **feasibility cuts**. The algorithm discovers them on demand rather than enumerating them first.

## 2. Choosing the decomposition boundary

The boundary is a mathematical decision before it is a code-organization decision. Good master candidates normally include:

- discrete decisions that determine the global combinatorial structure;
- relatively tight constraints that provide a useful lower bound;
- coupling variables that the subproblem must fix;
- a $\theta$ variable that underestimates recourse cost.

Good subproblem candidates normally include:

- continuous variables that can be optimized after the master decision is fixed;
- expensive but structurally stable feasibility or recourse constraints;
- a part that can generate valid cuts from a dual solution or infeasibility certificate;
- components separable by scenario, resource, or time period.

Do not move a package into the subproblem merely because it is named “security” or “capacity.” First establish the fixed-$x$ subproblem structure and the validity of its cuts.

## 3. Context responsibilities

| Component | Main responsibility | External protocol |
|---|---|---|
| Master Contexts | Register $x$, master rows, master objective, and $\theta$ | Master candidate solution |
| Subproblem Contexts | Register $y$, coupling rows, and recourse objective | Subproblem status, value, and certificate |
| Fixed-variable mapper | Map domain variables to candidate values | `fixedVariables` |
| Cut factory | Convert a dual solution/ray into a named domain cut | Feasibility or optimality cut |
| Application/algorithm service | Iterate, manage bounds, stop, guard quality, and fall back | Domain solution and diagnostics |

Domain objects own variables and intermediate values; a Context decides which model receives them:

~~~kotlin
class StowageContext {
    fun register(model: AbstractLinearMetaModel<Flt64>): Try = TODO()

    fun registerForBendersMP(
        model: AbstractLinearMetaModel<Flt64>
    ): Try = TODO()

    fun registerForBendersSP(
        model: AbstractLinearMetaModel<Flt64>,
        fixedVariables: Map<AbstractVariableItem<*, *>, Flt64>
    ): Try = TODO()
}
~~~

These entry points may share aggregates and pipelines, but each must document which variables, intermediate values, objectives, and constraints it registers. Correctness must not depend on one entry point having happened to run first.

## 4. Fixed-variable protocol

`fixedVariables` means “master domain variable → current candidate value,” not “master column number → subproblem column number”:

~~~kotlin
val fixedVariables =
    mutableMapOf<AbstractVariableItem<*, *>, Flt64>()

for (item in items.indices) {
    for (position in positions.indices) {
        fixedVariables[stowage.x[item, position]] =
            masterSolution[stowage.x[item, position]]
    }
}
~~~

The protocol should guarantee:

- both sides use the same domain-variable identity or an explicit stable key;
- every coupling variable needed by the subproblem has a value;
- discrete values use one rounding tolerance, while raw values remain available for diagnostics;
- fixing affects only the current subproblem iteration;
- missing, duplicate, or out-of-range values fail immediately rather than silently becoming zero.

Intermediate values can remain context interfaces, but registering every intermediate value in both meta-models does not automatically create a mapping. Master/subproblem communication must be explicit.

## 5. Selective registration

One domain context may participate in several solve paths:

| Path | Registration method | Purpose |
|---|---|---|
| Monolithic MILP | `register` | Full benchmark or fallback model |
| Benders master | `registerForBendersMP` | Master variables, tight rows, and master objective |
| Benders subproblem | `registerForBendersSP` | Recourse variables, fixed relations, and subproblem objective |

Implement selective registration in the Context or pipeline composition, not by copying formulas into the Application. A business mode may bind different implementations to the same intermediate-value semantics, but the published meaning must remain stable.

## 6. Iteration lifecycle

1. initialize domain objects and participating Contexts;
2. construct and register the master and subproblem separately;
3. solve the master for $\bar x$, $\bar\theta$, and a lower bound;
4. fix $\bar x$ in the subproblem through `fixedVariables`;
5. solve the subproblem and classify the result:
   - optimal: extract a dual extreme point and create an optimality cut;
   - infeasible: extract a Farkas certificate/extreme ray and create a feasibility cut;
   - unbounded, timeout, or solver error: use a separate failure policy;
6. recheck cut coefficients and sense in the domain layer, then add the cut to the master;
7. update bounds, gap, cut metrics, and the convergence trace;
8. return to step 3 until converged;
9. analyze the final master solution and recover $y$ from the subproblem when needed;
10. apply quality guards and, when policy permits, fall back to the complete MILP.

~~~kotlin
while (iteration < config.maxIterations) {
    val master = solveMaster()
    val fixed = mapFixedVariables(master)
    val sub = solveSubproblem(fixed)

    when (sub.status) {
        Optimal -> addOptimalityCut(cutFactory.fromDual(sub))
        Infeasible -> addFeasibilityCut(cutFactory.fromRay(sub))
        else -> return handleSubproblemFailure(sub)
    }

    updateBounds(master, sub)
    if (converged()) break
    iteration += 1
}
~~~

This pseudocode describes responsibilities only. Reliable dual values, Farkas certificates, and incremental row addition are necessary backend capabilities for a strict implementation.

## 7. Bounds, convergence, and duplicate cuts

For minimization, the master objective normally supplies a lower bound:

$$
LB_k = c^\mathsf{T}x^k+\theta^k.
$$

When the subproblem is feasible, the candidate supplies an upper bound:

$$
UB_k = c^\mathsf{T}x^k+Q(x^k).
$$

Absolute and relative gaps may be defined as:

$$
\operatorname{gap}_{abs}=UB-LB,
\qquad
\operatorname{gap}_{rel}
=\frac{UB-LB}{\max(1,|UB|)}.
$$

At termination, check all of the following:

- $LB$ and $UB$ are valid and ordered correctly;
- the gap satisfies configured tolerances;
- the latest cuts are not materially violated;
- no artificial slack or unacceptable fallback remains;
- numerical noise is not repeatedly adding the same cut.

A cut should have a normalized stable signature and record its source context, iteration, type, violation, and dual certificate.

## 8. Multiple subproblems

When the subproblem separates by scenario $s\in S$, choose among:

- **single-cut**: one $\theta$ and one aggregate cut; smaller master, often slower convergence;
- **multi-cut**: a $\theta_s$ and independent cuts per scenario; larger master, stronger information;
- **parallel solves**: one independent subproblem per scenario, with deterministic aggregation in the Application.

In every design, scenario probabilities, objective weights, and missing-scenario failure policy belong in the domain protocol, not only in thread-orchestration code.

## 9. Quality guards and fallback

A production application should not equate “solver returned success” with sufficient quality. Monitor:

- Benders gap over its limit;
- time or iteration limits;
- no progress over several rounds;
- weak bound improvement per cut;
- abnormal objective trajectory or repeated cuts;
- missing certificates or numerical instability in the subproblem.

When policy permits, failure or insufficient quality can fall back to a complete MILP. The fallback model must reuse the same Contexts through their `register` path and report the fallback reason; it must not assemble a second, unverified set of formulas.

## 10. Verification checklist

### Decomposition equivalence

- build both the complete MILP and Benders models on a small instance, then compare feasibility and optimal objective;
- for a fixed $\bar x$, compare the subproblem value with a manual $Q(\bar x)$ calculation;
- verify that the union of MP/SP selective registrations covers the complete model and that their intersection contains only intentionally shared semantics.

### Cut tests

- an optimality cut gives the correct lower bound at its generating point $\bar x$;
- a feasibility cut rejects the current infeasible $\bar x$ without rejecting known feasible solutions;
- dual signs, constant terms, and $T x$ coefficients follow backend conventions;
- naming, deduplication, and tolerance boundaries are stable.

### Lifecycle tests

- `fixedVariables` is complete and fresh, with a consistent integrality tolerance;
- each $LB$ is nondecreasing and incumbent $UB$ is nonincreasing for minimization, within tolerance;
- cover optimal, infeasible, unbounded, timeout, and backend-error outcomes separately;
- quality guards trigger the specified failure or MILP fallback path.

## 11. Common mistakes

- choosing the split from package names rather than the coupling matrix;
- treating an infeasible subproblem as an algorithm failure instead of generating a feasibility cut;
- mapping variables by positions in the master and subproblem models;
- duplicating MP/SP constraints in the Application;
- computing a gap from incomparable objective values;
- claiming strict Benders cuts when the backend provides no valid certificate;
- using different domain rules in the fallback model.

## 12. Related material

- [Complex Example 2: Aircraft Cargo Load Planning](/examples/framework-example2)
- [DDD architecture fundamentals](/guide/use-ddd-architecture)
- [Formal design and formal verification](/guide/formal-design-and-formal-verification)
