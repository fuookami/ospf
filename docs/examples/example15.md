# Example 15: Multi-plant distribution with substitutions

## 1. Overview

This bounded context models distribution of four car models from three manufacturers to two distribution centers, with directional model-substitution rates. The current Kotlin data are:

| Center | M1 demand | M2 demand | M3 demand | M4 demand | Directional replacement records |
| :---: | ---: | ---: | ---: | ---: | :--- |
| Denver | 700 | 500 | 500 | 600 | M1→M2: 0.10; M2→M1: 0.10; M3→M4: 0.20; M4→M3: 0.20 |
| Miami | 600 | 500 | 200 | 100 | M1→M2: 0.10; M2→M1: 0.10; M2→M4: 0.05; M4→M2: 0.05 |

Manufacturer production maps and logistics costs are:

| Manufacturer | Available production by model | Denver cost | Miami cost |
| :---: | :--- | ---: | ---: |
| Los Angeles | M3: 700; M4: 300 | 80 | 215 |
| Detroit | M1: 500; M2: 600; M4: 400 | 100 | 108 |
| New Orleans | M1: 800; M2: 400 | 102 | 68 |

Rates are decimal fractions. The Kotlin source computes adjusted demand, but its current constraints do not use that expression; this distinction is part of the model contract documented below.

### 1. Dependent Contexts

None. Car models, replacement records, manufacturer capacities, center demands, and logistics costs are defined directly in `Demo15`.

---

## 2. Concepts / Entities

### 1. Car model

A car model is a product type that can be demanded by a distribution center and produced by a manufacturer.

**$Name_k$** : name of car model $k$ (M1–M4).

### 2. Distribution center

A distribution center has demand by model and a list of directional replacement rules.

**$Demand_{dk}$** : demand for model $k$ at center $d$; an absent Kotlin map entry is treated as zero by the adjusted-demand expression.

**$R_d$** : directional replacement records belonging to center $d$.

### 3. Replacement rule

A replacement rule permits some demand for a source model to be fulfilled by a target model at a bounded rate.

**$From_r$** and **$To_r$** : source and target models of replacement record $r$.

**$Rate^{Max}_{dr}$** : maximum replacement fraction for record $r$ at center $d$.

### 4. Manufacturer

A manufacturer produces selected models and has a per-center logistics cost.

**$Productivity_{mk}$** : source production-map value for model $k$ at manufacturer $m$.

**$c_{md}$** : logistics cost per shipped unit from manufacturer $m$ to center $d$.

---

## 3. Variables

### 1. Decision Variables

**$x_{mdk}$** : quantity of model $k$ shipped from manufacturer $m$ to center $d$, a shipment quantity, domain $\mathbb{Z}_{\ge 0}$, for every $(m,d,k)\in P\times D\times M$. If the source has no `productivity` entry for $(m,k)$, the Kotlin variable is fixed to zero.

**$y_{dr}$** : fraction of the source model's demand replaced by the target model under rule $r$ at center $d$, dimensionless and continuous, domain $[0,Rate^{Max}_{dr}]$, for every $d\in D,r\in R_d$. Kotlin implements it with `PctVariable1` and bounds each component by `Replacement.maximum`.

### 2. Auxiliary Variables

There are no solver-owned auxiliary variables. `Receive`, `Trans`, `ExDemand`, and `Cost` are intermediate expressions.

---

## 4. Predicates

### 1. Product and production predicates

**HasDemand($d,k$)** : center $d$ has a positive/defined demand entry for model $k$.

**CanProduce($m,k$)** : manufacturer $m$ has a `productivity` entry for model $k$.

### 2. Replacement predicates

**ReplacementFrom($r,k$)** : $From_r=k$, so rule $r$ reduces the required shipment of model $k$.

**ReplacementTo($r,k$)** : $To_r=k$, so rule $r$ increases the required shipment of model $k$.

---

## 5. Sets

### 1. Participants

**$P$** : the universal set of three manufacturers.

**$D$** : the universal set of two distribution centers.

**$M$** : the universal set of four car models.

### 2. Replacement and availability sets

**$R_d$** : the directional replacement-rule set for center $d$; opposite directions are separate records when both are present.

**$H$** : the available manufacturer–model relation,

$$
H=\{(m,k)\in P\times M\mid CanProduce(m,k)\}.
$$

It is the set of model types for which the source permits a manufacturer to ship a nonzero quantity.

### 3. Entity Pairs / Relations

**$Y$** : the replacement-variable index relation,

$$
Y=\{(d,r)\mid d\in D,\ r\in R_d\}.
$$

