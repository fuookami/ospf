# Symbolic Expressions and Symbolic Operations

Numerical evaluation asks for a result given values. Symbolic operations preserve unknowns and relationships, describing which inputs determine a result and how. This layer supports optimization modeling, indicators, and rule explanations independently of LLMs.

## 1. Expressions are not formula strings

Strings lose type, reference, and scope information. An abstract syntax tree (AST) represents constants, references, arithmetic, comparisons, and logic with nodes. These are conceptual nodes, not Kotlin/Rust API names:

```text
LessEqual
├─ Add
│  ├─ Multiply(Constant(2), Reference(production[A]))
│  └─ Multiply(Constant(3), Reference(production[B]))
└─ Reference(labor.capacity)
```

This represents $2x_A+3x_B\le H$. An AST describes the relationship but does not automatically know units, business dates, or field permissions. Business bindings carry those concerns.

## 2. Three distinct representations

| Representation | Responsibility | Example |
|---|---|---|
| Business rule | Objects, units, time, and provenance | This shift uses at most 8 labor hours |
| Symbolic expression | Mathematical structure and references | $2x_A+3x_B\le H$ |
| Solver model | Domains, constraints, objective, executable representation | An integer linear production model |

Representability in a mathematical AST does not imply support by the chosen optimization backend. Constructing a variable-by-variable product is not permission to send it down a linear-only path.

## 3. Evaluation, binding, and partial evaluation

With $x_A=1,x_B=2,H=8$, evaluation gives $8\le8$, true. Binding only $H=8$ leaves $2x_A+3x_B\le8$ symbolic. Parameter binding or partial evaluation does not solve for production.

An environment must distinguish unbound, invalid, and zero values. Missing production is not zero, and division by zero must not disappear through simplification. Unknown Boolean values are not automatically false unless the language explicitly defines and records that semantics.

Reusing a rule for another shift means binding a new $H$ and new production references, not embedding old variable instances in a persistent template.

## 4. Normalization and conditions for equivalence

Combining $2x+x$ into $3x$ supports normalization. Replacing $x/x$ with 1 requires $x\ne0$. Floating-point reassociation can change rounding, and conditional expressions may have branch-evaluation rules. Transformations need numerical and domain assumptions, not just pattern matching.

Named intermediates can form a directed acyclic dependency graph without duplicating entire trees. Shared nodes do not imply a shared mutable evaluation cache; caching must respect runs, inputs, and concurrency. Cyclic definitions such as $a=b+1,b=a-1$ cannot be ordinary expression expansion. Model an equation system explicitly if that is intended.

## 5. Running example: two uses of a labor indicator

### Concepts, variables, and intermediates

Let $I=\{A,B\}$, with production $x_i\in\mathbb Z_{\ge0}$ in items, coefficients $a_A=2,a_B=3$ hours/item, and budget $H=8$ hours. No filtering predicates or auxiliary variables are needed. Labor usage is:

$$
U=\sum_{i\in I}a_i x_i.
$$

Data assertions require positive coefficients and a nonnegative budget. An optimization model can add:

$$
\text{s.t.}\quad U\le H,\qquad \max(3x_A+4x_B).
$$

### Numerical and modeling paths

The numerical path binds existing plan $(1,2)$ and returns $U=8$ hours. The modeling path retains unknown production and converts the relation into constraint coefficients. They share a mathematical definition but produce different outputs: a value versus model structure.

This is why reusable expression components matter: one business indicator can support display, constraint construction, and result checking without maintaining three independent formulas.

## 6. Dependencies, caching, and business identity

Dependency collection identifies referenced fields and symbols, helping determine rebinding scope. Cache keys include structure, parameter values or slot conventions, numerical semantics, data versions, and transformation policy.

Matching structure does not imply matching inputs or permissions. Caching an evaluation plan is not caching an optimal solution. Original rule identity should remain distinct from a normalized hash so errors and results can be traced back to business rules.

See [Compiler-Like Architecture and Model Transformation](./compiler-architecture) for conversion to solver models and the [LLM Technical Path](./integrating-llms-technical) for controlled business inputs using this layer.
