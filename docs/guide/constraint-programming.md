# Constraint Programming and Global Constraints

Constraint programming (CP) is useful when the decisions are naturally discrete and the rules are easier to state as relations than as a large collection of linear rows. A CP model declares finite domains, global constraints, and an objective. The solver then combines propagation, search, and objective bounds to find or prove a schedule, assignment, packing, or routing plan.

This page is a mathematical tutorial. It describes the meaning that an OSPF model should preserve; it is not a list of Kotlin or Rust API calls.

## 1. When finite-domain CP is a good fit

Use finite-domain CP when a decision has a small or explicitly bounded set of alternatives:

- a start time measured in integer minutes or shifts;
- a worker, room, vehicle, or machine selected from a finite set;
- a sequence position, color, or rank;
- an optional activity with a presence decision.

CP is especially expressive when several decisions interact through global rules. A single `AllDifferent` constraint can describe a permutation, `NoOverlap` can describe a disjunctive machine, and `Cumulative` can describe a renewable resource profile. The global form communicates the business rule directly and can let a CP solver propagate more strongly than an ad-hoc encoding.

Finite does not mean that every possible value must be listed one by one. A domain such as $\{0,1,\ldots,480\}$ is finite and compact. What matters is that the lower and upper bounds, integrality, and any holes in the domain are part of the model's declared semantics.

## 2. Core definitions

### 2.1 Finite-domain variables

A finite-domain variable $x$ has a declared set $D_x$. For example,

$$
x\in D_x=\{0,1,2,3,4\}.
$$

An interval notation such as $x\in\{0,\ldots,6\}$ means integer values, not every real number in $[0,6]$. A Boolean variable is the special case $x\in\{0,1\}$. The solver's search may temporarily narrow a domain, but a narrowed search state is not a change to the business model.

### 2.2 `AllDifferent`

For variables $x_1,\ldots,x_n$,

$$
\operatorname{AllDifferent}(x_1,\ldots,x_n)
\quad\Longleftrightarrow\quad
\forall i<j:\ x_i\ne x_j.
$$

It is often used for worker slots, permutation positions, or one-to-one assignments. It does not by itself say that every value in a larger universe is used, nor does it say that a task is assigned to exactly one resource unless the variable domains and other constraints give that meaning.

### 2.3 `NoOverlap`

For a positive-duration interval activity $i$, let $s_i$ be its start, $p_i>0$ its duration, and its end

$$
e_i=s_i+p_i.
$$

The usual interval is half-open, $[s_i,e_i)$. For activities on one disjunctive resource,

$$
\operatorname{NoOverlap}(I_1,\ldots,I_n)
\quad\Longleftrightarrow\quad
\forall i<j:\ e_i\le s_j\ \lor\ e_j\le s_i.
$$

Thus one activity may finish exactly when another starts. If a model admits zero-duration activities, it must state separately whether their empty intervals participate in ordering or resource conventions. If activities are optional, the semantic definition also includes their presence literals: two absent activities impose no ordering, and an absent activity contributes no occupied time.

### 2.4 `Cumulative`

Let activity $i$ have demand $r_i\ge0$ and a common capacity $C$. At every time $t$,

$$
\operatorname{Cumulative}(I_i,r_i,C)
\quad\Longleftrightarrow\quad
\sum_{i:\ s_i\le t<e_i}r_i\le C.
$$

For integer time, it is enough to check each relevant time bucket. A cumulative resource can allow overlap: two activities of demand one may run together on a capacity-two resource, while an activity of demand two cannot overlap either of them if the capacity is two.

## 3. Worked example: a small resource-constrained schedule

### 3.1 Business statement and data

Four jobs must be scheduled. Starts are integer time units. Jobs $A$ and $B$ share a machine, so they may not overlap. All jobs consume a power resource of capacity $3$. Job $A$ must finish before job $C$ starts. Each job receives a distinct worker slot in this teaching instance. The goal is to minimize the makespan.

| job $i$ | duration $p_i$ | power demand $r_i$ | worker domain |
|---|---:|---:|---|
| $A$ | $2$ | $2$ | $\{1,2,3,4\}$ |
| $B$ | $3$ | $2$ | $\{1,2,3,4\}$ |
| $C$ | $2$ | $1$ | $\{1,2,3,4\}$ |
| $D$ | $1$ | $1$ | $\{1,2,3,4\}$ |

