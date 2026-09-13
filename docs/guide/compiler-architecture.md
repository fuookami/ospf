# Compiler-Like Architecture and Model Transformation

A compiler-like architecture organizes business expressions, mathematical representations, backend execution, and business results. The analogy explains separation of responsibilities; it does not make modeling synonymous with text compilation or solving with code generation.

## 1. Frontends, intermediate representations, and backends

| Layer | Retained information | Responsibility |
|---|---|---|
| Business frontend | Entities, parameters, rule identities, units, contexts | Bind business definitions to mathematics |
| Symbolic and modeling layer | Variables, expressions, function symbols, constraints, objectives | Compose, check, and organize semantics |
| Solver-level representation | Domains and relations for a target model class | Explicit transformations and executable auxiliary structures |
| Backend adapter | Solver-specific objects and settings | Match capabilities, load models, solve, read status |
| Result mapping | Original identities and transformation mappings | Return solutions and diagnostics to business objects |

There need not be a single intermediate representation. Sparse linear coefficients, quadratic terms, and native CP relations differ; not every model should become a linear matrix first.

## 2. Internal DSLs do not reparse host code

Kotlin/Rust syntax is already handled by the host compiler. Modeling DSLs normally construct expressions through objects and operations, without lexing their source strings again.

Model-level semantic checks remain necessary: references must belong to this run, types and domains must agree, linearity must be established, and logical or global constraints need a suitable execution path. These checks differ from successful host-language compilation.

[Symbolic Expressions](./symbolic-expressions) carry mathematical structure; units, permissions, and provenance need additional bindings.

## 3. Running example: a production-shortfall penalty

### Overview, concepts, and variables

A production context chooses integer quantity $x\in\{0,\ldots,8\}$ in items against demand $d=10$. Unit production cost is $c=2$ and shortage penalty $p=5$ currency units/item, both known and positive. There is one product and no filtering predicate.

### Intermediate value and original objective

Define shortage:

$$
s(x)=\max(0,d-x).
$$

Total cost combines production and shortage:

$$
F(x)=cx+p\,s(x),\qquad \min F(x).
$$

The production domain already imposes capacity. Here $s$ is a function definition, not a freely chosen second business decision.

### Transformed constraints

Introduce a continuous auxiliary $u\ge0$ with:

$$
\text{s.t.}\quad u\ge d-x,\qquad u\ge0,
$$

and minimize $cx+pu$. Because $p>0$ and $u$ has no other coupling in this model, its optimal value for fixed $x$ is exactly $\max(0,d-x)$. The optimization problems therefore share optimal values and optimal production quantities.

This is not an unconditional formulation of the function graph: nonoptimal transformed points can have $u>s(x)$. If other constraints use $u$, or the objective no longer drives it down, provide another argument or an exact graph formulation.

### Result

For $0\le x\le8$, the original objective is $50-3x$. Thus $x=8,s=2,F=26$ is optimal. Map the result through the original definition to explain a shortage of two items, rather than displaying an anonymous auxiliary column.

## 4. Preserve provenance through transformation

A function symbol can generate several rows and auxiliary columns. Record their relationships to original symbols, variables, and constraints. Public reports use business identities rather than mutable row and column positions.

Solution mapping differs from diagnostic mapping. The former retrieves original variables and evaluates intermediates; the latter explains why internal constraints exist and which business rule they implement. Renaming a low-level IIS does not establish original-constraint-level minimality.

## 5. Equivalence, relaxation, and approximation

An exact transformation should state whether it preserves projected feasibility, objective values, or optimal solutions. Auxiliary variables change the dimension, so equality of feasible sets in different spaces is not the right claim.

A relaxation typically enlarges feasibility and can provide bounds; its solutions may violate the original model. Piecewise approximation changes a function and needs a stated interval and error. Expression simplification rewrites representation during construction; it is not optimization solving.

If a backend lacks a semantic feature, select an explicit transformation or reject the model. Do not silently drop terms, relax integrality, or approximate while claiming to solve the unchanged model.

## 6. Backend independence and capability boundaries

Backend independence means business rules do not directly depend on vendor objects, not that all backends have identical features. Model classes, domains, logic, time controls, and diagnostics need capability matching.

Remote execution can transport versioned model representations, but process-local addresses are not cross-machine identities. Manage models, parameters, transformation policies, and solver settings separately; see [Remote Solving](./remote-solver).

## 7. Connections to other chapters

The [Domain Language](./operations-research-language) explains composition, [DDD](./use-ddd-architecture) business ownership, and this chapter transformation and execution. [Formal Design and Verification](./formal-design-and-formal-verification) develops correctness arguments; [Critical Constraint Analysis](./critical-constraint-analysis) uses reverse evidence mapping to explain objective limits.
