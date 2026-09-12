# Formal Design and Formal Verification

Optimization requirements already make extensive use of sets, quantifiers, equalities, and inequalities. They are therefore natural candidates for formal design before coding and for verification that domain rules, mathematical expressions, and program behavior agree afterward. The goal is not necessarily one machine-checked proof for the entire system. It is to give every important rule explicit premises, checkable semantics, and traceable evidence.

## 1. Three objects to align

Distinguish three levels for one business rule:

| Level | Notation | Example |
|---|---|---|
| Domain proposition | $p$ | A point on a triangulated surface has one interpolated height |
| Mathematical specification | $p_f:X\to\mathbb B$ | The point belongs to one triangle and its height is the barycentric interpolation |
| Executable implementation | $p_e:X\to\mathbb B$ | Triangle selectors, barycentric weights, and their linear constraints |

Let $K$ be the existing domain knowledge and premises. A new rule must first be consistent:

$$
\operatorname{SAT}(K\cup\{p_f\})
\quad\Longleftrightarrow\quad
\exists x\in X:
\left(
\bigwedge_{q\in K}q(x)
\right)\land p_f(x).
$$

Implementation correctness requires:

$$
K\models(p_f\Leftrightarrow p_e),
$$

or explicitly:

$$
\forall x\in X:
\left(
\bigwedge_{q\in K}q(x)
\right)
\Rightarrow
\bigl(p_f(x)\Leftrightarrow p_e(x)\bigr).
$$

A unit, bound, integrality condition, or data assumption that is absent from $K$ cannot be used silently in a proof or test.

## 2. Formal design workflow

### 2.1 Give the rule an identity

Assign a stable ID, domain name, owning Context, source, and version. A name such as `bandwidth.no_outflow_without_service` expresses business meaning; an implementation sequence number does not.

### 2.2 Declare the universe and premises

Specify:

- sets, indices, and empty-set behavior;
- decision-variable domains;
- parameter units, ranges, and null policy;
- finite bounds and how they were derived;
- conversion of strict inequalities;
- solver and business tolerances.

### 2.3 Write the domain predicate

First write a proposition independent of Big-M, auxiliary variables, or a solver. Split a sentence that contains several conjunctions, exceptions, or “unless” clauses into independently named predicates.

### 2.4 Derive the mathematical implementation

Two workflows are valid:

1. derive executable $p_e$ deductively from $K$ and $p_f$;
2. propose $p_e$, then prove $K\models(p_f\Leftrightarrow p_e)$.

The second workflow cannot stop at “this is a common formulation” or “the sample solves.” It must state the equivalence conditions, especially finite bounds and variable domains.

### 2.5 Map it to OSPF elements

- an Aggregation or domain Model owns variables;
- reusable definitions become named intermediate values;
- a named Pipeline registers constraints;
- the Context selects registrations for a business mode or solve path;
- the Application orchestrates without redefining formulas.

### 2.6 Establish traceability

Link the proposition ID, formula, Pipeline, tests, benchmark data, and change record. Any formula change should reveal the affected tests and consuming contexts.

## 3. Equivalence obligations

Logical equivalence separates into two directions:

$$
\underbrace{p_e(x)\Rightarrow p_f(x)}_{\text{soundness: the implementation accepts no domain-invalid solution}},
\qquad
\underbrace{p_f(x)\Rightarrow p_e(x)}_{\text{completeness: the implementation rejects no domain-valid solution}}.
$$

It can also be viewed as positive and negative specification branches:

| Specification $p_f$ | Implementation $p_e$ | Result |
|---|---|---|
| true | true | Positive branch passes |
| true | false | False negative; completeness violation |
| false | false | Negative branch passes |
| false | true | False positive; soundness violation |

Tests cannot cover only expected-feasible examples. Every constraint needs at least one candidate that violates that rule while satisfying as many other premises as possible.

## 4. Classic derivation I: bivariate linear piecewise function