The start domains are $s_i\in\{0,1,\ldots,6\}$. The makespan has the safe finite domain $T\in\{0,1,\ldots,10\}$. The worker variables are $w_i\in\{1,2,3,4\}$.

### 3.2 Variables, intermediates, and constraints

Decision variables:

$$
s_i\in\{0,\ldots,6\},\qquad
w_i\in\{1,2,3,4\},\qquad
T\in\{0,\ldots,10\}.
$$

Derived end variables are defined by

$$
e_A=s_A+2,\quad e_B=s_B+3,\quad e_C=s_C+2,\quad e_D=s_D+1.
$$

The rules are

$$
\operatorname{AllDifferent}(w_A,w_B,w_C,w_D),
$$

$$
e_A\le s_C,
$$

$$
e_A\le s_B\ \lor\ e_B\le s_A,
$$

$$
\sum_{i:\ s_i\le t<e_i}r_i\le3
\qquad\text{for every relevant }t,
$$

$$
T\ge e_i\qquad(i\in\{A,B,C,D\}).
$$

The last row is an epigraph for the interval ends. Under the stated minimization objective, its optimal value equals $\max_i e_i$; in a merely feasible solution, $T$ may be larger. The objective is

$$
\min T.
$$

The power rule is not a second machine-order rule. It is a time-indexed capacity rule: power alone would allow $A$ and $C$ to overlap because $2+1=3$, but the separate precedence rule $e_A\le s_C$ forbids that overlap in this instance. Jobs $A$ and $B$ cannot overlap both because of `NoOverlap` and because their combined demand would be $4>3$.

### 3.3 A hand-checkable candidate

Consider the following values:

| job | $s_i$ | $e_i$ | $w_i$ | $r_i$ |
|---|---:|---:|---:|---:|
| $A$ | $0$ | $2$ | $1$ | $2$ |
| $B$ | $2$ | $5$ | $2$ | $2$ |
| $C$ | $2$ | $4$ | $3$ | $1$ |
| $D$ | $0$ | $1$ | $4$ | $1$ |

The worker values are $1,2,3,4$, so `AllDifferent` holds. The machine intervals for $A$ and $B$ are $[0,2)$ and $[2,5)$, so they meet at an endpoint and do not overlap. Since $e_A=2=s_C$, the precedence rule holds.

The resource profile is easy to check by time bucket:

| time bucket | active jobs | load |
|---|---|---:|
| $[0,1)$ | $A,D$ | $2+1=3$ |
| $[1,2)$ | $A$ | $2$ |
| $[2,3)$ | $B,C$ | $2+1=3$ |
| $[3,4)$ | $B,C$ | $2+1=3$ |
| $[4,5)$ | $B$ | $2$ |

Every load is at most $3$. The end values imply $T=5$, and all jobs finish by that value.

### 3.4 Why $T=5$ is optimal

The two machine activities have a disjunction. If $B$ is before $A$, then $B$ needs three units, $A$ needs two units, and $C$ must follow $A$ for another two units. Starting no earlier than zero gives $T\ge3+2+2=7$.

If $A$ is before $B$, then $B$ cannot start before $s_A+2$ and therefore cannot finish before $s_A+5$. Because $s_A\ge0$, $T\ge5$. The candidate above realizes $T=5$; hence it is optimal. This proof does not depend on a solver-specific search trace.

## 4. Propagation is part of the value of the global form

A solver may derive consequences before branching. For this example, choosing $A$ before $B$ immediately gives a lower bound of $5$ on $T$; assigning a worker value removes that value from the other worker domains; and a power demand of two prevents a simultaneous demand of two on capacity three. These are propagation effects, not extra business rules.

It is important to distinguish:

- the original global constraint, such as `NoOverlap`;
- any backend representation used to solve it;
- evidence returned to a user, such as the occupied intervals and maximum resource load.

If a global constraint is expanded into pairwise rows or auxiliary variables, those rows are implementation details. A diagnostic report should still identify the original machine or power rule.