**$H\times D$** : the allowed shipment-index relation for $x_{mdk}$; the source still allocates a full three-dimensional array and fixes entries outside it to zero.

---

## 6. Intermediate Values

### 1. Received quantity

**Description**: `Receive` is the quantity of a model arriving at a distribution center from all manufacturers.

$$
Receive_{dk}=\sum_{m\in P}x_{mdk},\qquad \forall d\in D,\ k\in M.
$$

### 2. Transferred quantity

**Description**: `Trans` is the quantity of a model shipped by one manufacturer to all distribution centers.

$$
Trans_{mk}=\sum_{d\in D}x_{mdk},\qquad \forall m\in P,\ k\in M.
$$

### 3. Adjusted demand

**Description**: `ExDemand` is the source's demand expression after replacing part of a source model's demand by a target model. For a rule $r$, $y_{dr}$ is the selected replacement fraction. An absent demand map value is treated as zero in the Kotlin expression.

$$
ExDemand_{dk}=Demand_{dk}
-\sum_{\substack{r\in R_d\\From_r=k}}Demand_{dk}\,y_{dr}
+\sum_{\substack{r\in R_d\\To_r=k}}Demand_{d,From_r}\,y_{dr},
\qquad \forall d\in D,\ k\in M.
$$

The first sum removes demand for model $k$ that is replaced by another model; the second adds demand absorbed from another source model.

### 4. Logistics cost

**Description**: `Cost` is the total logistics cost of all manufacturer-to-center shipments. Replacement fractions do not occur in the cost expression.

$$
Cost=\sum_{m\in P}\sum_{d\in D}\sum_{k\in M}c_{md}x_{mdk},
$$

where entries with $(m,k)\notin H$ contribute zero because their $x_{mdk}$ variables are fixed to zero.

---

## 7. Assertions

### 1. Replacement preserves total demand units

**Description**: A replacement transfers demand between model types and does not change the total number of units required at a center.

$$
\sum_{k\in M}ExDemand_{dk}=\sum_{k\in M}Demand_{dk},\qquad \forall d\in D.
$$

### 2. Zero replacement reproduces base demand

**Description**: If every replacement fraction at a center is zero, its adjusted demand equals its original demand for every model.

$$
\left(\forall r\in R_d,\ y_{dr}=0\right)\Rightarrow
\left(\forall k\in M,\ ExDemand_{dk}=Demand_{dk}\right),\qquad \forall d\in D.
$$

### 3. Unavailable production is zero

**Description**: A manufacturer cannot ship a model absent from its source production map.

$$
\forall(m,k)\in(P\times M)\setminus H,\ \forall d\in D:\quad x_{mdk}=0.
$$

---

## 8. Constraints

### 1. Production Availability Fixing [生产可用性固定]

**Description**: Shipment variables for manufacturer–model pairs without a `productivity` entry are fixed to zero for every destination center.

$$
s.t.\quad x_{mdk}=0,\qquad \forall(m,k)\in(P\times M)\setminus H,\ \forall d\in D.
$$

### 2. Replacement Rate Bounds [替代比例界限]

**Description**: Every directional replacement fraction is nonnegative and cannot exceed its center-specific maximum.

$$
s.t.\quad 0\le y_{dr}\le Rate^{Max}_{dr},\qquad \forall d\in D,\ r\in R_d.
$$

### 3. Current Demand Coverage [当前需求覆盖]

**Description**: The current Kotlin implementation requires received quantity to cover the original demand for each defined demand entry. It does not use `ExDemand` here.

$$
s.t.\quad Receive_{dk}\ge Demand_{dk},\qquad \forall(d,k)\text{ with }HasDemand(d,k).
$$

### 4. Current Production Lower Bound [当前生产下界]

**Description**: The current Kotlin implementation uses a lower bound, requiring shipped quantity to be at least the mapped `productivity` value. This is the opposite of the usual capacity interpretation; no upper capacity constraint is registered in Kotlin.

$$
s.t.\quad Trans_{mk}\ge Productivity_{mk},\qquad \forall(m,k)\in H.
$$

**Corollary**: Under the current Kotlin constraints, $y$ affects neither feasibility nor the objective, and quantities are not bounded above by `productivity`. A capacity-style formulation such as $Trans_{mk}\le Productivity_{mk}$ and a demand formulation using $ExDemand$ would be a different implementation.

---

## 9. Objective Function

**Description**: Minimize logistics cost from manufacturers to distribution centers. The replacement variables are not priced.

$$
\min Cost=\min\sum_{m\in P}\sum_{d\in D}\sum_{k\in M}c_{md}x_{mdk}.
$$

