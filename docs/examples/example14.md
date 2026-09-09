# Example 14: Multi-stage distribution

## 1. Overview

This bounded context models a directed transshipment network with production nodes, sales nodes, and intermediate distribution nodes. Production capacity, sales demand, and the source's `unitCost` arc map are:

| Node type | Nodes and capacity/demand |
| :---: | :--- |
| Production ($P$) | Guangzhou $600$; Dalian $400$ |
| Sales ($S$) | Nanjing $200$; Jinan $150$; Nanchang $350$; Qingdao $300$ |
| Transshipment ($T$) | Shanghai; Tianjin |

The directed arcs and unit costs are exactly those present in the source map. An absent `unitCost` entry is not an arc.

| Arc | Cost | Arc | Cost |
| :--- | ---: | :--- | ---: |
| Guangzhou → Shanghai | 2 | Guangzhou → Tianjin | 3 |
| Dalian → Qingdao | 4 | Dalian → Shanghai | 3 |
| Dalian → Tianjin | 1 | Shanghai → Nanjing | 2 |
| Shanghai → Jinan | 6 | Shanghai → Nanchang | 3 |
| Shanghai → Qingdao | 6 | Tianjin → Nanjing | 4 |
| Tianjin → Jinan | 4 | Tianjin → Nanchang | 6 |
| Tianjin → Qingdao | 5 |  |  |

### 1. Dependent Contexts

None. Node classes, arc data, and all capacities/costs are local to `Demo14`.

---

## 2. Concepts / Entities

### 1. Network node

A network node is one location in the distribution network. The Kotlin source represents the three roles with the sealed `Node` interface and `Product`, `Sale`, and `Distribution` classes.

**$Name_i$** : display name of node $i$.

**$Storage_i$** : production capacity of a production node $i\in P$; distribution and sales nodes have no production capacity field.

**$Demand_i$** : demand of a sales node $i\in S$; production and distribution nodes have no demand field.

### 2. Directed arc

A directed arc connects a source node to a destination node and carries an integer flow.

**$c_{ij}$** : unit transportation cost on arc $(i,j)$, as read from `unitCost` or `ArcData.unit_cost`.

**$x_{ij}$** : flow assigned to arc $(i,j)$; its decision-variable definition is given in Section 3.

---

## 3. Variables

### 1. Decision Variables

**$x_{ij}$** : integer goods flow from node $i$ to node $j$, a shipment quantity, domain $\mathbb{Z}_{\ge 0}$, for every $(i,j)\in N\times N$. Non-arc entries are fixed to zero; only arc entries are registered in the Kotlin model. Kotlin additionally applies the production-node range bound $x_{ij}\le Storage_i$ to each outgoing arc, while the Rust model represents the same production limit through its node constraint.

### 2. Auxiliary Variables

There are no solver-owned auxiliary variables. `Cost`, `Out`, and `In` are intermediate expressions.

---

## 4. Predicates

### 1. Node-role predicates

**ProductNode($i$)** : node $i$ is a production node and has a storage/capacity value.

**SaleNode($i$)** : node $i$ is a sales node and has a demand value.

**DistributionNode($i$)** : node $i$ is an intermediate transshipment node.

### 2. Arc predicate

**ArcExists($i,j$)** : the source `unitCost` map (or Rust `arcs` list) contains the directed arc $(i,j)$.

---

## 5. Sets

### 1. Network nodes

**$N$** : the universal set of eight nodes.

**$P=\{i\in N\mid ProductNode(i)\}$** : production nodes Guangzhou and Dalian, which supply goods.

**$S=\{i\in N\mid SaleNode(i)\}$** : sales nodes Nanjing, Jinan, Nanchang, and Qingdao, which receive demand.

**$T=\{i\in N\mid DistributionNode(i)\}$** : transshipment nodes Shanghai and Tianjin, which conserve flow.

### 2. Directed arcs

**$E$** : the directed arc set,

$$
E=\{(i,j)\in N\times N\mid ArcExists(i,j)\}.
$$

The model does not infer a reverse arc when $(i,j)\in E$; each direction must be listed explicitly.

### 3. Entity Pairs / Relations

**$E\subseteq N\times N$** : the directed transport relation. Non-edges are fixed to zero before model registration and therefore cannot carry flow.

---

## 6. Intermediate Values

### 1. Transportation cost

**Description**: `Cost` is the total unit cost of all flow on the defined directed arcs.

$$
Cost=\sum_{(i,j)\in E}c_{ij}x_{ij}.
$$

### 2. Outgoing flow

**Description**: `Out` is the total flow leaving a production or transshipment node. Because non-arc variables are fixed to zero, the source's all-destination sum is equivalent to the arc-restricted sum.

