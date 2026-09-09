# Example 6: Bounded-multiplicity knapsack

## 1. Overview

This bounded context chooses an integer quantity of each cargo type under per-type stock limits and a total-weight limit, maximising total value.

### 1. Dependent Contexts

1. Core integer linear optimisation context (non-negative integer variables, linear expressions, constraints, and solver adapter).

The Kotlin and Rust snippets are model-building fragments. Cargo data and solver setup come from the linked Demo6 implementations.

---

## 2. Concepts / Entities

### 1. Cargo Type

A cargo type can be selected several times, but its stock is finite.

**$Weight_c$** : weight of one unit of cargo type $c$.

**$Value_c$** : value of one unit of cargo type $c$.

**$Amount^{Max}_c$** : maximum available number of units of cargo type $c$.

The current data are:

| Cargo | Weight | Value | Available amount |
| :---: | ---: | ---: | ---: |
| 1 | 1 | 6 | 10 |
| 2 | 2 | 10 | 5 |
| 3 | 2 | 20 | 2 |

The total weight limit is $Weight^{Max}=8$.

---

## 3. Variables

### 1. Decision Variables

**$x_c$** : cargo-count variable, item-count quantity, domain $\mathbb{Z}_{\ge0}$, number of selected units of cargo type $c$, $\forall c\in C$.

### 2. Auxiliary Variables

None. Total value and total weight are registered linear expressions/intermediate values.

---

## 4. Predicates

### 1. Cargo Quantity Status

> Predicates classify cargo types by whether they contribute to the plan.

**SelectedType**$(c)$ : cargo type $c$ has a positive selected quantity, $x_c>0$.

**UnusedType**$(c)$ : cargo type $c$ has no selected units, $x_c=0$.

**AtStockLimit**$(c)$ : all available units of cargo type $c$ are selected, $x_c=Amount^{Max}_c$.

---

## 5. Sets

### 1. Cargo Type Category

**$C$** : universal set of cargo types.

**$C^{SelectedType}$** : subset satisfying SelectedType, $C^{SelectedType}=\{c\in C\mid x_c>0\}$, cargo types contributing at least one unit.

**$C^{UnusedType}$** : subset satisfying UnusedType, $C^{UnusedType}=\{c\in C\mid x_c=0\}$, cargo types not selected.

### 2. Entity Pairs / Relations

No pair relation is required; each stock rule applies to one cargo type.

---

## 6. Intermediate Values

### 1. Total Value

**Description**: Total value is the sum of unit values multiplied by the selected count of each cargo type.

$$
Value(x)=\sum_{c\in C}Value_cx_c.
$$

### 2. Total Weight

**Description**: Total weight is the sum of unit weights multiplied by the selected count of each cargo type.

$$
Weight(x)=\sum_{c\in C}Weight_cx_c.
$$

---

## 7. Assertions

### 1. Integer Non-negative Counts

**Description**: A cargo type can contribute zero or more whole units only.

$$
\forall c\in C\; (x_c\in\mathbb{Z}_{\ge0}).
$$

### 2. Stock-Bounded Count

**Description**: Every selected count is no greater than the recorded stock amount.

$$
\forall c\in C\; (0\le x_c\le Amount^{Max}_c).
$$

---

## 8. Constraints

### 1. Total Weight Capacity

**[总重量容量上限]**: the total weight of all selected units must not exceed the knapsack capacity.

$$
s.t.\quad Weight(x)=\sum_{c\in C}Weight_cx_c\le Weight^{Max}=8.
$$

### 2. Cargo Stock Limit

**[货物库存上限]**: the selected quantity of every cargo type must not exceed its available stock.

$$
s.t.\quad x_c\le Amount^{Max}_c,\qquad \forall c\in C.
$$

The lower bound $x_c\ge0$ is supplied by the Kotlin UIntVariable1 and Rust UInteger variable domains; it is not a separate lower-bound row in either current source.

---

## 9. Objective Function (if applicable)

**Description**: maximise the value of the selected cargo units.

$$
\max\; Value(x)=\sum_{c\in C}Value_cx_c.
$$

A displayed optimum is $x=(4,0,2)$, with total weight $8$ and total value $64$.

---

## 10. Algorithm References