This section follows the current triangulation-and-barycentric-interpolation implementation in ospf-kotlin. The goal is not merely to obtain approximately correct values at a few sampled points. Starting with the business semantics, it derives linear constraints from a logical disjunction and proves that every feasible symbolic solution describes exactly the piecewise-linear surface over the given triangulation.

### 4.1 Natural-language rule and premises

Let $\mathcal T$ be a non-empty set of triangles. The three three-dimensional vertices of triangle $t\in\mathcal T$ are:

$$
P_{tj}=(x_{tj},y_{tj},z_{tj}),
\qquad j\in\{1,2,3\}.
$$

For input $(x,y)$ and output $z$, the rule is:

- $(x,y)$ must lie in the two-dimensional projection of at least one triangle;
- after selecting a triangle that contains $(x,y)$, $z$ must equal the barycentric interpolation of its three vertex heights;
- if $(x,y)$ belongs to no triangle, the function is undefined there and the symbolic model must be infeasible.

Every projected triangle must be non-degenerate:

$$
D_t=
(x_{t2}-x_{t1})(y_{t3}-y_{t1})
-(x_{t3}-x_{t1})(y_{t2}-y_{t1})
\ne0.
$$

Triangles that share an edge or vertex must agree on $z$ at every shared position. Triangle interiors must not overlap unless their affine planes agree everywhere on the overlap. Otherwise, one $(x,y)$ may correspond to multiple $z$ values, producing a relation rather than a function.

### 4.2 Logical language

Do not introduce selectors yet. For every triangle $t$, define the predicate “the input and output lie on this piece”:

$$
\begin{aligned}
R_t(x,y,z)
\equiv{}&
\exists\lambda_{t1},\lambda_{t2},\lambda_{t3}\in\mathbb R_{\ge0}:\\
&\sum_{j=1}^{3}\lambda_{tj}=1\\
&\land\quad
x=\sum_{j=1}^{3}\lambda_{tj}x_{tj}\\
&\land\quad
y=\sum_{j=1}^{3}\lambda_{tj}y_{tj}\\
&\land\quad
z=\sum_{j=1}^{3}\lambda_{tj}z_{tj}.
\end{aligned}
$$

The first three conditions say that $(x,y)$ is a point in the projected triangle. The final condition interpolates the height with the same barycentric weights. The entire bivariate linear piecewise function is the logical disjunction of all triangle branches:

$$
p_f(x,y,z)
\equiv
\bigvee_{t\in\mathcal T}R_t(x,y,z).
$$

This definition also expresses the domain: if no $R_t$ holds, no legal output $z$ exists. If several predicates hold on a shared boundary, the surface-consistency premise ensures that all of them produce the same $z$.

### 4.3 Derivation of the linear inequalities

The “at least one branch holds” logic must become a mixed-integer linear model. Introduce a selector for every triangle:

$$
\delta_t\in\{0,1\}.
$$

**Step 1: activate exactly one piece.**

$$
\sum_{t\in\mathcal T}\delta_t=1.
$$

If the solver's standard form retains only inequalities, this equality is equivalent to:

$$
\sum_t\delta_t\le1,
\qquad
-\sum_t\delta_t\le-1.
$$

**Step 2: activate the corresponding barycentric weights.** Introduce $\lambda_{tj}\ge0$ for every vertex and impose:

$$
\sum_{j=1}^{3}\lambda_{tj}=\delta_t,
\qquad
\forall t\in\mathcal T.
$$

When $\delta_t=0$, non-negativity and the weight sum force every weight of that piece to zero. When $\delta_t=1$, the weights are non-negative and sum to one. The inequality-only form is:

$$
\sum_j\lambda_{tj}-\delta_t\le0,
\qquad
\delta_t-\sum_j\lambda_{tj}\le0.
$$

**Step 3: combine the coordinates and heights of all branches.**

Define:

$$
A_x(\lambda)=\sum_{t\in\mathcal T}\sum_{j=1}^{3}\lambda_{tj}x_{tj},
$$

