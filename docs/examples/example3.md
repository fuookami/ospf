# Example 3: Exact-yield material planning

## 1. Overview

This bounded context chooses non-negative integer quantities of raw materials so that every product reaches its required yield exactly, while minimising material cost.

### 1. Dependent Contexts

1. Core integer linear optimisation context (integer variables, linear intermediate symbols, constraints, and solver adapter).

The Kotlin and Rust snippets are model-building fragments. Their material and product data come from the linked Demo3 implementations.

---

## 2. Concepts / Entities

### 1. Product Target

A product target specifies the required yield of one product.

**$Demand_p$** : required yield of product $p$. In the Kotlin source this value is the Product.minYield field; in the Rust source it is ProductTarget.min_yield.

### 2. Material

A material is an integer-usable raw material with a unit cost and product-specific yields.

**$Cost_m$** : unit cost of material $m$.

**$Yield_{mp}$** : yield of product $p$ produced by one unit of material $m$; an omitted map/vector entry contributes zero in the current data.

The current product targets are:

| Product | P1 | P2 | P3 |
| :---: | ---: | ---: | ---: |
| Required yield | 15000 | 15000 | 10000 |

The current material data are:

| Material | Cost | P1 yield | P2 yield | P3 yield |
| :---: | ---: | ---: | ---: | ---: |
| A | 115 | 30 | 10 | 0 |
| B | 97 | 15 | 0 | 20 |
| C | 82 | 0 | 25 | 15 |
| D | 76 | 15 | 15 | 15 |

---

## 3. Variables

### 1. Decision Variables

**$x_m$** : material quantity, material-unit quantity, domain $\mathbb{Z}_{\ge0}$, the integer number of units of material $m$ to use, $\forall m\in M$.

### 2. Auxiliary Variables

None. Cost and per-product yield are registered linear expressions/intermediate symbols.

---

## 4. Predicates

### 1. Material and Yield Coverage

> Predicates classify materials and material-product relations.

**YieldDefined**$(m,p)$ : material $m$ has an explicit non-zero yield entry for product $p$.

**Used**$(m)$ : the solution assigns a positive quantity to material $m$, equivalently $x_m>0$.

---

## 5. Sets

### 1. Material Category

**$M$** : universal set of raw materials.

### 2. Product Category

**$P$** : universal set of product targets.

### 3. Entity Pairs / Relations

**$E$** : defined material-product yield relation, $E=\{(m,p)\in M\times P\mid YieldDefined(m,p)\}$.

Missing pairs in $M\times P$ are treated as zero contribution and are omitted when the source constructs the yield expression.

---

## 6. Intermediate Values

### 1. Total Material Cost

**Description**: Total material cost is the sum of each material's unit cost multiplied by its selected integer quantity.

$$
Cost(x)=\sum_{m\in M}Cost_mx_m.
$$

### 2. Product Yield

**Description**: Product yield is the total amount of product $p$ produced by all materials with a defined yield entry; it is defined for every product target.

$$
Yield_p(x)=\sum_{m\in M:(m,p)\in E}Yield_{mp}x_m,\qquad \forall p\in P.
$$

---

## 7. Assertions

### 1. Integer Material Quantity

**Description**: Every material quantity is a non-negative integer.

$$
\forall m\in M\; (x_m\in\mathbb{Z}_{\ge0}).
$$

### 2. Non-negative Data

**Description**: Current costs, yields, and demands are non-negative physical quantities.

$$
\forall m\in M\;(Cost_m\ge0)
\;\wedge\;
\forall(m,p)\in E\;(Yield_{mp}\ge0)
\;\wedge\;
\forall p\in P\;(Demand_p\ge0).
$$

---

## 8. Constraints

> The source deliberately registers two constraints for each product. Together they express exact yield, not only a lower bound.

### 1. Minimum Product Yield

**[最低产品产量]**: every product must reach at least its required yield.

$$
s.t.\quad Yield_p(x)\ge Demand_p,\qquad \forall p\in P.
$$

### 2. Maximum Product Yield

**[最高产品产量]**: every product must not exceed its required yield.

$$
s.t.\quad Yield_p(x)\le Demand_p,\qquad \forall p\in P.
$$

**Corollary**: the two hard bounds imply exact yield for every target product.

$$
\forall p\in P\;
\bigl(Yield_p(x)\ge Demand_p\;\wedge\;Yield_p(x)\le Demand_p\bigr)
\Rightarrow Yield_p(x)=Demand_p.
$$

---

## 9. Objective Function (if applicable)

**Description**: minimise the cost of the integer material plan.

$$
\min\; Cost(x)=\sum_{m\in M}Cost_mx_m.
$$

