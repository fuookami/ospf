# Material context model

## 1. Overview

The material context supplies product demand, material dimensions, and cutting-plan data. [Produce](../domain-produce/domain-model) consumes these data and owns master plan-usage variables. This page does not declare a separate product-production variable family.

### 1. Dependent Contexts

Input adapters and configuration supply products, materials, and cutting rules. Cutting-plan generation constructs feasible plans from them.

## 2. Concepts / Entities

### 1. Products

**$w_p$**: Cut width of product $p$, in a common length unit.

**$d_p$**: Product demand. Demo3's count-based demand uses roll-count semantics. Length or unit-weight configurations require the corresponding demand-contribution conversion.

### 2. Materials

**$L_m$**: Usable cutting width of material $m$. The full framework can include dimension ranges and processing properties; generation selects applicable values.

### 3. Cutting Plans

**$m(j)$**: Material used by plan $j$.

**$n_{pj}$**: Nonnegative integer number of pieces of product $p$ in plan $j$.

**$a_{pj}$**: Plan $j$'s contribution to product $p$'s demand. For this example's count-based products without length or unit weight, $a_{pj}=n_{pj}$.

## 3. Variables

### 1. Decision Variables

The material data contract documented here declares no master decision variables. Plan usage $x_j$ belongs to produce; product yield $q_p$ is derived from $x_j$.

### 2. Auxiliary Variables

Piece counts and remaining width are known data when a plan becomes a master column, not independent auxiliary master variables. Pricing may search these values while generating the plan.

## 4. Predicates

**$\operatorname{uses}(j,m)$**: Plan $j$ uses material $m$.

**$\operatorname{contains}(j,p)$**: Plan $j$ contains product $p$.

**$\operatorname{feasible}(j)$**: Plan $j$ satisfies material dimensions and configured processing rules.

**$\operatorname{dynamic}(p)$**: Product $p$ has a length rule requiring separate treatment.

## 5. Sets

**$P$**: Products; **$M$**: materials.

**$\mathcal J$**: Candidate plans satisfying material and processing rules; **$J_t\subseteq\mathcal J$**: plans inserted before master solve $t$.

**$P_j=\{p\in P:n_{pj}>0\}$**: Products included in plan $j$.

**$P^{dynamic}$**: Products with optional length rules.

## 6. Intermediate Values

### 1. Used Width per Plan

**Description**: Used width depends on piece counts within one plan, not total product output from the whole master.

$$
W_j=\sum_{p\in P}w_pn_{pj},\qquad j\in\mathcal J.
$$

### 2. Remaining Width per Plan

**Description**: For the count-based example, remaining width is usable material width minus the plan's used width.

$$
r_j=L_{m(j)}-W_j,\qquad j\in\mathcal J.
$$

If processing introduces additional losses or a different width convention, use coefficients computed for that configuration rather than blindly applying this no-extra-loss formula.

### 3. Demand Contribution

**Description**: This example satisfies demand by piece count. Other demand units, such as length or weight, require conversion.

$$
a_{pj}=n_{pj},\qquad p\in P,\ j\in\mathcal J
\quad\text{(this example's count configuration)}.
$$

## 7. Assertions

Valid inputs have positive widths and nonnegative demand. Plan piece counts are nonnegative integers:

$$
\forall p\in P:\quad w_p>0\wedge d_p\ge0,
\qquad
\forall p\in P,\ j\in\mathcal J:\quad n_{pj}\in\mathbb Z_{\ge0}.
$$

Under the no-extra-loss convention in Section 6, feasible plans satisfy:

$$
\forall j:\quad\operatorname{feasible}(j)\Rightarrow r_j\ge0.
$$

## 8. Constraints

### 1. Material Width Feasibility [材料宽度可行性]

**Description**: A candidate plan must fit its material. Configured generation rules additionally check knife counts, lengths, and other processing restrictions.

$$
\sum_{p\in P}w_pn_{pj}\le L_{m(j)},\qquad j\in\mathcal J.
$$

This is a single-plan data and generation-feasibility condition, not an additional master row owned here. Produce registers $q_p\ge d_p$ separately for each product. This page neither aggregates unrelated product demands nor invents stock limits from the material context's name.

## 9. Objective Function (if applicable)

The material data contract has no independent master objective. It supplies demand-contribution, material-usage, and remaining-width coefficients for produce and optional loss-objective pipelines.

## 10. Algorithm References

| Algorithm | Referenced In | Description |
|---|---|---|
| Cutting-plan generation | Sections 4, 5, 8 | Searches plans satisfying material and processing rules |
| Demand-contribution conversion | Sections 2, 6 | Converts count, length, or weight into the demand unit |
| Plan-property calculation | Section 6 | Computes used and remaining width |

[Kotlin material-context source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-csp1d/csp1d-domain-material-context); [Kotlin/Rust example entry points](/examples/framework-example3).

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|---|---|---|
| Product width | $w_p$ | Transverse cutting dimension |
| Demand | $d_p$ | Product requirement in the selected demand unit |
| Plan piece count | $n_{pj}$ | Pieces of a product in one use of a plan |
| Demand contribution | $a_{pj}$ | Contribution of one plan use to product demand |
| Remaining width | $r_j$ | Unused width in one plan |

## 12. Design Decisions

| Decision | Alternative | Rationale |
|---|---|---|
| Separate plan coefficients from usage variables | Put global production into plan properties | Keeps coefficients independent of the master solution |
| State demand units explicitly | Always treat contribution as piece count | Length and weight configurations require conversion |
| Assign demand rows to produce | Build another production model in material | Gives variables and constraints a single owner |

## 13. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual material contracts and plan coefficients | Removed incorrect cross-product demand sums and mixed variables |
