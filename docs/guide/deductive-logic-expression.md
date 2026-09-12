# Deductive Logic Expression in Mathematical Models

A conventional mathematical program is normally written as an objective and a set of equalities or inequalities. That representation is ideal for a solver, but it does not fully express why a business rule holds, which premises it needs, or what it implies. A deductive-logic representation treats every constraint as a **proposition function** over candidate solutions, giving domain knowledge, mathematics, and executable code a shared semantics.

## 1. From standard form to predicate form

A conventional linear model can be written as:

$$
\begin{aligned}
\min/\max\quad & c^\mathsf{T}x\\
\text{s.t.}\quad & Ax\ \rho\ b,\\
& x\in X,
\end{aligned}
$$

where each row of $\rho$ is $\le$, $=$, or $\ge$. The deductive form does not require every rule to already be a linear inequality:

$$
\operatorname{opt}_{x\in X} f(x)
\quad\text{s.t.}\quad
\bigwedge_{p\in C} p(x).
$$

- $X$: the candidate universe, including variable types, domains, and modeling premises;
- $f:X\to\mathbb R$: the objective evaluation function;
- $C$: a set of constraint predicates;
- $p:X\to\{\mathsf{true},\mathsf{false}\}$: the truth of one domain proposition for a candidate.

The feasible set is:

$$
F(C)=
\left\{
x\in X
\;\middle|\;
\bigwedge_{p\in C}p(x)
\right\}.
$$

The optimum set of a minimization problem is:

$$
\operatorname{Best}_{\min}(f,C)=
\left\{
x\in F(C)
\;\middle|\;
\forall x'\in F(C),\ f(x)\le f(x')
\right\}.
$$

For maximization, replace the final $\le$ with $\ge$. This definition separates “feasible” from “at least as good as every other feasible candidate,” which supports reasoning about optimality, critical constraints, and infeasibility.

## 2. A concrete constraint is also a predicate

A concrete mathematical-programming constraint can be represented by:

$$
p=(g,\rho,b),
\qquad
p(x)\equiv[g(x)\ \rho\ b],
$$

where $g:X\to\mathbb R$ is a symbolic expression and $\rho\in\{\le,=,\ge\}$. For example:

$$
\sum_{i\in I}w_i x_i\le W
$$

corresponds to:

$$
p_{\mathrm{capacity}}(x)
\equiv
\left[
\sum_{i\in I}w_i x_i\le W
\right].
$$

During domain analysis, the predicate can remain abstract:

$$
p_{\mathrm{capacity}}(x)
\equiv
\text{“solution }x\text{ does not exceed resource capacity.”}
$$

Formal design must prove that the concrete numerical predicate is equivalent to this domain predicate under declared premises. It is not enough to replace the sentence with a plausible formula.

## 3. Connectives and quantifiers

Standard logic composes domain rules:

| Form | Meaning | Modeling example |
|---|---|---|
| $p\land q$ | Both rules hold | Capacity and time windows both hold |
| $p\lor q$ | At least one holds | Use an owned or outsourced resource |
| $\neg p$ | The rule does not hold | Forbid a combination |
| $p\Rightarrow q$ | The consequent is required when the antecedent holds | An active server may carry flow |
| $p\Leftrightarrow q$ | Both sides have the same meaning | An indicator matches a business state |
| $\forall i\in I:p_i$ | Every object satisfies the rule | Every demand is covered |
| $\exists i\in I:p_i$ | At least one object satisfies it | Select at least one plan |

OSPF logical function symbols can compile some propositions into linear or quadratic models, but logical semantics and numerical implementation should remain separately documented. Big-M constants, auxiliaries, and bounds are compilation choices, not the domain rule itself.

## 4. Premises, definitions, and constraints

Distinguish three kinds of statements:

1. **Premises/assumptions** define the candidate universe $X$, such as nonnegative demand, unique indices, and consistent capacity units;
2. **Definitions** name a domain concept, such as “node assigned” being the sum of assignment variables;
3. **Constraints** filter feasible candidates, such as at most one server on each node.

Encoding a premise as a decision constraint can conceal invalid input data. Leaving a definition as an anonymous local expression loses cross-context reuse and traceability.

Defining an intermediate value $z$ means:

$$
\forall x\in X,\qquad
z(x)=g(x).
$$

A named intermediate value is therefore not an approximate cache. It is an equivalent definition that other predicates may reference.

## 5. Service-placement example

