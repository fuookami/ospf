# Example 13: Two-stage transportation with truck counts

## 1. Overview

This bounded context models shipping from three distribution centers to five dealers while respecting center supply, dealer demand, and truck capacity. The source data are:

| Distribution center | Dealer 1 | Dealer 2 | Dealer 3 | Dealer 4 | Dealer 5 | Supply |
| :---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DC1 | 100 | 150 | 200 | 140 | 35 | 400 |
| DC2 | 50 | 70 | 60 | 65 | 80 | 200 |
| DC3 | 40 | 90 | 100 | 150 | 130 | 150 |
| Dealer demand | 100 | 200 | 150 | 160 | 140 | -- |

The entries are the distances in each distribution center's `distance` map. One truck has capacity $Q=18$ units. All five dealer maps are complete in the current data, and total supply and total demand are both $750$.

### 1. Dependent Contexts

None. The example is a source-local transportation model; dealer, center, distance, and truck-capacity data are supplied directly by `Demo13`.

---

## 2. Concepts / Entities

### 1. Dealer

A dealer is a demand point that must receive the required number of units.

**$Demand_d$** : integer demand of dealer $d$, measured in shipment units.

### 2. Distribution Center

A distribution center supplies goods and has a distance to each dealer.

**$Supply_c$** : integer supply available at center $c$, measured in shipment units.

**$distance_{cd}$** : integer distance from center $c$ to dealer $d$, used as the cost coefficient for a truck assigned to that pair.

### 3. Truck assignment

A truck assignment records the number of trucks allocated to one center–dealer pair. It is represented by the decision variable $y_{dc}$ rather than by a separate route entity.

---

## 3. Variables

### 1. Decision Variables

**$x_{dc}$** : shipped quantity from center $c$ to dealer $d$, a shipment quantity, domain $\mathbb{Z}_{\ge 0}$, with one integer value for every $(d,c)\in D\times C$.

**$y_{dc}$** : number of trucks assigned from center $c$ to dealer $d$, a dimensionless integer count, domain $\mathbb{Z}_{\ge 0}$, for every $(d,c)\in D\times C$. The source creates both arrays with `UIntVariable2` and indexes them as `[dealer, distributionCenter]`.

### 2. Auxiliary Variables

There are no auxiliary variables. `Trans`, `Receive`, and `Cost` are intermediate expressions, not solver-owned auxiliary decision variables.

---

## 4. Predicates

### 1. Transport participant predicates

**Dealer($d$)** : $d$ is an entity that has a demand value and receives shipments.

**Center($c$)** : $c$ is an entity that has a supply value and sends shipments.

### 2. Route-data predicates

**DistanceDefined($c,d$)** : the source distance map contains the center–dealer pair $(c,d)$ and therefore supplies a cost coefficient. In the current data this predicate is true for every pair in $C\times D$.

---

## 5. Sets

### 1. Dealers and centers

**$D$** : the universal set of the five dealers.

**$C$** : the universal set of the three distribution centers.

### 2. Transport pairs

**$A$** : the set of center–dealer pairs with a defined distance,

$$
A=\{(d,c)\in D\times C\mid DistanceDefined(c,d)\}.
$$

For this instance, $A=D\times C$; the source nevertheless uses full two-dimensional variable arrays.

### 3. Entity Pairs / Relations

**$A\subseteq D\times C$** : the directed shipment relation from a distribution center to a dealer. No reverse direction or route sequence is modeled.

---

## 6. Intermediate Values

### 1. Center shipment total

**Description**: `Trans` is the total number of units shipped by center $c$ over all dealer destinations.

$$
Trans_c=\sum_{d:(d,c)\in A}x_{dc},\qquad \forall c\in C.
$$

### 2. Dealer receipt total

**Description**: `Receive` is the total number of units received by dealer $d$ from all centers.

$$
Receive_d=\sum_{c:(d,c)\in A}x_{dc},\qquad \forall d\in D.
$$

### 3. Truck-distance cost

**Description**: `Cost` charges the distance once for every assigned truck. It is therefore a distance-per-truck objective, not a distance-per-shipped-unit objective.

$$
Cost=\sum_{(d,c)\in A}distance_{cd}\,y_{dc}.
$$

