# Produce context model

## 1. Overview

Produce owns cutting-plan usage variables and product-yield expressions. It compiles generated plans into a restricted master and updates the model as columns are inserted.

### 1. Dependent Contexts

[Material](../domain-material/domain-model) supplies products, demand, and plan coefficients. Cutting-plan generation supplies initial and priced columns. Optional length-assignment and loss-objective pipelines depend on the produce aggregate. The application controls solving.

## 2. Concepts / Entities

### 1. Products and Plans

**$d_p$**: Demand of product $p$.

**$a_{pj}$**: Contribution of one use of plan $j$ to product $p$'s demand. In Demo3's count-based configuration, it is the corresponding piece count.

### 2. Material Usage and Remaining Width

**$m(j)$**: Material used by plan $j$; one use consumes one batch of that material.

**$r_j$**: Remaining width of one plan, computed from plan data. It does not depend on total product output in the master.

## 3. Variables

### 1. Decision Variables

**$x_j$**: Dimensionless usage count of plan $j$, for every $j\in J_t$. Column-generation LPs use $x_j\in\mathbb R_{\ge0}$; integer solving uses $x_j\in\mathbb Z_{\ge0}$.

Product yield $q_p$ is an intermediate value, not another unlinked production variable.

### 2. Auxiliary Variables

When integer-solve configuration includes yield-deviation terms for a demand, **$u_p,o_p\in\mathbb R_{\ge0}$** represent underproduction and overproduction in demand units, for every $p\in P$. The LP path does not register yield slack. Demo3 configures no such terms and uses hard demand coverage. This condition is not a separate API switch named `slack`.

## 4. Predicates

**$\operatorname{active}(j,t)$**: Plan $j$ was inserted before the current solve.

**$\operatorname{feasible}(j)$**: Plan $j$ satisfies material and processing rules.

**$\operatorname{contributes}(j,p)$**: Plan $j$ contributes nonzero demand for product $p$.

**$\operatorname{slackEnabled}(p)$**: Integer-solve configuration includes yield-deviation terms for product $p$'s demand; this is notation for a configuration condition.

## 5. Sets

**$P$**: Products; **$M$**: materials.

**$J_t$**: Available plan columns at restricted-master solve $t$; **$J_t^{m}$**: those using material $m$.

$$
J_t=\{j:\operatorname{active}(j,t)\wedge\operatorname{feasible}(j)\}.
$$

This is an iteration-dependent finite set, not all cutting plans enumerated in advance.

## 6. Intermediate Values

### 1. Product Yield

**Description**: Product output is each plan's demand contribution multiplied by its usage, summed per product.

$$
q_p=\sum_{j\in J_t}a_{pj}x_j,\qquad \forall p\in P.
$$

### 2. Material Usage

**Description**: Each use of a plan belonging to material $m$ consumes one batch. Usage is not reconstructed as a separate model from product output.

$$
U_m=\sum_{j\in J_t^{m}}x_j,\qquad \forall m\in M.
$$

### 3. Total Remaining Width

**Description**: Each plan's remaining width is a fixed column coefficient. Its total is weighted by plan usage.

$$
R=\sum_{j\in J_t}r_jx_j.
$$

$q_p,U_m,R$ are derived quantities. Configuration determines whether a particular loss expression is registered or included in an objective.

## 7. Assertions

Every master column must be feasible, with demand contributions in units compatible with $d_p$:

$$
\forall j\in J_t:\quad\operatorname{feasible}(j),
\qquad
\forall p\in P,\ j\in J_t:\quad a_{pj}\ge0.
$$

Fractional LP plan usage must not be interpreted as a final production plan. Integer results still require analysis against original demand and plan data.

## 8. Constraints

### 1. Product Demand Coverage [产品需求覆盖]

**Description**: The ordinary demand configuration requires output to meet demand separately for each product.

$$
s.t.\quad
\sum_{j\in J_t}a_{pj}x_j\ge d_p,
\qquad \forall p\in P.
$$

Summing all product outputs and comparing that single sum with each product's demand is not equivalent.

### 2. Yield Slack Form [产出松弛形式]

**Description**: When under/overproduction is enabled, the corresponding balance form is:

$$
q_p-o_p+u_p=d_p,\qquad
u_p,o_p\ge0,\qquad \forall p\in P.
$$

The selected pipelines determine whether shortage is allowed, bounded, or penalized. This equation alone does not enforce the original hard demand.

### 3. Optional Resource and Length Rules

For finite available batches $B_m$, the material pipeline registers:

$$
s.t.\quad U_m\le B_m,\qquad
\forall m\in M\text{ with finite available batches }B_m.
$$

Demo3 sets no finite material-batch limit and has an empty machine list. It therefore registers material-usage expressions but produces no material-batch upper bounds or machine constraints. Dynamic-length and other resource rules also require corresponding data and configuration, not an undefined machine-hour capacity substitute.

## 9. Objective Function (if applicable)

**Description**: The base plan-usage objective minimizes usage count:

$$
\min\quad Z=\sum_{j\in J_t}x_j.
$$

Demo3 supplies no extra solve configuration and uses this default objective, without additional yield-deviation, waste, or length objectives. Optional loss pipelines may configure remaining-width, material-cost, or overproduction objectives. Do not add all terms unconditionally or mix width and monetary cost without a defined unit conversion.

## 10. Algorithm References

| Algorithm | Referenced In | Description |
|---|---|---|
| Restricted-master LP | Sections 3, 8, 9 | Computes the current column-pool solution and dual prices |
| Plan pricing | Material and generation services | Searches feasible plans with negative reduced cost |
| Column insertion | Sections 5, 6 | Adds usage variables and updates yield, resource, and objective coefficients |
| Integer solving and analysis | Sections 3, 7 | Converts column usage into executable cutting plans |

[Kotlin produce-context source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-csp1d/csp1d-domain-produce-context); [Kotlin/Rust example entry points](/examples/framework-example3).

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|---|---|---|
| Plan usage | $x_j$ | Number of uses of a plan |
| Product yield | $q_p$ | Total demand contribution from plans |
| Material usage | $U_m$ | Plan-weighted material consumption |
| Total remaining width | $R$ | Plan remaining widths weighted by usage |
| Under/overproduction | $u_p,o_p$ | Deviations in the optional demand-balance form |
| Current column pool | $J_t$ | Cutting plans available to this master solve |

## 12. Design Decisions

| Decision | Alternative | Rationale |
|---|---|---|
| Declare usage by plan | Declare unlinked product and plan quantities | Column coefficients preserve yield consistency |
| Separate LP and integer phases | Solve an integer master before every pricing call | LP duals drive column generation |
| Identify optional slack and objectives | Enable every penalty and resource rule by default | Keeps the model consistent with configuration |

## 13. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual yield equations, domains, and remaining-width definitions | Removed mixed product and plan variables |