The current Kotlin core build test checks structure only and does not establish a numeric optimum. The Rust counterpart has the same cost direction but different demand and production constraints described in Section 12.

---

## 10. Algorithm References

No standalone algorithm document is referenced; replacement adjustment and linear expressions are constructed directly in the core demo.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| None | — | — | Source-local replacement arithmetic and linear distribution model |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Manufacturer | $P$ / $m$ | Entity that ships available car models. |
| Distribution center | $D$ / $d$ | Destination with demand and replacement rules. |
| Car model | $M$ / $k$ | Product type being distributed. |
| Shipment | $x_{mdk}$ | Units of model $k$ shipped from manufacturer $m$ to center $d$. |
| Replacement rate | $y_{dr}$ | Fraction of source-model demand assigned to the target model under rule $r$. |
| Adjusted demand | $ExDemand_{dk}$ | Demand after applying directional replacement arithmetic. |
| Productivity | $Productivity_{mk}$ | Manufacturer–model value used by the current Kotlin lower-bound constraint. |
| Logistics cost | $c_{md}$ | Per-unit cost from manufacturer $m$ to center $d$. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Treat the Kotlin implementation as authoritative for the current-model section | Infer intended capacity semantics | The source registers `Receive ≥ Demand` and `Trans ≥ Productivity`; documenting `ExDemand` or `Trans ≤ Productivity` as active would be inaccurate. | 2026-09-08 |
| Keep directional replacement records separate | Collapse opposite directions into an undirected edge | Kotlin stores each `Replacement(c1,c2,maximum)` record independently. | 2026-09-08 |
| Document Rust as a non-equivalent counterpart | Present Kotlin and Rust as one model | Rust uses integer `UInteger` replacement variables, so all current rates below one force $y=0$; it also uses `Trans ≤ Productivity` and constrains `Receive ≥ ExDemand`. | 2026-09-08 |

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganized the page into domain-model sections, included source manufacturer data, and separated Kotlin and Rust semantics. | Make the unused Kotlin `ExDemand` and the cross-language model differences explicit. |

---

## Minimal current model-building example

The snippets are model-building fragments. Kotlin data and `flt64Converter` come from `Demo15.kt`; Rust builders, helpers, and `next_auto_intermediate_symbol_id` come from `demo15.rs`.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.math.symbol.polynomial.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

// manufacturers, distributionCenters, carModels, and flt64Converter come from Demo15.kt.
val model = LinearMetaModel<Flt64>("demo15", converter = flt64Converter)
val x = UIntVariable3("x", Shape3(manufacturers.size, distributionCenters.size, carModels.size))
for (m in manufacturers) for (d in distributionCenters) for (k in carModels) {
    if (m.productivity.containsKey(k)) model.add(x[m, d, k])
    else x[m, d, k].range.eq(UInt64.zero)
}
val y = distributionCenters.associateWith { d ->
    PctVariable1("y_${d.name}", Shape1(d.replacements.size)).also { rates ->
        for ((r, replacement) in d.replacements.withIndex()) rates[r].range.leq(replacement.maximum)
    }
}
val receive = LinearIntermediateSymbols2<Flt64>("receive", Shape2(distributionCenters.size, carModels.size)) { _, v ->
    LinearExpressionSymbol(sum(x[_a, distributionCenters[v[0]], carModels[v[1]]]), name = "receive_${v.joinToString("_")}")
}
val demand = LinearIntermediateSymbols2<Flt64>("demand", Shape2(distributionCenters.size, carModels.size)) { _, v ->
    val d = distributionCenters[v[0]]; val k = carModels[v[1]]
    val removed = sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
        if (replacement.c1 == k && (d.demands[k] ?: UInt64.zero) gr UInt64.zero)
            d.demands[k]!!.toFlt64() * y[d]!![r] else null
    })
    val added = sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
        if (replacement.c2 == k) d.demands[replacement.c1]?.let { it.toFlt64() * y[d]!![r] } else null
    })
    LinearExpressionSymbol(LinearPolynomial((d.demands[k] ?: UInt64.zero).toFlt64()) - removed + added,
        name = "demand_${d.name}_${k.name}")
}
val trans = LinearIntermediateSymbols2<Flt64>("trans", Shape2(manufacturers.size, carModels.size)) { _, v ->
    LinearExpressionSymbol(sum(x[manufacturers[v[0]], _a, carModels[v[1]]]), name = "trans_${v.joinToString("_")}")
}
val cost = LinearExpressionSymbol(sum(manufacturers.flatMap { m -> distributionCenters.flatMap { d ->
    m.logisticsCost[d]?.let { c -> carModels.filter { m.productivity.containsKey(it) }.map { k -> c * x[m, d, k] } } ?: emptyList()
} }}), name = "cost")
model.add(y.values.flatten()); model.add(receive); model.add(demand); model.add(trans); model.add(cost)
model.minimize(cost, "cost")
for (d in distributionCenters) for (k in carModels) d.demands[k]?.let { model.addConstraint(receive[d, k] geq it) }
for (m in manufacturers) for (k in carModels) m.productivity[k]?.let { model.addConstraint(trans[m, k] geq it) }

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