$$
Out_i=\sum_{j:(i,j)\in E}x_{ij},\qquad \forall i\in P\cup T.
$$

### 3. Incoming flow

**Description**: `In` is the total flow entering a sales or transshipment node. The source uses the incoming orientation $x_{ji}$; writing $x_{ij}$ here would count outgoing flow instead.

$$
In_i=\sum_{j:(j,i)\in E}x_{ji},\qquad \forall i\in S\cup T.
$$

---

## 7. Assertions

### 1. Node-role partition

**Description**: Every node belongs to exactly one of the three source node classes.

$$
N=P\mathbin{\dot\cup}S\mathbin{\dot\cup}T,
\qquad P\cap S=S\cap T=P\cap T=\emptyset.
$$

### 2. Balanced capacity and demand data

**Description**: Total production capacity equals total sales demand in this instance.

$$
\sum_{i\in P}Storage_i=600+400=1000
=200+150+350+300=\sum_{i\in S}Demand_i.
$$

### 3. Flow accounting identity

**Description**: Every arc flow is counted once at its source and once at its destination; transshipment balance therefore transfers, rather than creates or destroys, flow.

$$
\sum_{i\in N}Out_i-\sum_{i\in N}In_i=0
$$

where undefined `Out`/`In` terms at node roles are understood as zero and non-arc flows are zero.

---

## 8. Constraints

### 1. Production Capacity [生产能力上限]

**Description**: The flow dispatched by each production node cannot exceed its storage/capacity value.

$$
s.t.\quad Out_i\le Storage_i,\qquad \forall i\in P.
$$

### 2. Sales Demand Coverage [销售需求满足]

**Description**: Each sales node must receive at least its stated demand.

$$
s.t.\quad In_i\ge Demand_i,\qquad \forall i\in S.
$$

### 3. Transshipment Flow Balance [转运流量平衡]

**Description**: A transshipment node cannot accumulate or create goods; its incoming and outgoing flows must be equal. Kotlin registers the equality as two inequalities, while Rust registers one equality relation.

$$
s.t.\quad In_i=Out_i,\qquad \forall i\in T.
$$

**Corollary**: With the balanced totals in Section 7, the aggregate inequalities force all production capacity and all sales demand to be tight.

$$
\sum_{i\in P}Out_i=\sum_{i\in P}Storage_i=1000,
\qquad
\sum_{i\in S}In_i=\sum_{i\in S}Demand_i=1000.
$$

---

## 9. Objective Function

**Description**: Minimize total transportation cost over the listed directed arcs.

$$
\min Cost=\min\sum_{(i,j)\in E}c_{ij}x_{ij}.
$$

For the current data, a cost-$4600$ flow is:

| Arc | Flow |
| :--- | ---: |
| Guangzhou → Shanghai | 550 |
| Guangzhou → Tianjin | 50 |
| Dalian → Qingdao | 300 |
| Dalian → Tianjin | 100 |
| Shanghai → Nanjing | 200 |
| Shanghai → Nanchang | 350 |
| Tianjin → Jinan | 150 |

All other defined arcs carry zero flow. This is a documented feasible optimum for the listed arc costs, not a numeric assertion made by the Kotlin core build-structure test.

---

## 10. Algorithm References

No standalone algorithm document is referenced; the page describes the source-local transshipment model.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| None | — | — | Direct arc-flow expressions and node-balance constraints |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Production node | $P$ | Node that supplies goods and is bounded by `Storage`. |
| Sales node | $S$ | Node that requires incoming goods according to `Demand`. |
| Transshipment node | $T$ | Node where incoming and outgoing flows balance. |
| Directed arc | $E$ | A listed source-to-destination transport connection. |
| Arc flow | $x_{ij}$ | Integer goods sent along arc $(i,j)$. |
| Unit cost | $c_{ij}$ | Cost of one unit on arc $(i,j)$. |
| Total cost | $Cost$ | Sum of unit cost times arc flow. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use the explicit directed arc set $E$ | Treat every node pair as an arc | Matches `unitCost`/`ArcData` and prevents nonexistent direct shipments. | 2026-09-08 |
| Keep `In` as $\sum_jx_{ji}$ | Use $\sum_jx_{ij}$ for both directions | The incoming orientation is required for transshipment balance. | 2026-09-08 |
| Preserve inequality/equality implementation differences | Normalize both languages to one API form | Kotlin uses two inequalities for balance; Rust uses `Equal`, and both semantics are recorded. | 2026-09-08 |

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganized the example into the domain-model sections and documented Kotlin/Rust arc-flow implementations. | Align the page with the template while retaining the current data and the cost-$4600$ flow. |

---

## Minimal current model-building example

