# Example 5: 0–1 knapsack

## 1. Overview

This bounded context selects whole cargo items under a total-weight limit, maximising total value without requiring the capacity to be filled exactly.

### 1. Dependent Contexts

1. Core binary linear optimisation context (binary variables, linear expressions, constraints, and solver adapter).

The Kotlin and Rust snippets are model-building fragments. Cargo data and solver setup come from the linked Demo5 implementations.

---

## 2. Concepts / Entities

### 1. Cargo

A cargo item is an indivisible item that can be selected at most once.

**$Weight_c$** : weight of cargo item $c$.

**$Value_c$** : value of cargo item $c$.

The current items are $(Weight,Value)=(2,6),(2,3),(6,5),(5,4),(4,6)$, and $Weight^{Max}=10$.

---

## 3. Variables

### 1. Decision Variables

**$x_c$** : cargo-selection variable, dimensionless binary, domain $\{0,1\}$, equals $1$ when cargo $c$ is selected, $\forall c\in C$.

### 2. Auxiliary Variables

None. Total value and total weight are registered linear expressions/intermediate values.

---

## 4. Predicates

### 1. Cargo Status

> Predicates classify cargo items by selection and capacity status.

**Selected**$(c)$ : cargo item $c$ is selected, equivalently $x_c=1$.

**NotSelected**$(c)$ : cargo item $c$ is not selected, equivalently $x_c=0$.

**FitsCapacity**$(x)$ : the selected cargo plan satisfies the total-weight limit.

---

## 5. Sets

### 1. Cargo Category

**$C$** : universal set of cargo items.

**$C^{Selected}$** : subset satisfying Selected, $C^{Selected}=\{c\in C\mid x_c=1\}$, the chosen cargo items.

**$C^{NotSelected}$** : subset satisfying NotSelected, $C^{NotSelected}=\{c\in C\mid x_c=0\}$, the unchosen items.

### 2. Entity Pairs / Relations

No pair relation is required; every decision concerns one cargo item.

---

## 6. Intermediate Values

### 1. Total Value

**Description**: Total value is the sum of the values of all selected cargo items and is the objective expression.

$$
Value(x)=\sum_{c\in C}Value_cx_c.
$$

### 2. Total Weight

**Description**: Total weight is the sum of the weights of all selected cargo items and is compared with the capacity limit.

$$
Weight(x)=\sum_{c\in C}Weight_cx_c.
$$

---

## 7. Assertions

### 1. Whole-Item Selection

**Description**: Every cargo item is either selected once or not selected; fractional selection and repeated copies are not represented.

$$
\forall c\in C\; (x_c=0\vee x_c=1).
$$

### 2. Non-negative Cargo Data

**Description**: Current item weights, values, and capacity are non-negative.

$$
\forall c\in C\;(Weight_c\ge0\wedge Value_c\ge0)
\;\wedge\;
Weight^{Max}\ge0.
$$

---

## 8. Constraints

### 1. Weight Capacity

**[重量容量上限]**: the total weight of selected cargo must not exceed the available capacity; unused capacity is allowed.

$$
s.t.\quad Weight(x)=\sum_{c\in C}Weight_cx_c\le Weight^{Max}=10.
$$

There is no equality requirement and no minimum-fill constraint.

---

## 9. Objective Function (if applicable)

**Description**: maximise the total value of selected cargo items.

$$
\max\; Value(x)=\sum_{c\in C}Value_cx_c.
$$

---

## 10. Algorithm References

No standalone algorithm document is referenced. This is a direct 0–1 knapsack formulation.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Not applicable | — | — | No domain-specific algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Cargo item | $c\in C$ | Indivisible item that can be selected once. |
| Weight | $Weight_c$ | Weight of one cargo item; $Weight(x)$ is the selected total. |
| Value | $Value_c$ | Value of one cargo item; $Value(x)$ is the selected total. |
| Selection | $x_c$ | Binary decision indicating whether cargo $c$ is selected. |
| Capacity | $Weight^{Max}$ | Maximum permitted total weight. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use binary cargo variables | Integer multiplicity variables | The source uses BinVariable1 and treats each listed cargo item as one indivisible item. | Current implementation |
| Allow unused capacity | Require Weight(x)=WeightMax | The source adds only a less-than-or-equal weight constraint. | Current implementation |
| Keep Kotlin and Rust model APIs separate | Present one as portable code | Both implementations share the mathematics but use independent registration APIs. | Current implementation |

### Minimal current implementation fragments

The following are non-standalone fragments from Demo5. cargos, maxWeight, model setup, converter, and solver setup are supplied by the linked source.

::: code-group

```kotlin [Kotlin]
// Fragment from Demo5.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo5's private cargos list and maxWeight.
val x = BinVariable1("x", Shape1(cargos.size))
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
metaModel.maximize(cargoValue, "value")
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    name = "weight"
)
```

```rust [Rust]
// Fragment from demo5.rs::KnapsackModel::register/add_constraints.
// Data source: build_cargos and max_weight in demo5.rs.
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
```

:::

#### Source and verification

- [Rust counterpart: src/core/demo5.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo5.rs)

The Rust counterpart uses the same 0–1 mathematics and data, but its Rust MetaModel, variable-combination, and symbol-combination APIs are independent of the Kotlin API.

- [Current implementation: Demo5.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo5.kt)
- [Core build-structure test: CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The model is 0–1; it is not the bounded-multiplicity model in Example 6. The fragments above contain the current variable, intermediate expression, objective, and constraint APIs, but remain excerpts rather than complete runnable programs.

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganised the example into the domain-model template and added quantified assertions, a named capacity constraint, and Kotlin/Rust fragments. | Make whole-item selection and capacity semantics explicit. |
