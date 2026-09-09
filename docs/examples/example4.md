# Example 4: Production with material and balance limits

## 1. Overview

This bounded context chooses production quantities for two products, maximising profit under material-availability, product-yield, and pairwise-difference rules.

### 1. Dependent Contexts

1. Core continuous linear optimisation context (real variables, linear intermediate symbols, constraints, and solver adapter).

The Kotlin and Rust implementations are mathematically close but have different variable domains. The snippets below are model-building fragments whose data come from the linked Demo4 implementations.

---

## 2. Concepts / Entities

### 1. Material

A material is a consumable resource with a finite availability.

**$Available_m$** : available amount of material $m$.

The current material data are $Available_A=24$ and $Available_B=8$.

### 2. Product

A product has a profit, a maximum production quantity, and material usage coefficients.

**$Profit_p$** : profit per unit of product $p$.

**$Yield^{Max}_p$** : maximum production quantity of product $p$.

**$Use_{pm}$** : amount of material $m$ consumed by one unit of product $p$.

The current product data are:

| Product | Profit | Maximum yield | A per unit | B per unit |
| :---: | ---: | ---: | ---: | ---: |
| P1 | 5 | 3 | 6 | 1 |
| P2 | 4 | 2 | 4 | 2 |

The source sets $Diff^{Max}=1$ (maxDiff = 1).

---

## 3. Variables

### 1. Decision Variables

**$x_p$** : production quantity, continuous production amount, domain $\mathbb{R}$ in the current Kotlin implementation, representing the quantity of product $p$, $\forall p\in P$.

The Kotlin source uses RealVariable1 and sets only the upper bound $x_p\le Yield^{Max}_p$. The Rust counterpart uses UContinuous with $0\le x_p\le Yield^{Max}_p$; this implementation difference is intentional and documented below.

### 2. Auxiliary Variables

None. Material use and pairwise differences are derived linear values.

---

## 4. Predicates

### 1. Production Status

> Predicates classify products and ordered product pairs.

**WithinYield**$(p)$ : production of product $p$ is no greater than its configured maximum.

**MaterialFeasible**$(m)$ : total use of material $m$ is no greater than its availability.

**OrderedDistinct**$(p,q)$ : $p$ and $q$ are distinct products and the ordered pair is subject to the difference rule.

---

## 5. Sets

### 1. Material Category

**$M$** : universal set of materials.

### 2. Product Category

**$P$** : universal set of products.

### 3. Entity Pairs / Relations

**$D=\{(p,q)\in P\times P\mid p\ne q\}$** : ordered distinct-product pairs.

Both $(p,q)$ and $(q,p)$ are present for every two distinct products in the current implementation.

---

## 6. Intermediate Values

### 1. Total Profit

**Description**: Total profit is the profit per product multiplied by the selected production quantity; it is the objective expression.

$$
Profit(x)=\sum_{p\in P}Profit_px_p.
$$

### 2. Material Use

**Description**: Material use records the total consumption of each material across all products.

$$
Use_m(x)=\sum_{p\in P}Use_{pm}x_p,\qquad \forall m\in M.
$$

### 3. Ordered Production Difference

**Description**: The ordered production difference is a derived value used by the pairwise rule; it is not a separate registered decision variable.

$$
Difference_{pq}(x)=x_p-x_q,\qquad \forall(p,q)\in D.
$$

---

## 7. Assertions

### 1. Non-negative Current Data

**Description**: All current profits, capacities, yields, material uses, and difference limits are non-negative data.

$$
\forall p\in P\;(Profit_p\ge0\wedge Yield^{Max}_p\ge0)
\;\wedge\;
\forall m\in M\;Available_m\ge0
\;\wedge\;
\forall(p,m)\in P\times M\;Use_{pm}\ge0.
$$

### 2. Reverse Ordered Pairs

**Description**: Every distinct product pair has both orientations, which makes the one-sided difference family equivalent to an absolute-difference bound.

$$
\forall(p,q)\in D\; \bigl((q,p)\in D\bigr).
$$

---

## 8. Constraints

> These are the constraints present in the current Kotlin model; the Rust counterpart additionally encodes non-negativity in the variable range.

### 1. Maximum Product Yield

**[最大产品产量]**: each product's production quantity cannot exceed its configured maximum.

$$
s.t.\quad x_p\le Yield^{Max}_p,\qquad \forall p\in P.
$$

### 2. Material Availability

**[物料可用量上限]**: total use of each material cannot exceed the available amount.

$$
s.t.\quad Use_m(x)\le Available_m,\qquad \forall m\in M.
$$

### 3. Pairwise Production Difference

**[产品间产量差上限]**: the production quantity of one product may exceed another by at most $Diff^{Max}$ for every ordered distinct pair.

$$
s.t.\quad Difference_{pq}(x)=x_p-x_q\le Diff^{Max},\qquad \forall(p,q)\in D.
$$

**Corollary**: because both orientations are present, the pairwise family is equivalent to an absolute-difference bound.

$$
\forall(p,q)\in D\quad |x_p-x_q|\le Diff^{Max}.
$$

The current Kotlin model does not add $x_p\ge0$. Adding that business rule would change the Kotlin implementation; the Rust counterpart already uses the bounded range $0\le x_p\le Yield^{Max}_p$.

---

## 9. Objective Function (if applicable)