No standalone algorithm document is referenced. This is a direct bounded-integer knapsack formulation.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Not applicable | — | — | No domain-specific algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Cargo type | $c\in C$ | Item type that may be selected multiple times. |
| Unit weight | $Weight_c$ | Weight of one unit of type $c$. |
| Unit value | $Value_c$ | Value of one unit of type $c$. |
| Selected count | $x_c$ | Whole-unit quantity selected for type $c$. |
| Stock limit | $Amount^{Max}_c$ | Available quantity of type $c$. |
| Weight capacity | $Weight^{Max}$ | Maximum total selected weight. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use non-negative integer count variables | Binary variables | The source uses UIntVariable1/UInteger and permits multiple units of one type. | Current implementation |
| Encode stock as an upper bound | Omit stock or expand each unit into a binary item | Kotlin sets each variable range upper bound; Rust adds one upper-bound constraint per cargo type. | Current implementation |
| Keep this model distinct from Example 5 | Reuse 0–1 subset language | Example 6 permits repeated units, so “subset” alone would be misleading. | Current implementation |

### Minimal current implementation fragments

The following are non-standalone fragments from Demo6. cargos, maxWeight, model setup, converter, and solver setup are supplied by the linked source.

::: code-group

```kotlin [Kotlin]
// Fragment from Demo6.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo6's private cargos list and maxWeight.
val x = UIntVariable1("x", Shape1(cargos.size))
val cargoValue = LinearExpressionSymbol(
    sum(cargos) { c -> c.value * x[c] },
    name = "value"
)
val cargoWeight = LinearExpressionSymbol(
    sum(cargos) { c -> c.weight * x[c] },
    name = "weight"
)
metaModel.add(x)
metaModel.add(cargoValue)
metaModel.add(cargoWeight)
for (c in cargos) {
    x[c].range.ls(c.amount)
}
metaModel.maximize(cargoValue, "value")
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    name = "weight"
)
```

```rust [Rust]
// Fragment from demo6.rs::IntegerKnapsackModel::register/add_constraints.
// Data source: build_cargos and max_weight in demo6.rs.
let x = VariableCombination1D::new(Shape::new([cargos.len()]), "x");
let x_idx = model.register_combination(&x)?;
let cargo_value = flat_map1_indexed(
    "cargo_value",
    cargos,
    |i, cargo| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                cargo.value,
                x_idx[i],
            )],
            0.0,
        )
    },
    |_, cargo| cargo.name.clone(),
);
model.add_symbol_combination(&cargo_value)?;
let cargo_weight = flat_map1_indexed(
    "cargo_weight",
    cargos,
    |i, cargo| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                cargo.weight,
                x_idx[i],
            )],
            0.0,
        )
    },
    |_, cargo| cargo.name.clone(),
);
model.add_symbol_combination(&cargo_weight)?;
let val_coeffs = extract_coeffs(&cargo_value[0]);
model.add_linear_objective(&val_coeffs, "value");
model.set_objective_category(ObjectiveCategory::Maximum);
let wt_coeffs = extract_coeffs(&cargo_weight[0]);
model.add_linear_constraint(
    &wt_coeffs,
    ConstraintRelation::LessEqual,
    max_weight,
    "weight",
)?;
for (i, cargo) in cargos.iter().enumerate() {
    model.add_linear_constraint(
        &[(x_idx[i], 1.0)],
        ConstraintRelation::LessEqual,
        cargo.max_amount,
        &format!("upper_{}", i),
    )?;
}
```

:::

#### Source and verification

- [Rust counterpart: src/core/demo6.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo6.rs)

The Rust counterpart uses the same bounded-integer mathematics and data, but its Rust MetaModel, variable-combination, and symbol-combination APIs are independent of the Kotlin API.

- [Current implementation: Demo6.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo6.kt)
- [Core build-structure test: CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The source uses UIntVariable1/UInteger rather than a binary variable, sets the per-type upper bounds, and applies the total-weight inequality. The fragments above are model-building excerpts rather than complete runnable programs.

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganised the example into the domain-model template and added quantified stock assertions, named constraints, and Kotlin/Rust fragments. | Make bounded multiplicity distinct from the 0–1 model. |
