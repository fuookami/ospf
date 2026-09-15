# Example 1: Capital investment selection

## 1. Overview

This bounded context selects companies for an investment portfolio, maximising total profit while meeting a minimum total-capital requirement and an upper bound on total liability.

### 1. Dependent Contexts

1. Core linear optimisation context (model variables, linear expressions, constraints, and solver adapter).

The page describes the mathematical model. The Kotlin and Rust snippets are model-building fragments; the example data are supplied by the linked `Demo1` implementations.

---

## 2. Concepts / Entities

### 1. Company

A company is a candidate investment. Its three numeric attributes are read-only input data.

**$Capital_c$** : capital contributed by company $c$.

**$Liability_c$** : liability carried by company $c$.

**$Profit_c$** : profit contributed by company $c$.

The current data are:

| Company | Capital | Liability | Profit |
| :---: | ---: | ---: | ---: |
| A | 3.48 | 1.28 | 5400 |
| B | 5.62 | 2.53 | 2300 |
| C | 7.33 | 1.02 | 4600 |
| D | 6.27 | 3.55 | 3300 |
| E | 2.14 | 0.53 | 980 |

The capital and liability values use the same model unit; profit uses the profit unit shown in the data table.

---

## 3. Variables

### 1. Decision Variables

**$x_c$** : company-selection variable, dimensionless binary, domain $\{0,1\}$, equals $1$ exactly when company $c$ is selected, $\forall c\in C$.

### 2. Auxiliary Variables

None. Capital, liability, and profit are registered linear expressions, not solver decision variables.

---

## 4. Predicates

### 1. Company Status

> Predicates classify entity sets; each predicate defines a subset.

**Selected**$(c)$ : company $c$ belongs to the chosen portfolio.

**Unselected**$(c)$ : company $c$ is not chosen.

---

## 5. Sets

### 1. Company Category

**$C$** : the universal set of candidate companies.

**$C^{Selected}$** : subset satisfying `Selected`, $C^{Selected}=\{c\in C\mid x_c=1\}$, the selected investment portfolio.

**$C^{Unselected}$** : subset satisfying `Unselected`, $C^{Unselected}=\{c\in C\mid x_c=0\}$, the companies left out of the portfolio.

### 2. Entity Pairs / Relations

No pair relation is required; each decision concerns one company only.

---

## 6. Intermediate Values

### 1. Total Capital

**Description**: Total capital is the sum of the capital of every selected company. It is the quantity compared with the minimum-capital threshold.

$$
Capital(x)=\sum_{c\in C}Capital_cx_c.
$$

### 2. Total Liability

**Description**: Total liability is the sum of the liability of every selected company. It is the quantity compared with the maximum-liability threshold.

$$
Liability(x)=\sum_{c\in C}Liability_cx_c.
$$

### 3. Total Profit

**Description**: Total profit is the sum of the profit of every selected company and is the optimisation objective.

$$
Profit(x)=\sum_{c\in C}Profit_cx_c.
$$

---

## 7. Assertions

### 1. Binary Selection

**Description**: Every company is either selected or unselected; fractional selection is not part of this model.

$$
\forall c\in C\; (x_c=0\vee x_c=1).
$$

### 2. Portfolio Partition

**Description**: The selected and unselected subsets partition the candidate-company set.

$$
C^{Selected}\cap C^{Unselected}=\emptyset
\quad\wedge\quad
C^{Selected}\cup C^{Unselected}=C.
$$

---

## 8. Constraints

> Both constraints are hard, inclusive bounds.

### 1. Minimum Capital Requirement

**[最低资本要求]**: the portfolio must reach at least the required total capital; equality is allowed.

$$
s.t.\quad Capital(x)\ge Capital^{Min},\qquad Capital^{Min}=10.
$$

### 2. Maximum Liability Limit

**[最大负债上限]**: the portfolio must not exceed the permitted total liability; equality is allowed.

$$
s.t.\quad Liability(x)\le Liability^{Max},\qquad Liability^{Max}=5.
$$

There is no cardinality, diversification, or balance constraint in the current model.

---

## 9. Objective Function (if applicable)

**Description**: maximise the profit contributed by the selected companies.

$$
\max\; Profit(x)=\sum_{c\in C}Profit_cx_c.
$$

For the displayed data, one optimal portfolio is $\{A,B,C\}$, with capital $16.43$, liability $4.83$, and profit $12300$.

---

## 10. Algorithm References