One displayed optimum is $(x_A,x_B,x_C,x_D)=(284,8,232,424)$, which yields $(15000,15000,10000)$.

---

## 10. Algorithm References

No standalone algorithm document is referenced. The model is a direct integer linear formulation; the yield expressions are built from the defined material-product entries.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Not applicable | — | — | No domain-specific algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Material | $m\in M$ | Raw material available in integer units. |
| Product target | $p\in P$ | Product with a required exact yield. |
| Material quantity | $x_m$ | Integer amount of material $m$. |
| Unit cost | $Cost_m$ | Cost per unit of material $m$. |
| Yield coefficient | $Yield_{mp}$ | Product $p$ yield from one unit of material $m$. |
| Product yield | $Yield_p(x)$ | Total produced amount of product $p$. |
| Demand | $Demand_p$ | Required amount of product $p$. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use non-negative integer material variables | Continuous or signed quantities | Kotlin uses UIntVariable1 and Rust uses UInteger; material quantities cannot be fractional or negative. | Current implementation |
| Omit undefined yield entries | Add explicit zero monomials | Kotlin filters the yield map; Rust filters zero coefficients before constructing the linear expression. | Current implementation |
| Keep both product-yield bounds | Add only a lower bound | The current source adds geq and leq constraints, so the result is exact yield. | Current implementation |

### Minimal current implementation fragments

The following are non-standalone fragments from Demo3. materials, products/targets, model setup, converter, and solver setup are supplied by the linked source.

::: code-group

```kotlin [Kotlin]
// Fragment from Demo3.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo3's private materials and products lists.
val x = UIntVariable1("x", Shape1(materials.size))
val cost = LinearExpressionSymbol(
    sum(materials) { it.cost * x[it] },
    name = "cost"
)
val yield = LinearIntermediateSymbols1<Flt64>(
    "yield",
    Shape1(products.size)
) { p, _ ->
    val product = products[p]
    LinearExpressionSymbol(
        sum(materials.filter { it.yieldQuantity.contains(product) }) { m ->
            m.yieldQuantity[product]!! * x[m]
        },
        name = "yield_product"
    )
}
metaModel.add(x)
metaModel.add(cost)
metaModel.add(yield)
metaModel.minimize(cost)
for (p in products) {
    metaModel.addConstraint(yield[p.index] geq p.minYield)
    metaModel.addConstraint(yield[p.index] leq p.minYield)
}
```

```rust [Rust]
// Fragment from demo3.rs::BlendingModel::register/add_constraints.
// Data source: build_materials/build_product_targets; zero entries are filtered.
let x = VariableCombination1D::new(Shape::new([materials.len()]), "x");
let x_idx = model.register_combination(&x)?;
let cost = flat_map1_indexed(
    "cost",
    materials,
    |m_idx, m| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                m.unit_cost,
                x_idx[m_idx],
            )],
            0.0,
        )
    },
    |_, m| m.name.clone(),
);
model.add_symbol_combination(&cost)?;
let yields = flat_map1_indexed(
    "yield",
    targets,
    |p, _target| {
        let monomials: Vec<_> = materials
            .iter()
            .enumerate()
            .filter_map(|(m_idx, m)| {
                let coeff = m.yields[p];
                if coeff != 0.0 {
                    Some(ospf_rust_core::symbol::flatten::LinearMonomial::new(
                        coeff,
                        x_idx[m_idx],
                    ))
                } else {
                    None
                }
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, target| target.name.clone(),
);
model.add_symbol_combination(&yields)?;
let cost_coeffs = extract_coeffs(&cost[0]);
model.add_linear_objective(&cost_coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for (p, target) in targets.iter().enumerate() {
    let coeffs = extract_coeffs(&yields[p]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::GreaterEqual,
        target.min_yield,
        &format!("yield_{}_lb", target.name),
    )?;
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::LessEqual,
        target.min_yield,
        &format!("yield_{}_ub", target.name),
    )?;
}
```

:::

#### Source and verification

- [Rust counterpart: src/core/demo3.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo3.rs)

The Rust counterpart uses the same mathematical model and data, but its Rust MetaModel, variable-combination, and symbol-combination APIs are independent of the Kotlin API.

- [Current implementation: Demo3.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo3.kt)
- [Core build-structure test: CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The Kotlin source names the product field minYield; the Rust source names the equivalent field min_yield. Both add lower and upper yield constraints for every product target. The fragments above are model-building excerpts rather than complete runnable programs.

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganised the example into the domain-model template and added yield assertions, quantified constraints, and Kotlin/Rust fragments. | Make exact-yield semantics and missing-entry handling explicit. |