$$
A_y(\lambda)=\sum_{t\in\mathcal T}\sum_{j=1}^{3}\lambda_{tj}y_{tj},
\qquad
A_z(\lambda)=\sum_{t\in\mathcal T}\sum_{j=1}^{3}\lambda_{tj}z_{tj}.
$$

Link the inputs and output:

$$
x=A_x(\lambda),
\qquad
y=A_y(\lambda),
\qquad
\hat z=A_z(\lambda).
$$

Each equality $v=A_v(\lambda)$ expands into a pair of linear inequalities:

$$
v-A_v(\lambda)\le0,
\qquad
A_v(\lambda)-v\le0,
\qquad
v\in\{x,y,\hat z\}.
$$

The logical disjunction has now been converted completely into binary variables, non-negative variables, and linear inequalities. No arbitrarily large Big-M is needed. In ospf-kotlin, `zVars` play the role of $\delta_t$ and `lambdaVars` play the role of $\lambda_{tj}$.

### 4.4 Equivalence proof

**Soundness $p_e\Rightarrow p_f$:** Since $\sum_t\delta_t=1$ and every $\delta_t$ is binary, exactly one triangle $t^\star$ is selected. For every $t\ne t^\star$:

$$
\sum_j\lambda_{tj}=\delta_t=0.
$$

Together with $\lambda_{tj}\ge0$, this makes every weight of an inactive triangle zero. The weights of $t^\star$ are non-negative and sum to one, and the input-output linking constraints therefore satisfy $R_{t^\star}(x,y,\hat z)$. Consequently, $p_f(x,y,\hat z)$ holds.

**Completeness $p_f\Rightarrow p_e$:** If $p_f(x,y,z)$ holds, at least one triangle $t^\star$ and one set of barycentric weights satisfy $R_{t^\star}(x,y,z)$. Set $\delta_{t^\star}=1$ and every other selector to zero; retain the weights of that piece and set all other weights to zero. This assignment satisfies every linear constraint.

Under the non-degeneracy and overlap-consistency premises $K_{\mathcal T}$, therefore:

$$
K_{\mathcal T}
\models
\left(
p_f(x,y,z)
\Leftrightarrow
p_e(x,y,z)
\right).
$$

Soundness ensures that the model accepts no point outside the domain or with an incorrect interpolated value. Completeness ensures that the linear model represents every surface point admitted by the logical specification.

### 4.5 Tests derived from the proof

Use the triangle:

$$
P_1=(0,0,0),\quad
P_2=(1,0,1),\quad
P_3=(0,1,1).
$$

The surface on this piece is $z=x+y$, so the correct result for $(x,y)=(0.25,0.25)$ is:

$$
z=0.5.
$$

At minimum, verify:

| Class | Input | Verification |
|---|---|---|
| Three vertices | $P_1,P_2,P_3$ | Weights are one-hot and the result equals the vertex height |
| Interior point | $(0.25,0.25)$ | Three weights are non-negative and sum to one; $z=0.5$ |
| Shared edge | One weight is zero | Adjacent pieces return the same $z$ and order does not affect the result |
| Outside domain | Point belongs to no triangle | Direct evaluation returns empty and the symbolic model is infeasible |
| Degenerate triangle | $D_t=0$ or nearly zero | Construction/evaluation rejects it according to contract, outside the proof domain |
| Overlapping pieces | One $(x,y)$ is in multiple interiors | Either reject them or prove equal heights throughout the overlap |
| Dual-path consistency | Same input | `evaluate` agrees with $\hat z$ after the solver fixes the inputs |

See [Bivariate Linear Piecewise Function](/guide/linear-functional/blp) for APIs and additional boundaries.

## 5. Classic derivation II: recommended load weight

This example focuses on the successive translation of one business rule into executable constraints. It does not address code reuse or implementation techniques for intermediate values.

### 5.1 Natural-language rule

For every loading position $p$:

- if the position is not selected for recommended loading, its recommended load weight must be zero;
- if it is selected, its recommended load weight must lie in the permitted interval $[\underline W_p,\overline W_p]$;
- the interval uses one consistent weight unit and satisfies $0<\underline W_p\le\overline W_p$.

