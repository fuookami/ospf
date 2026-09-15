# Example 7: Transportation from warehouses to stores

## 1. Overview

This bounded context assigns integer shipments from warehouses to stores, minimizing total transportation cost while respecting warehouse capacity and store demand. The page documents the current `Demo7` model; the Kotlin and Rust snippets are model-building fragments, and their input data comes from the linked source files.

The sample demands are $(200,400,600,300)$ and warehouse capacities are $(510,470,520)$:

| Warehouse | S1 | S2 | S3 | S4 |
| :---: | ---: | ---: | ---: | ---: |
| W1 | 12 | 13 | 21 | 7 |
| W2 | 14 | 17 | 8 | 18 |
| W3 | 10 | 11 | 9 | 15 |

### 1. Dependent Contexts

1. None. The example is a self-contained linear optimization model.

---

## 2. Concepts / Entities

### 1. Warehouse

A warehouse supplies goods and has a total stowage capacity and a unit cost for each modeled store.

**$Stowage_w$**: capacity of warehouse $w$ in shipment units.

**$Cost_{ws}$**: unit transportation cost from warehouse $w$ to store $s$, defined when the source cost map contains the pair.

### 2. Store

A store receives goods and specifies a minimum demand.

**$Demand_s$**: required shipment quantity for store $s$.

---

## 3. Variables

### 1. Decision Variables

**$x_{ws}$**: shipment quantity from warehouse $w$ to store $s$, measured in shipment units, integer and non-negative, domain $mathbb{Z}_{\ge 0}$, $\forall (w,s)\in A$. `Demo7` implements this as `UIntVariable2`; it does not add a per-arc upper bound.

### 2. Auxiliary Variables

There are no separately declared auxiliary decision variables. `Shipment_w`, `Purchase_s`, and `Cost` in Section 6 are registered expression/intermediate symbols derived from $x$.

---

## 4. Predicates

### 1. Modeled Route Predicate

> Predicates classify entity sets; this predicate records whether a source cost map contains a route.

**`HasRoute(w,s)`**: true exactly when `warehouse.cost` defines a coefficient for store $s$; only such pairs contribute to the source expressions.

---

## 5. Sets

### 1. Warehouse and Store Categories

**$W$**: the universal set of warehouses.

**$S$**: the universal set of stores.

**$A$**: modeled warehouse-store arcs, $A=\{(w,s)\in W\times S\mid \mathrm{HasRoute}(w,s)\}$.

In the supplied Kotlin data every warehouse has a cost for every store, so $A=W\times S$; the Rust source likewise stores one cost per table cell.

### 2. Entity Pairs / Relations

**$ShipRoute$**: relation $A$ linking a warehouse to a store that it can serve.

---

## 6. Intermediate Values

### 1. Total Transportation Cost

**Description**: The total cost paid for all selected shipments; each shipment is multiplied by its warehouse-store unit cost.

$$
Cost=\sum_{(w,s)\in A}Cost_{ws}x_{ws}.
$$

### 2. Warehouse Shipment

**Description**: The amount shipped by warehouse $w$ across all modeled routes. It is the quantity compared with that warehouse's capacity.

$$
Shipment_w=\sum_{s:(w,s)\in A}x_{ws},\qquad \forall w\in W.
$$

### 3. Store Purchase

**Description**: The amount delivered to store $s$ from all warehouses. It is the quantity compared with the store's demand.

$$
Purchase_s=\sum_{w:(w,s)\in A}x_{ws},\qquad \forall s\in S.
$$

---

## 7. Assertions

### 1. Demand Is Feasible for the Sample Data

**Description**: The sample has enough aggregate capacity to cover aggregate demand; this is a data consistency check, not a replacement for per-warehouse constraints.

$$
\sum_{s\in S}Demand_s=1500\le 1500=\sum_{w\in W}Stowage_w.
$$

### 2. Non-Negative Shipment Accounting

**Description**: Every modeled shipment and both derived totals are non-negative.

$$
\forall (w,s)\in A\;(x_{ws}\ge0)\;\wedge\;\forall w\in W\;(Shipment_w\ge0)\;\wedge\;\forall s\in S\;(Purchase_s\ge0).
$$

---

## 8. Constraints

### 1. Warehouse Capacity (仓库容量约束)

**Description**: A warehouse cannot dispatch more goods than its stowage capacity.

$$
s.t. \quad Shipment_w\le Stowage_w,\qquad \forall w\in W.
$$

### 2. Store Demand (商店需求约束)

**Description**: Every store must receive at least its stated demand. The current source does not require exact equality.

$$
s.t. \quad Purchase_s\ge Demand_s,\qquad \forall s\in S.
$$

### 3. Shipment Domain (发货量定义域约束)

**Description**: Shipments are integral quantities and cannot be negative.

$$
s.t. \quad x_{ws}\in\mathbb{Z}_{\ge0},\qquad \forall (w,s)\in A.
$$

---

## 9. Objective Function (if applicable)

**Description**: Minimize the total transportation cost. Because all costs are non-negative in the supplied data, unnecessary over-delivery is not attractive, but exact demand is still not a hard constraint.

$$
\min Cost.
$$

---

## 10. Algorithm References