---

## 7. Assertions

### 1. Flow identity

**Description**: Every unit counted as shipped by a center is counted as received by exactly one dealer.

$$
\sum_{c\in C}Trans_c=\sum_{d\in D}Receive_d=\sum_{(d,c)\in A}x_{dc}.
$$

### 2. Balanced input data

**Description**: The instance supplies exactly enough total goods to cover all dealer demand.

$$
\sum_{c\in C}Supply_c=400+200+150=750=100+200+150+160+140=\sum_{d\in D}Demand_d.
$$

### 3. Complete current distance relation

**Description**: Every current dealer–center pair has a distance coefficient, so no pair is omitted from the current cost expression.

$$
A=D\times C,\qquad |D|=5,\qquad |C|=3.
$$

---

## 8. Constraints

### 1. Supply Capacity [供应能力上限]

**Description**: A distribution center cannot ship more units than its available supply.

$$
s.t.\quad Trans_c\le Supply_c,\qquad \forall c\in C.
$$

### 2. Dealer Demand Coverage [经销商需求满足]

**Description**: Every dealer must receive at least its stated demand.

$$
s.t.\quad Receive_d\ge Demand_d,\qquad \forall d\in D.
$$

### 3. Truck Capacity Linking [卡车容量联结]

**Description**: The shipment assigned to a pair cannot exceed the capacity of its assigned trucks. The source does not model truck routes or split-load sequencing.

$$
s.t.\quad x_{dc}\le Qy_{dc},\qquad \forall(d,c)\in A,\qquad Q=18.
$$

**Corollary**: Because the total supply and total demand are both $750$, the inequality families imply equality in aggregate; with nonnegative flows, every center's supply and every dealer's demand are tight in this instance.

$$
\sum_{c\in C}Trans_c=\sum_{c\in C}Supply_c=750,
\qquad
\sum_{d\in D}Receive_d=\sum_{d\in D}Demand_d=750.
$$

---

## 9. Objective Function

**Description**: Minimize the sum of center–dealer distances weighted by the number of trucks assigned to each pair.

$$
\min Cost=\min\sum_{(d,c)\in A}distance_{cd}\,y_{dc}.
$$

The current source does not replace $y$ with $x$ in this objective; doing so would describe a different model.

---

## 10. Algorithm References

No standalone algorithm document is referenced. The model is built directly in the linked core demo.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| None | — | — | Source-local linear transportation expressions and constraints |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Dealer | $D$ / $d$ | A demand point that receives goods. |
| Distribution center | $C$ / $c$ | A supply point that dispatches goods. |
| Shipment | $x_{dc}$ | Units sent from center $c$ to dealer $d$. |
| Truck count | $y_{dc}$ | Integer trucks assigned to the center–dealer pair. |
| Truck capacity | $Q$ | Maximum units carried by one truck; $Q=18$. |
| Distance cost | $Cost$ | Distance-weighted truck count minimized by the model. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Keep the source array orientation as `[dealer, distributionCenter]` | Use `[center, dealer]` | Documents the actual `UIntVariable2` indexing while keeping $x_{dc}$ semantically center-to-dealer. | 2026-09-08 |
| Charge distance per truck through $y$ | Charge distance per shipped unit through $x$ | The current `Demo13` objective is defined from `y`; changing it would change the model. | 2026-09-08 |
| Retain supply and demand inequalities | Replace both by equalities | The inequalities are the constraints actually registered; equal aggregate totals are documented as a corollary. | 2026-09-08 |

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganized the example into the domain-model sections and documented the current Kotlin/Rust core APIs. | Align the page with the domain-model template and preserve implementation-specific semantics. |

---

## Minimal current model-building example

The snippets below are model-building fragments. `dealers`, `distributionCenters`, `carCapacity`, `flt64Converter`, and the Rust data/helper builders are supplied by the linked source files.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

