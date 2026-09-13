# Infeasibility Analysis and Constraint Relaxation

Infeasibility analysis asks why no plan satisfies all conditions. It shares methods with [Critical Constraint Analysis](./critical-constraint-analysis), which typically adds a better target to a feasible model and explains why that target is unreachable.

## 1. Distinguish the source of the problem

Missing inputs, wrong units, incorrect domains, and genuine business contradictions can all lead to failure. Establish that data and expressions match the business before explaining conflicts. A timeout without a solution is not infeasibility proof, nor is an unsupported backend a business contradiction.

A conflict shows that particular modeled conditions cannot coexist. It does not prove any individual rule is wrong. Relaxability depends on business meaning and authorization.

## 2. Example: demand exceeds internal capacity

### Overview, concepts, and variables

A single-period production context must meet demand $D=10$ items with internal capacity $K=8$. Variable $x\in\mathbb Z_{\ge0}$ is internal production. No auxiliaries or filtering predicates are needed; delivered quantity is intermediate $Q=x$.

### Assertions and constraints

Nonnegative integer demand and capacity are input assertions. The business constraints are:

$$
c_D:\ Q\ge10,\qquad c_K:\ x\le8.
$$

This is satisfaction diagnosis without an optimization objective. Background $B$ retains the nonnegative integer domain.

### Explanation

$c_D$ and $c_K$ cannot both hold. Without demand, $x=0$ is feasible; without capacity, $x=10$ is feasible. They therefore form an inclusion-minimal conflict relative to $B$.

This does not authorize changing capacity data to 10 or delivering two fewer items. It only explains incompatibility.

## 3. Conflicts, IIS, and MUS

A conflict set is sufficient for infeasibility but need not be minimal. A MUS is an inclusion-minimal unsatisfiable subset. An IIS in mathematical programming is an irreducible infeasible subsystem and may include rows and variable bounds.

Minimal does not mean minimum cardinality or uniqueness. Mapping a solver-row IIS to business names does not automatically establish business-constraint-level minimality. Splitting rules or grouping instances changes diagnostic granularity.

For explanation set $M$ and background $B$, minimality means:

$$
B\land\bigwedge_{c\in M}c\text{ is unsatisfiable},
$$

$$
\forall c\in M,\quad B\land\bigwedge_{d\in M\setminus\{c\}}d\text{ is satisfiable}.
$$

Unknown outcomes during repeated satisfaction solving leave minimality unresolved. A timeout is not unsatisfiability.

## 4. Original evidence and non-relaxable background

Business reports use rule identities, bounds, discrete domains, and instance indices. Auxiliaries and generated rows can aid internal diagnosis but explanations return to original definitions.

Background conditions always remain. If the background itself is infeasible, correction among adjustable candidates cannot fix it. Legal, safety, or physical rules may belong to the background, but that classification needs explicit provenance rather than an algorithmic guess.

## 5. From diagnosis to a relaxation model

### Permitted business changes

Suppose capacity cannot simply be inflated, but overtime for at most one item and outsourcing for at most two are permitted. Introduce $h\in\{0,1\}$ and $o\in\{0,1,2\}$ in items, representing extra capacity and outsourced quantity.

Delivery and additional cost are:

$$
Q=x+o,\qquad C=3h+5o.
$$

Require $0\le x\le8+h$, meet $Q\ge10$, and minimize $C$. Input assertions establish that the overtime and outsourcing limits are available. This explicitly adds resource options rather than deleting an original rule.

### Result and limits

Choosing $h=1,o=1,x=9$ costs 8. Without overtime, $o=2$ costs 10. Thus optimal additional cost is 8.

Diagnosis explains a two-item shortfall. The relaxation model identifies how permitted options cover it. Without approval, the result remains a recommendation, not an execution plan.

## 6. Correction sets and relaxation amounts

A minimal correction set (MCS) is an inclusion-minimal set of permitted conditions whose removal restores full-model feasibility. It is not the conflict set itself. Other conflicts may survive after one conflict is resolved.

Quantifiable constraints can receive penalized slack, but units and costs must be explicit. Reducing demand, increasing capacity, and allowing late delivery are different business actions even when each admits a slack formulation.

## 7. Organizing diagnosis in OSPF

Use separate models or snapshots so temporary deactivation and relaxation do not rewrite business definitions. Contexts retain stable constraint and group identities. Applications choose the background, candidates, and permitted changes. Backends supply supported conflict or satisfaction information.

Reports show conflicts, background, granularity, and uncertainty together. Several explanations may coexist; the first is not necessarily the only cause. See [Multi-Objective Optimization and Soft Constraints](./multi-objective-optimization) and [Scenario Analysis](./scenario-analysis).