The “if ... then ...” and interval clauses must first become logical propositions rather than jumping directly to a Big-M row.

### 5.2 Logical language

Define:

$$
u_p\in\{0,1\}
$$

to indicate whether position $p$ is selected, and let $w_p\in\mathbb R_{\ge0}$ be its recommended load weight. The domain predicate is:

$$
p_f(u_p,w_p)
\equiv
\left(u_p=0\land w_p=0\right)
\lor
\left(
u_p=1
\land
\underline W_p\le w_p\le\overline W_p
\right).
$$

Equivalently, it is the conjunction of two implications:

$$
u_p=0\Rightarrow w_p=0,
$$

$$
u_p=1\Rightarrow
\underline W_p\le w_p\le\overline W_p.
$$

Because $u_p$ is binary, the states are mutually exclusive and exhaustive. The positive lower bound also means $w_p>0$ if and only if $u_p=1$. If the business permits a selected position to carry a zero recommendation, use a zero lower bound and remove that biconditional interpretation.

### 5.3 Derivation of the linear inequalities

First consider the upper bound. The two logical states require:

$$
u_p=0:\quad w_p\le0,
$$

$$
u_p=1:\quad w_p\le\overline W_p.
$$

Since $u_p\in\{0,1\}$, both cases combine into:

$$
w_p\le\overline W_pu_p.
$$

For the lower bound, the two states require:

$$
u_p=0:\quad w_p\ge0,
$$

$$
u_p=1:\quad w_p\ge\underline W_p.
$$

They combine into:

$$
w_p\ge\underline W_pu_p.
$$

The final linear implementation is therefore:

$$
p_e(u_p,w_p)
\equiv
\underline W_pu_p
\le w_p\le
\overline W_pu_p.
$$

Here $\overline W_p$ is not an arbitrarily large Big-M. It is the valid business upper bound for position $p$; $\underline W_p$ is likewise part of the specification.

### 5.4 Equivalence proof

**Soundness $p_e\Rightarrow p_f$:**

- If $u_p=0$, the linear row becomes $0\le w_p\le0$, so $w_p=0$.
- If $u_p=1$, it becomes $\underline W_p\le w_p\le\overline W_p$.

Both binary branches satisfy the domain predicate, so the implementation accepts no rule-violating solution.

**Completeness $p_f\Rightarrow p_e$:**

- In the domain state $u_p=0\land w_p=0$, substitution gives $0\le0\le0$.
- In the domain state $u_p=1\land\underline W_p\le w_p\le\overline W_p$, substitution yields exactly that interval.

Under the premises

$$
u_p\in\{0,1\},
\qquad
0<\underline W_p\le\overline W_p,
$$

we therefore have:

$$
K_{\mathrm{weight}}
\models
\left(
p_f(u_p,w_p)
\Leftrightarrow
p_e(u_p,w_p)
\right).
$$

### 5.5 Tests derived from the proof

For $\underline W_p=100\,\mathrm{kg}$ and $\overline W_p=500\,\mathrm{kg}$, cover at least:

| $u_p$ | $w_p$ | Expected | Proof branch |
|---:|---:|---|---|
| 0 | $0$ | Feasible | Positive unselected case |
| 0 | $\delta$ | Infeasible | Negative unselected case |
| 1 | $100$ | Feasible | Closed lower boundary |
| 1 | $300$ | Feasible | Interval interior |
| 1 | $500$ | Feasible | Closed upper boundary |
| 1 | $100-\delta$ | Infeasible | Below lower bound |
| 1 | $500+\delta$ | Infeasible | Above upper bound |

$\delta$ must exceed the solver's feasibility tolerance. Also verify that:

- the equivalence test fails if $u_p$ is incorrectly relaxed to a continuous variable;
- inconsistent units among the bounds and weight are rejected or converted before model construction;
- data with $\underline W_p>\overline W_p$ is rejected during initialization;
- a small multi-position instance compares the truth values of $p_f$ and $p_e$ point by point;
- Kotlin and Rust solver tests reach the same feasibility verdict after fixing $u_p,w_p$.