No standalone algorithm document is referenced. The model uses the regular `LinearMetaModel`/`MetaModel` registration path and `ScipLinearSolver`/Rust solver adapter.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| None | — | — | No standalone algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Warehouse | $w\in W$ | Origin that supplies shipments. |
| Store | $s\in S$ | Destination with a minimum demand. |
| Shipment | $x_{ws}$ | Integer quantity sent on a modeled route. |
| Stowage | $Stowage_w$ | Maximum total shipment from a warehouse. |
| Purchase | $Purchase_s$ | Total quantity received by a store. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use non-negative integer shipment variables | Continuous flow or binary route selection | The current Kotlin source uses `UIntVariable2`, and the Rust source uses `UInteger` | 2026-09-08 |
| Use at-least-demand constraints | Exact-demand equality | This preserves the current `geq` API and source semantics | 2026-09-08 |
| Treat Kotlin and Rust snippets as independent API examples | Present one pseudo-API | The implementations use different meta-model, combination, and symbol APIs | 2026-09-08 |

### Minimal current model-building snippets

The data (`stores`, `warehouses`) and the converter are taken from `Demo7.kt`; the Rust data is built by `build_warehouses()` and `build_stores()` in `demo7.rs`. The snippets show only the model construction and are not a promise that either block is a standalone file.

::: code-group
```kotlin [Kotlin]
// `stores`, `warehouses`, and `flt64Converter` come from Demo7.kt.
val metaModel = LinearMetaModel<Flt64>("demo7", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(warehouses.size, stores.size))
metaModel.add(x)

val cost = LinearExpressionSymbol(
    sum(warehouses.map { w ->
        sum(stores.filter { w.cost.contains(it) }.map { s -> w.cost[s]!! * x[w, s] })
    }),
    name = "cost"
)
val shipment = LinearIntermediateSymbols1<Flt64>("shipment", Shape1(warehouses.size)) { i, _ ->
    val w = warehouses[i]
    LinearExpressionSymbol(
        sum(stores.filter { w.cost.contains(it) }.map { s -> x[w, s] }),
        name = "shipment_${w.index}"
    )
}
val purchase = LinearIntermediateSymbols1<Flt64>("purchase", Shape1(stores.size)) { i, _ ->
    val s = stores[i]
    LinearExpressionSymbol(
        sum(warehouses.filter { w -> w.cost.contains(s) }.map { w -> x[w, s] }),
        name = "purchase_${s.index}"
    )
}
metaModel.add(cost)
metaModel.add(shipment)
metaModel.add(purchase)
metaModel.minimize(cost, "cost")
for (w in warehouses) {
    metaModel.addConstraint(shipment[w] leq w.stowage, name = "stowage_${w.index}")
}
for (s in stores) {
    metaModel.addConstraint(purchase[s] geq s.demand, name = "demand_${s.index}")
}
```

```rust [Rust]
// `warehouses` and `stores` come from demo7.rs; this is the register/constraint fragment.
let mut model = MetaModel::<f64>::new("demo7");
let x_vars: VariableCombination2D<UInteger> = VariableCombination2D::with_name_generator(
    Shape::new([warehouses.len(), stores.len()]),
    "x",
    |_index, vector| format!("{}_{}", vector[0], vector[1]),
);
let x_idx = model.register_combination(&x_vars)?;
let cost = flat_map1_indexed(
    "cost",
    warehouses,
    |w, warehouse| {
        let monomials = stores.iter().enumerate().map(|(s, _)|
            ospf_rust_core::symbol::flatten::LinearMonomial::new(
                warehouse.cost_to(s), x_idx[&[w, s]],
            )
        ).collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, warehouse| warehouse.name.clone(),
);
model.add_symbol_combination(&cost)?;
let shipment = flat_map1_indexed("shipment", warehouses, |w, _| {
    let monomials = stores.iter().enumerate().map(|(s, _)|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, warehouse| warehouse.name.clone());
let purchase = flat_map1_indexed("purchase", stores, |s, _| {
    let monomials = warehouses.iter().enumerate().map(|(w, _)|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, store| store.name.clone());
model.add_symbol_combination(&shipment)?;
model.add_symbol_combination(&purchase)?;
let mut cost_coeffs = Vec::new();
for w in 0..warehouses.len() {
    for monomial in cost.symbol_polynomial(w).monomials() {
        cost_coeffs.push((monomial.var_index(), *monomial.coefficient()));
    }
}
model.set_linear_objective_input(
    LinearObjectiveInput::minimize("cost").terms(cost_coeffs.into_iter())
);
for w in 0..warehouses.len() {
    model.add_linear_constraint(
        &extract_coeffs(&shipment[w]), ConstraintRelation::LessEqual,
        warehouses[w].stowage, &format!("stowage_{}", w),
    )?;
}
for s in 0..stores.len() {
    model.add_linear_constraint(
        &extract_coeffs(&purchase[s]), ConstraintRelation::GreaterEqual,
        stores[s].demand, &format!("demand_{}", s),
    )?;
}
```
:::

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 2026-09-08 | Reorganized the page into the domain-model template; added quantified intermediate definitions, bilingual constraint names, and Kotlin/Rust tabs | Align documentation with the current Demo7 implementations |

## Source and verification

- [Kotlin implementation: `Demo7.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo7.kt)
- [Rust counterpart: `demo7.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo7.rs)
- [Kotlin structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
