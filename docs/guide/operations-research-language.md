# The Language of Operations Research

OSPF modeling interfaces can be understood as an internal domain-specific language (DSL): valid Kotlin or Rust programs express mathematical models while making business concepts nameable, composable, and reusable. This chapter explains that language without requiring knowledge of solver algorithms.

## 1. Primitives, composition, and abstraction

A modeling language needs three layers. Primitives introduce constants, parameters, and decision variables. Composition forms arithmetic expressions, logical relations, constraints, and objectives. Abstraction names recurring expressions so they can be used as units.

| Element | Mathematical meaning | Not the same as |
|---|---|---|
| Parameter | Data known before this model is built | A solver-selected variable |
| Variable | An unknown value in a specified domain | Its value in an incumbent |
| Expression | A relation built from parameters, variables, and operations | A completed numerical calculation |
| Intermediate value | A named expression with business meaning | A freely chosen independent variable |
| Constraint | A condition restricting feasible plans | An assertion checking input data |
| Objective | A preference among feasible plans | A mandatory feasibility condition |

Linear and polynomial expressions are only part of the language. Logical relations, finite domains, and CP global constraints have their own semantics; they are not all inequalities between polynomials.

## 2. Running model: compartment loading

### Overview, concepts, and sets

A loading context decides where cargo goes and limits compartment weight, area loading, and loading per unit length. This is a simplified static uniform-loading model, not a structural-strength analysis.

Let $I$ be the cargo set and $J$ the compartment set. Cargo $i$ has mass $w_i>0$ in kg. Compartment $j$ has effective area $A_j>0$ in square meters, length $L_j>0$ in meters, capacity $C_j$ in kg, area-loading limit $P_j$ in kg per square meter, and line-loading limit $D_j$ in kg per meter. Area loading is sometimes called pressure in business terminology, but is not physical pressure measured in Pa here.

### Variables and predicates

The dimensionless variable $x_{ij}\in\{0,1\}$ means cargo $i$ is assigned to compartment $j$, for all $i\in I,j\in J$. Compatibility relation $R\subseteq I\times J$ lists permitted assignments; fix $x_{ij}=0$ outside it. No independent auxiliary variables are needed.

### Intermediate values

Compartment weight sums the mass assigned to it:

$$
W_j=\sum_{i\in I}w_i x_{ij}.
$$

Area and line loading both derive from that weight:

$$
p_j=\frac{W_j}{A_j},\qquad d_j=\frac{W_j}{L_j},\qquad j\in J.
$$

These are not three independent decisions. Once $W_j$ is known, $p_j$ and $d_j$ are determined.

### Data assertions and constraints

Positive areas and lengths are input assertions checked before constructing expressions, not choices left to the solver. Load each item at most once and respect all three compartment limits:

$$
\begin{aligned}
\text{s.t.}\quad &\sum_{j\in J}x_{ij}\le1 &&\forall i\in I,\\
&W_j\le C_j &&\forall j\in J,\\
&p_j\le P_j,\quad d_j\le D_j &&\forall j\in J,\\
&x_{ij}=0 &&\forall(i,j)\notin R.
\end{aligned}
$$

### Objective and result

Maximize loaded mass:

$$
\max\sum_{j\in J}W_j.
$$

Take one compartment with $A=2$, $L=2$, $C=120$, $P=50$, $D=60$, and compatible cargo of 60 and 40 kg. Loading both gives $W=100$, $p=50$, $d=50$. All limits hold, and all available mass is loaded, so the plan is optimal. The area-loading limit is active, but that alone does not imply benefit from relaxing it.

## 3. Why intermediate values matter

If every constraint repeats $\sum_iw_ix_{ij}$, readers must repeatedly recognize it as compartment weight. Naming it $W_j$ makes it business vocabulary whose definition is shared by capacity, area-loading, and line-loading rules.

An intermediate value can appear like a variable in composition while remaining mathematically bound to its definition. An arithmetic intermediate may be expanded; a function symbol may require auxiliary variables and constraints. A name need not create a solver column, nor be mere textual substitution. See [Compiler-Like Architecture and Model Transformation](./compiler-architecture).

Sharing belongs to an explicit model or business context, not to process-global state. Reusing a symbol from an old model in a new one can break identity and lifetime assumptions.

## 4. Indices, bulk expressions, and the host language

$x_{ij}$ is a variable family whose indices express business identity. A solver column number is only a position after compilation. Compatibility filtering, compartment aggregation, and allocation-vector views should preserve those identities. When a set is empty, check whether the empty sum of zero matches the intended business rule.

An internal DSL uses host-language functions, types, collections, and operators to construct expressions. Ordinary host-language branches run during model construction. Conditions depending on unknown decisions must become symbolic logic, not ordinary Boolean branches that prematurely choose an outcome.

## 5. From language to business components

A loading context can expose $W_j$ for area-loading and line-loading rules without duplicating its construction. This connects language abstraction to context interfaces. Business ownership, expression dependencies, and Kotlin/Rust memory management are different concerns; one ownership diagram cannot replace all three explanations.

Continue with [The Modeling and Solving Workflow](./modeling-workflow) for assembly, and [Using Domain-Driven Design](./use-ddd-architecture) for collaboration across business contexts.