## 6. Five verification layers

| Layer | Subject | Recommended evidence |
|---|---|---|
| 1. Domain predicate | Natural language agrees with the formal specification | Domain review, truth table, counterexamples |
| 2. Mathematical derivation | $K\models(p_f\Leftrightarrow p_e)$ | Algebraic proof, SMT/exhaustion, small counterexample model |
| 3. Symbol construction | OSPF expression equals $p_e$ | Snapshot of coefficients, constants, bounds, and row sense |
| 4. Solver contract | Compiled model implements symbol semantics | Tiny feasible/infeasible models and multi-backend tests |
| 5. Application behavior | Context composition and analysis are correct | Benchmarks, regression suite, and domain invariants |

Passing layer 3 does not prove layer 2. Obtaining the expected objective from a solver does not replace negative-branch verification.

## 7. Consistency, redundancy, and conflict

Before adding $p$, check these separately.

### Consistency

$$
\operatorname{SAT}(K\cup\{p\}).
$$

If unsatisfiable, report an unsatisfiable core or minimal conflicting rule set using domain names.

### Redundancy

$$
K\models p.
$$

A redundant constraint does not change the feasible region. It may remain as an LP strengthening, readable invariant, or diagnostic aid, but record that purpose instead of treating it as new business knowledge.

### Conflict

$$
K\models\neg p.
$$

The new rule rejects every currently feasible case. If that is an intended business change, replace or version the affected rules and benchmarks rather than simply stacking it on top.

## 8. Numerical semantics

A formal specification uses exact relations; a floating-point solver uses tolerances. A verification report should state:

- the specification relation, such as $g(x)\le b$;
- solver tolerance $\varepsilon_s$;
- test-oracle tolerance $\varepsilon_t$;
- business tolerance $\varepsilon_b$;
- scaling and units.

Do not use a value inside the tolerance gray zone as the only negative test. For an equality, cover $b-\delta$, $b$, and $b+\delta$, and state which are business-valid, mathematically valid, and solver-accepted.

## 9. Verification in decomposition algorithms

### Column generation

Prove that the feasible columns produced by Pricing have the same definition as columns accepted by Master, and verify:

$$
\bar c_p
=c_p-\sum_i\pi_i a_{ip}
$$

with one cost, coefficient, and dual-sign convention on both sides. Compare against complete column enumeration on small instances.

### Benders

Prove that selective registration is equivalent to the complete model, fixed-variable mapping preserves semantics, and every cut is valid for all feasible original solutions. A feasibility cut must reject the current infeasible master candidate; an optimality cut must provide the correct recourse lower bound at its generating point.

## 10. Change control

When a rule changes:

1. update its domain proposition, premises, and version;
2. recheck consistency, redundancy, and affected deductions;
3. update the mathematical derivation and intermediate-value interfaces;
4. modify the Pipeline or function-symbol implementation;
5. update tests from the new proof branches;
6. run Context, complete-model, decomposition, and backend regressions;
7. record whether the feasible region or objective semantics changed.

An implementation optimization that preserves the truth set of $p_e$ is a semantics-preserving refactor. A changed truth set is a domain-rule change.

## 11. Definition of done

A rule is complete only when:

- it has a stable ID, owner, and business description;
- premises, units, domains, and tolerances are explicit;
- both the mathematical specification and executable expression are recorded;
- both equivalence directions have a proof or sufficient checkable evidence;
- positive, negative, boundary, and invalid-input tests exist;
- documentation traces to intermediate values, Pipeline, tests, and benchmarks;
- contract tests pass on every solver backend used in production.

## 12. Related material

- [Deductive logic expression in mathematical models](/guide/deductive-logic-expression)
- [DDD architecture fundamentals](/guide/use-ddd-architecture)
- [DDD with column generation](/guide/use-ddd-architecture-with-column-generation)
- [DDD with Benders decomposition](/guide/use-ddd-architecture-with-benders)