The snippets are model-building fragments. `nodes`, `unitCost`, `Node` subclasses, `arcs`, `NodeType`, and helper functions are supplied by the linked source files.

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

// nodes, unitCost, and flt64Converter come from Demo14.kt.
val model = LinearMetaModel<Flt64>("demo14", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
for (from in nodes) for (to in nodes) {
    if (unitCost[from]?.get(to) != null) {
        if (from is Product) x[from, to].range.leq(from.storage)
        model.add(x[from, to])
    } else {
        x[from, to].range.eq(UInt64.zero)
    }
}
val cost = LinearExpressionSymbol(
    sum(nodes.flatMap { from -> nodes.mapNotNull { to ->
        unitCost[from]?.get(to)?.let { it * x[from, to] }
    } }), name = "cost"
)
val out = LinearIntermediateSymbols1<Flt64>("out", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[nodes[i], _a]), name = "out_${nodes[i].name}")
}
val input = LinearIntermediateSymbols1<Flt64>("in", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, nodes[i]]), name = "in_${nodes[i].name}")
}
model.add(cost); model.add(out); model.add(input); model.minimize(cost)
for (node in nodes.filterIsInstance<Product>()) model.addConstraint(out[node] leq node.storage)
for (node in nodes.filterIsInstance<Sale>()) model.addConstraint(input[node] geq node.demand)
for (node in nodes.filterIsInstance<Distribution>()) {
    model.addConstraint(out[node] geq input[node])
    model.addConstraint(out[node] leq input[node])
}

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

```rust [Rust]
use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::flat_map1;
use ospf_rust_core::variable::{UInteger, VariableCombination2D, VariableRange};
use ospf_rust_multiarray::Shape;

// build_nodes/build_arcs, extract_coeffs, and solve_typed are from demo14.rs.
let nodes = build_nodes();
let arcs = build_arcs();
let mut model = MetaModel::<f64>::new("demo14");
let x_vars = VariableCombination2D::with_name_and_range_generator(
    Shape::new([nodes.len(), nodes.len()]), "x", |_i, v| format!("{}_{}", v[0], v[1]),
    |_i, v| if arcs.iter().any(|a| a.from == v[0] && a.to == v[1]) {
        VariableRange::with_lower(0.0)
    } else { VariableRange::fixed(0.0) }
);
let x_idx = model.register_combination(&x_vars)?;
let cost = flat_map1("cost", &arcs, |arc| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(
            arc.unit_cost, x_idx[&[arc.from, arc.to]])
    ], 0.0)
}, |_, arc| format!("{}_{}", arc.from, arc.to));
let ids: Vec<usize> = (0..nodes.len()).collect();
let out = flat_map1("trans_out", &ids, |&i| {
    let terms = (0..nodes.len()).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, i| i.to_string());
let input = flat_map1("trans_in", &ids, |&j| {
    let terms = (0..nodes.len()).map(|i|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, j| j.to_string());
model.add_symbol_combination(&cost)?;
model.add_symbol_combination(&out)?;
model.add_symbol_combination(&input)?;
let coeffs = (0..arcs.len()).flat_map(|i| cost.symbol_polynomial(i).monomials().iter()
    .map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>()).collect::<Vec<_>>();
model.add_linear_objective(&coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for (i, node) in nodes.iter().enumerate() {
    let out_coeffs = extract_coeffs(&out[i]);
    let in_coeffs = extract_coeffs(&input[i]);
    match node.kind {
        NodeType::Product(cap) => model.add_linear_constraint(
            &out_coeffs, ConstraintRelation::LessEqual, cap, &format!("product_{}", i))?,
        NodeType::Sale(demand) => model.add_linear_constraint(
            &in_coeffs, ConstraintRelation::GreaterEqual, demand, &format!("sale_{}", i))?,
        NodeType::Distribution => {
            let mut balance = out_coeffs;
            balance.extend(in_coeffs.into_iter().map(|(idx, c)| (idx, -c)));
            model.add_linear_constraint(&balance, ConstraintRelation::Equal, 0.0,
                &format!("balance_{}", i))?;
        }
    }
}
let _output = solve_typed(model)?;
```

:::

## Source and verification

### Kotlin/Rust correspondence

- [Rust counterpart: `src/core/demo14.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo14.rs)

Both implementations use the same eight nodes, thirteen directed arcs, costs, capacities, demands, and minimum-cost flow semantics. Their model, variable-combination, and symbol-combination APIs are independent. Kotlin registers each defined arc and applies a redundant per-production-arc range bound; Rust fixes non-arcs through `VariableRange` and applies production capacity at the node constraint.

- [Current implementation: `Demo14.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo14.kt)
- [Core build-structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The Kotlin core build test checks structure only and does not assert the numeric flow or the cost-$4600$ result.