// Data and flt64Converter come from Demo13.kt.
val model = LinearMetaModel<Flt64>("demo13", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(dealers.size, distributionCenters.size))
val y = UIntVariable2("y", Shape2(dealers.size, distributionCenters.size))
val trans = LinearIntermediateSymbols1<Flt64>("trans", Shape1(distributionCenters.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, distributionCenters[i]]), name = "trans_$i")
}
val receive = LinearIntermediateSymbols1<Flt64>("receive", Shape1(dealers.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[dealers[i], _a]), name = "receive_$i")
}
val cost = LinearExpressionSymbol(
    sum(dealers.flatMap { dealer -> distributionCenters.mapNotNull { center ->
        center.distance[dealer]?.let { it * y[dealer, center] }
    } }),
    name = "cost"
)
model.add(x); model.add(y); model.add(trans); model.add(receive); model.add(cost)
model.minimize(cost, "cost")
for (center in distributionCenters) model.addConstraint(trans[center] leq center.supply)
for (dealer in dealers) model.addConstraint(receive[dealer] geq dealer.demand)
for (dealer in dealers) for (center in distributionCenters) {
    model.addConstraint(x[dealer, center] - carCapacity.toFlt64() * y[dealer, center] leq Flt64.zero)
}

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

```rust [Rust]
use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::flat_map1_indexed;
use ospf_rust_core::variable::{UInteger, VariableCombination2D};
use ospf_rust_multiarray::Shape;

// build_dealers/build_centers, extract_coeffs, and solve_typed are from demo13.rs.
let dealers = build_dealers();
let centers = build_centers();
let capacity = 18.0;
let mut model = MetaModel::<f64>::new("demo13");
let shape = Shape::new([dealers.len(), centers.len()]);
let x_vars = VariableCombination2D::with_name_generator(shape.clone(), "x", |_i, v| {
    format!("{}_{}", v[0], v[1])
});
let y_vars = VariableCombination2D::with_name_generator(shape, "y", |_i, v| {
    format!("{}_{}", v[0], v[1])
});
let x_idx = model.register_combination(&x_vars)?;
let y_idx = model.register_combination(&y_vars)?;

let cost = flat_map1_indexed("cost", &dealers, |d, dealer| {
    let terms = (0..centers.len()).map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(
        dealer.distance_to(c), y_idx[&[d, c]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, dealer| dealer.name.clone());
let trans = flat_map1_indexed("trans", &centers, |c, _| {
    let terms = (0..dealers.len()).map(|d| ospf_rust_core::symbol::flatten::LinearMonomial::new(
        1.0, x_idx[&[d, c]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, center| center.name.clone());
let receive = flat_map1_indexed("receive", &dealers, |d, _| {
    let terms = (0..centers.len()).map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(
        1.0, x_idx[&[d, c]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, dealer| dealer.name.clone());
model.add_symbol_combination(&cost)?;
model.add_symbol_combination(&trans)?;
model.add_symbol_combination(&receive)?;
let cost_coeffs = (0..dealers.len()).flat_map(|i| cost.symbol_polynomial(i).monomials().iter()
    .map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>()).collect::<Vec<_>>();
model.add_linear_objective(&cost_coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for c in 0..centers.len() {
    model.add_linear_constraint(&extract_coeffs(&trans[c]), ConstraintRelation::LessEqual,
        centers[c].supply, &format!("supply_{}", c))?;
}
for d in 0..dealers.len() {
    model.add_linear_constraint(&extract_coeffs(&receive[d]), ConstraintRelation::GreaterEqual,
        dealers[d].demand, &format!("demand_{}", d))?;
}
for d in 0..dealers.len() { for c in 0..centers.len() {
    let terms = vec![(x_idx[&[d, c]], 1.0), (y_idx[&[d, c]], -capacity)];
    model.add_linear_constraint(&terms, ConstraintRelation::LessEqual, 0.0,
        &format!("truck_{}_{}", d, c))?;
}}
let _output = solve_typed(model)?;
```

:::

## Source and verification

### Kotlin/Rust correspondence

- [Rust counterpart: `src/core/demo13.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo13.rs)

The Rust counterpart uses the same five-dealer/three-center data, array orientation, truck-capacity inequality, and distance-per-truck objective. Its `MetaModel`, variable-combination, and symbol-combination APIs are independent of the Kotlin API.

- [Current implementation: `Demo13.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo13.kt)
- [Core build-structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The current core build test checks model structure only; it does not assert a numeric allocation or objective value.