No standalone algorithm document is referenced. The model is a direct linear 0–1 formulation solved through the core linear-model API.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Not applicable | — | — | No domain-specific algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Company | $c\in C$ | Candidate investment company. |
| Capital | $Capital_c$ | Capital contributed by one company; $Capital(x)$ is the selected total. |
| Liability | $Liability_c$ | Liability contributed by one company; $Liability(x)$ is the selected total. |
| Profit | $Profit_c$ | Profit contributed by one company; $Profit(x)$ is the selected total. |
| Selection | $x_c$ | Binary decision indicating whether company $c$ is selected. |
| Minimum capital | $Capital^{Min}$ | Required lower bound on total capital. |
| Maximum liability | $Liability^{Max}$ | Permitted upper bound on total liability. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use one binary variable per company | Continuous share or integer quantity | The source models an all-or-nothing company choice with `BinVariable1`. | Current implementation |
| Keep both bounds inclusive | Strict inequalities | Kotlin uses `geq` and `leq`; the Rust model uses `GreaterEqual` and `LessEqual`. | Current implementation |
| Keep the Kotlin and Rust APIs separate | Treat one snippet as portable code | Both implementations express the same mathematics but have independent model-registration APIs. | Current implementation |

### Minimal current implementation fragments

The following are intentionally non-standalone model-building fragments. `companies`, `minCapital`, `maxLiability`, `flt64Converter`, and the solver setup come from the linked `Demo1` source; no additional API is implied.

::: code-group

```kotlin [Kotlin]
// Fragment from Demo1.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo1's private companies list and thresholds.
val x = BinVariable1("x", Shape1(companies.size))
val capital = LinearExpressionSymbol(
    sum(companies) { it.capital * x[it] },
    name = "capital"
)
val liability = LinearExpressionSymbol(
    sum(companies) { it.liability * x[it] },
    name = "liability"
)
val profit = LinearExpressionSymbol(
    sum(companies) { it.profit * x[it] },
    name = "profit"
)
metaModel.add(x)
metaModel.add(capital)
metaModel.add(liability)
metaModel.add(profit)
metaModel.maximize(profit)
metaModel.addConstraint(capital geq minCapital)
metaModel.addConstraint(liability leq maxLiability)
```

```rust [Rust]
// Fragment from demo1.rs::PortfolioModel::register/add_constraints.
// Data source: demo1.rs::get_companies; Metric and helper imports are from that module.
let select = VariableCombination1D::new(
    Shape::new([companies.len()]),
    "select",
);
let select_idx = model.register_combination(&select)?;
let metrics_list = vec![Metric::Capital, Metric::Liability, Metric::Profit];
let metrics = flat_map1(
    "portfolio_metric",
    &metrics_list,
    |metric| {
        let monomials: Vec<_> = companies
            .iter()
            .enumerate()
            .map(|(i, company)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    metric.value(company),
                    select_idx[i],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, metric| metric.name().to_string(),
);
model.add_symbol_combination(&metrics)?;
let cap_coeffs = extract_coeffs(&metrics[0]);
model.add_linear_constraint(
    &cap_coeffs,
    ConstraintRelation::GreaterEqual,
    min_capital,
    "capital_constraint",
)?;
let lia_coeffs = extract_coeffs(&metrics[1]);
model.add_linear_constraint(
    &lia_coeffs,
    ConstraintRelation::LessEqual,
    max_liability,
    "liability_constraint",
)?;
let obj_coeffs = extract_coeffs(&metrics[2]);
model.add_linear_objective(&obj_coeffs, "total_profit");
model.set_objective_category(ObjectiveCategory::Maximum);
```

:::

#### Source and verification

- [Rust counterpart: `src/core/demo1.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo1.rs)

The Rust counterpart uses the same mathematical model and data, but its Rust `MetaModel`, variable-combination, and symbol-combination APIs are independent of the Kotlin API.

- [Current implementation: `Demo1.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo1.kt)
- [Core build-structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The displayed optimum selects A, B, and C. The snippets above mirror the current `BinVariable1`, `LinearExpressionSymbol`, `LinearMetaModel<Flt64>`, `ScipLinearSolver`-compatible expression, and Rust `MetaModel` APIs; they are excerpts rather than complete runnable programs.

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganised the example into the domain-model template and added quantified constraints, assertions, and Kotlin/Rust fragments. | Make the mathematical model and current implementation boundary explicit. |