Let $x_{is}\in\{0,1\}$ indicate whether service $s$ is placed on node $i$. The Route context publishes:

$$
\operatorname{NodeAssigned}_i
=\sum_{s\in S}x_{is},
\qquad
\operatorname{ServiceAssigned}_s
=\sum_{i\in N}x_{is}.
$$

Two domain rules become:

$$
\begin{aligned}
p_{\mathrm{node}}(x)
&\equiv
\forall i\in N:
\operatorname{NodeAssigned}_i\le1,\\
p_{\mathrm{service}}(x)
&\equiv
\forall s\in S:
\operatorname{ServiceAssigned}_s\le1.
\end{aligned}
$$

In the Bandwidth context, “a node cannot produce net outflow unless it hosts a service” is:

$$
\forall i\in N:\quad
\neg\operatorname{Deployed}_i
\Rightarrow
\operatorname{OutFlow}_i=0.
$$

If $\operatorname{Deployed}_i$ is equivalent to the binary state derived from $\operatorname{NodeAssigned}_i$ and $0\le\operatorname{OutFlow}_i\le U_i$ is known, the familiar linear form follows:

$$
\operatorname{OutFlow}_i
\le U_i\operatorname{Deployed}_i.
$$

The bound $U_i$ must be valid and justified. An arbitrary huge M can be numerically unstable and, when too small, logically nonequivalent.

## 6. Deduction in a knowledge base

Let $K$ be the existing predicate set and $q$ a conclusion. Define:

$$
K\models q
\quad\Longleftrightarrow\quad
\forall x\in X:
\left(
\bigwedge_{p\in K}p(x)
\right)\Rightarrow q(x).
$$

This relation answers:

- **implication**: whether existing rules entail a conclusion;
- **redundancy**: if $K\models q$, adding $q$ does not change the feasible region;
- **conflict**: if $K\models\neg q$, the new rule is incompatible with current knowledge;
- **equivalence**: $K\models(q\Leftrightarrow r)$ means two expressions agree under the premises;
- **refinement**: an implementation predicate is more concrete while preserving required equivalence.

Consistency is satisfiability, not merely the absence of an obvious pairwise conflict:

$$
\operatorname{SAT}(K)
\quad\Longleftrightarrow\quad
\exists x\in X:
\bigwedge_{p\in K}p(x).
$$

## 7. From a symbolic constraint to an executable predicate

Tests, callbacks, and diagnostics can evaluate a concrete inequality as a predicate. Floating-point code must use an explicit tolerance:

~~~kotlin
fun satisfied(
    lhs: Flt64,
    relation: Relation,
    rhs: Flt64,
    tolerance: Flt64
): Boolean = when (relation) {
    LessEqual -> lhs <= rhs + tolerance
    Equal -> abs(lhs - rhs) <= tolerance
    GreaterEqual -> lhs + tolerance >= rhs
}
~~~

Keep these concepts separate:

- **exact semantics**: $g(x)\rho b$ in documentation and proof;
- **solver tolerance**: backend primal-feasibility rules;
- **business tolerance**: acceptable operational deviation;
- **display precision**: number formatting.

One “number of displayed decimal places” cannot replace all four.

## 8. Traceable representation

Maintain a record for each rule:

| Field | Example |
|---|---|
| Domain statement | At most one service is placed on each node |
| Predicate ID | `route.node_assignment` |
| Preconditions | Node and service sets are deduplicated |
| Formal predicate | $\forall i,\sum_s x_{is}\le1$ |
| Intermediate values | $\operatorname{NodeAssigned}_i$ |
| Executable owner | `NodeAssignmentLimit` pipeline |
| Evidence | Boundary unit tests, model snapshot, and small known optimum |

This mapping lets requirements, mathematical, code, and test reviews discuss the same semantics.

## 9. Boundaries of the method

- A predicate representation does not turn a general MILP into an automatically proved theorem.
- A solver's feasible status means the compiled constraints hold within numerical tolerance; it does not mean the domain rules are complete.
- Logical equivalence is meaningful only under an explicit universe $X$ and premises $K$.
- Linearizing nonlinear logic requires finite bounds and correct strict/non-strict boundary treatment.
- A callback that observes only the current candidate must handle missing values, nonintegral candidates, and solve-stage differences.

## 10. Next steps

- [Formal design and formal verification](/guide/formal-design-and-formal-verification)
- [DDD architecture fundamentals](/guide/use-ddd-architecture)
- [Linear logical function symbols](/guide/linear-functional/and)