**Description**: maximise total production profit.

$$
\max\; Profit(x)=\sum_{p\in P}Profit_px_p.
$$

For the displayed data, $(x_{P1},x_{P2})=(8/3,5/3)$ is an optimum of the implemented Kotlin model; both values are positive even though the Kotlin variable domain permits negative values.

---

## 10. Algorithm References

No standalone algorithm document is referenced. This is a direct continuous linear formulation.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Not applicable | — | — | No domain-specific algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Material | $m\in M$ | Consumable resource with an availability limit. |
| Product | $p\in P$ | Product with profit and an upper production limit. |
| Production quantity | $x_p$ | Continuous quantity of product $p$. |
| Material use | $Use_m(x)$ | Total consumption of material $m$. |
| Pairwise difference | $Difference_{pq}(x)$ | Ordered difference $x_p-x_q$. |
| Maximum yield | $Yield^{Max}_p$ | Upper production bound for product $p$. |
| Difference limit | $Diff^{Max}$ | Maximum allowed ordered production difference. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Keep Kotlin RealVariable1 signed | Add an explicit non-negative lower bound | The current Kotlin source sets only x[p].range.ls(maxYield); negative values remain allowed. | Current implementation |
| Keep Rust's bounded non-negative range documented as a difference | Force identical variable domains | Rust uses UContinuous and VariableRange::bounded(0.0, maxYield), so it excludes the Kotlin-only negative region. | Current implementation |
| Register both orientations of the difference rule | Add one absolute-value constraint | The source loops over every ordered pair p1 != p2; the two orientations give the same business effect. | Current implementation |

### Minimal current implementation fragments

The following are non-standalone fragments from Demo4. products, materials, maxDiff, model setup, converter, and solver setup come from the linked source.

::: code-group

```kotlin [Kotlin]
// Fragment from Demo4.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo4's private materials/products lists and maxDiff.
val x = RealVariable1("x", Shape1(products.size))
val profit = LinearExpressionSymbol(
    sum(products) { p -> p.profit * x[p] },
    name = "profit"
)
val use = LinearIntermediateSymbols1<Flt64>(
    "use",
    Shape1(materials.size)
) { m, _ ->
    val material = materials[m]
    LinearExpressionSymbol(
        sum(products.filter { it.use.contains(material) }) { p ->
            p.use[material]!! * x[p]
        },
        name = "use"
    )
}
metaModel.add(x)
metaModel.add(profit)
metaModel.add(use)
metaModel.maximize(profit, "profit")
for (p in products) {
    x[p].range.ls(p.maxYield)
}
for (m in materials) {
    metaModel.addConstraint(use[m] leq m.available)
}
for (p1 in products) {
    for (p2 in products) {
        if (p1.index != p2.index) {
            metaModel.addConstraint((x[p1] - x[p2]) leq maxDiff.toFlt64())
        }
    }
}
```

```rust [Rust]
// Fragment from demo4.rs::ProductionModel::register/add_constraints.
// Data source: build_materials/build_products; the Rust range is explicit.
let x = VariableCombination1D::with_range_generator(
    Shape::new([products.len()]),
    "x",
    |i, _| VariableRange::bounded(0.0, products[i].max_yield),
);
let x_idx = model.register_combination(&x)?;
let profit = flat_map1_indexed(
    "profit",
    products,
    |i, product| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                product.profit,
                x_idx[i],
            )],
            0.0,
        )
    },
    |_, product| product.name.clone(),
);
model.add_symbol_combination(&profit)?;
let r#use = flat_map1_indexed(
    "usage",
    materials,
    |m, _material| {
        let monomials: Vec<_> = products
            .iter()
            .enumerate()
            .map(|(p, product)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    product.usage_by_material[m],
                    x_idx[p],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, material| material.name.clone(),
);
model.add_symbol_combination(&r#use)?;
let profit_coeffs = extract_coeffs(&profit[0]);
model.add_linear_objective(&profit_coeffs, "profit");
model.set_objective_category(ObjectiveCategory::Maximum);
for (m, material) in materials.iter().enumerate() {
    let coeffs = extract_coeffs(&r#use[m]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::LessEqual,
        material.available,
        &format!("material_{}_{}", m, material.name),
    )?;
}
for p1 in 0..products.len() {
    for p2 in 0..products.len() {
        if p1 != p2 {
            let coefficients = vec![(x_idx[p1], 1.0), (x_idx[p2], -1.0)];
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("diff_{}_{}", p1, p2),
            )?;
        }
    }
}
```

:::

#### Source and verification

- [Rust counterpart: src/core/demo4.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo4.rs)

The Rust counterpart is mathematically close but not domain-identical: it bounds production by 0 <= x <= maxYield, while the current Kotlin implementation uses RealVariable1 with only x <= maxYield.

- [Current implementation: Demo4.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo4.kt)
- [Core build-structure test: CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The Kotlin source uses `LinearExpressionSymbol` for profit, `LinearIntermediateSymbols1` for material use, and `RealVariable1` for production. The Rust source uses `VariableCombination1D<UContinuous>` and a bounded `VariableRange`. The fragments are excerpts rather than complete runnable programs.

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganised the example into the domain-model template and added quantified constraints, a pairwise corollary, and Kotlin/Rust fragments. | Make the signed-domain discrepancy explicit. |