```rust [Rust]
use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::SymbolCombination;
use ospf_rust_core::variable::{UInteger, VariableCombination1D, VariableCombination3D, VariableRange};
use ospf_rust_multiarray::{MultiArray, Shape};

// build_car_models/build_centers/build_manufacturers and solve_typed are from demo15.rs.
let models = build_car_models(); let centers = build_centers(); let manufacturers = build_manufacturers();
let mut model = MetaModel::<f64>::new("demo15");
let x_vars = VariableCombination3D::with_name_and_range_generator(
    Shape::new([manufacturers.len(), centers.len(), models.len()]), "x", |_i, v| format!("{}_{}_{}", v[0], v[1], v[2]),
    |_i, v| if manufacturers[v[0]].productivity_by_model[v[2]].is_some() {
        VariableRange::with_lower(0.0)
    } else { VariableRange::fixed(0.0) });
let x_idx = model.register_combination(&x_vars)?;
let mut y_idx: Vec<MultiArray<usize, Shape<1>>> = Vec::new();
for d in 0..centers.len() {
    let y_vars = VariableCombination1D::with_name_and_range_generator(
        Shape::new([centers[d].replacements.len()]), &format!("y_{}", d), |_i, v| v[0].to_string(),
        |_i, v| VariableRange::bounded(0.0, centers[d].replacements[v[0]].max_ratio));
    y_idx.push(model.register_combination(&y_vars)?);
}
let total_cost = SymbolCombination::new(Shape::new([1]), "cost", |_i, _| {
    let terms = (0..manufacturers.len()).flat_map(|m| (0..centers.len()).flat_map(move |d| {
        (0..models.len()).map(move |k| ospf_rust_core::symbol::flatten::LinearMonomial::new(
            manufacturers[m].logistics_cost_to_centers[d], x_idx[&[m, d, k]]))
    })).collect();
    let p = ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0);
    LinearExpressionSymbol::new(ospf_rust_core::symbol::next_auto_intermediate_symbol_id(),
        "total_cost", p.monomials().to_vec(), *p.constant_term())
});
// The receive/trans/demand SymbolCombinations use the same x_idx/y_idx linear terms
// shown in DistributionModel::register in the linked source.
model.add_symbol_combination(&total_cost)?;
let coeffs = total_cost.symbol_polynomial(0).monomials().iter()
    .map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>();
model.add_linear_objective(&coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
// Rust's demand symbol is the replacement delta: receive + delta >= base demand.
for d in 0..centers.len() { for k in 0..models.len() {
    let coeffs = /* extract receive[d,k] + demand[d,k] as in demo15.rs */ Vec::new();
    model.add_linear_constraint(&coeffs, ConstraintRelation::GreaterEqual,
        centers[d].demands[k], &format!("demand_{}_{}", d, k))?;
}}
for m in 0..manufacturers.len() { for k in 0..models.len() {
    if let Some(cap) = manufacturers[m].productivity_by_model[k] {
        let coeffs = /* extract trans[m,k] as in demo15.rs */ Vec::new();
        model.add_linear_constraint(&coeffs, ConstraintRelation::LessEqual, cap,
            &format!("capacity_{}_{}", m, k))?;
    }
}}
let _output = solve_typed(model)?;
```

:::

## Source and verification

### Kotlin/Rust correspondence

- [Rust counterpart: `src/core/demo15.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo15.rs)

This counterpart is intentionally documented as non-equivalent to the current Kotlin model. Rust uses `UInteger` replacement variables with upper bounds $0.05$, $0.10$, or $0.20$; because the variables are integer, every replacement ratio is forced to $0$. Rust's demand symbol stores only the replacement delta and is constrained through `receive + demand ≥ base demand`, which is algebraically `Receive ≥ ExDemand`. Rust also uses `Trans ≤ Productivity`; Kotlin currently uses `Trans ≥ Productivity`. Both minimize logistics cost and fix unavailable manufacturer–model shipments to zero.

- [Current implementation: `Demo15.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo15.kt)
- [Core build-structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The Kotlin core build test checks model structure only and does not assert a numeric optimum. Re-solving either implementation should be described with that implementation's own constraints.