## 5. Conceptual organization in OSPF

An OSPF model can organize this problem around semantic responsibilities:

1. An input or scheduling context owns job durations, resource demands, worker availability, and the time unit.
2. A decision context owns starts, ends, assignments, optional presence, and the finite domains that make those decisions meaningful.
3. A rule context owns precedence, machine disjunctions, `AllDifferent`, and cumulative capacity. It should expose the original business identity of each rule.
4. An objective context owns the makespan definition and direction.
5. A result context records the witness values, objective value, feasibility or optimality status, and any bound or tolerance used for interpretation.

The compiler boundary can translate the semantic global constraints to a backend representation when that backend supports them, or to an exact reformulation when one is appropriate. The translation must preserve interval endpoints, presence semantics, domains, and constraint identity. The [compiler architecture](./compiler-architecture) page gives the broader separation between semantic models and backend mechanisms, while [solving results](./solving-results) describes what a result should prove and what it should leave unknown. This list is a conceptual organization, not a claim that each item is a current public class.

For critical-constraint explanations, report the original `NoOverlap`, `Cumulative`, or precedence rule rather than an arbitrary internal row. A global rule may have no single scalar slack. See [critical constraint analysis](./critical-constraint-analysis) for the distinction between activity, effectiveness, and a target-relative blocking explanation.

## 6. Common modeling pitfalls

### 6.1 Mixing closed and half-open intervals

With $[s,e)$, an activity ending at $t=10$ does not occupy bucket $10$. If one part of a model uses $s\le t<e$ and another treats the endpoint as occupied, schedules can appear to violate capacity by one time unit. State the convention once and use it everywhere.

### 6.2 Declaring a domain that is too small or accidentally continuous

A start domain ending at $6$ excludes a valid start at $7$; a real interval $[0,6]$ allows fractional starts that an integer-time schedule did not intend. Domain bounds are model data, not merely search hints. If time is measured in minutes, convert all durations, deadlines, and tolerances to minutes before declaring domains.

### 6.3 Forgetting presence semantics for optional work

An optional interval needs a presence decision. Its end relation, precedence, machine occupancy, and cumulative demand must be conditioned on presence. Merely giving an interval a nullable label while leaving its constraints unconditional can make an absent job consume capacity.

### 6.4 Confusing `AllDifferent` with assignment coverage

`AllDifferent(w_i)` says that the selected worker values differ. If five jobs may use ten workers, it does not require every worker to receive a job. If a worker can process several non-overlapping jobs, an `AllDifferent` assignment rule may be too strong; use a resource or per-worker `NoOverlap` semantics instead.

### 6.5 Treating cumulative capacity as a pairwise rule

Pairwise non-overlap is stronger than a capacity profile in some cases and weaker in others. A capacity-three resource permits three demand-one jobs together, but a pairwise rule would incorrectly forbid them. Conversely, a demand-two job and a demand-one job can overlap on capacity three even though two machine operations might still require `NoOverlap` for a separate reason.

### 6.6 Using an objective to repair an incomplete constraint

Minimizing $T$ makes the makespan tight in an optimal solution, but it does not make an omitted precedence or resource rule true. Derived values that must be exact should have a defining relation or a proven objective role. Do not rely on a favorable objective to enforce a business invariant.

### 6.7 Assuming every global constraint has a useful slack

The numeric residual of one internal linearization row is not a universal slack for `AllDifferent` or `NoOverlap`. Use semantic evidence such as a conflicting pair, a maximum resource load, or a violated domain, and label activity separately from objective impact.

## 7. Related pages

- [Critical constraint analysis](./critical-constraint-analysis)
- [Inequality indicator](./linear-functional/inequality)
- [Slack](./linear-functional/slack)
- [If-Then](./linear-functional/if-then)
- [Solving results](./solving-results)
- [Compiler architecture](./compiler-architecture)
- [Operations-research language](./operations-research-language)
- [Modeling workflow](./modeling-workflow)
- [Symbolic expressions](./symbolic-expressions)
- [Rolling optimization](./rolling-optimization)
- [Infeasibility analysis](./infeasibility-analysis)
- [Scenario analysis](./scenario-analysis)
