# Example 11: Maximum flow

## 1. Overview

This bounded context maximizes integral flow from root node 0 to sink node 8 in a directed capacity network. The Kotlin and Rust implementations use different registration details but encode the same edge set, balances, and objective.

The current network has these edge capacities:

| Edge | Capacity | Edge | Capacity |
| :---: | ---: | :---: | ---: |
| $0\to1$ | 15 | $0\to2$ | 10 |
| $0\to3$ | 40 | $1\to4$ | 15 |
| $2\to5$ | 10 | $2\to6$ | 35 |
| $3\to6$ | 30 | $3\to7$ | 20 |
| $4\to6$ | 10 | $5\to8$ | 10 |
| $6\to7$ | 10 | $7\to8$ | 45 |

### 1. Dependent Contexts

1. None. The network-flow model is self-contained.

---

## 2. Concepts / Entities

### 1. Node

A node is a network vertex. The sample has one root, seven normal nodes, and one sink.

**$r$**: root/source node, node 0.

**$t$**: end/sink node, node 8.

**$V^N$**: normal nodes, all nodes other than $r$ and $t$.

### 2. Directed Edge

An edge is a directed connection with a finite capacity.

**$c_{ij}$**: capacity of edge $(i,j)$, defined only for modeled edges.

---

## 3. Variables

### 1. Decision Variables

**$x_{ij}$**: integral flow on edge $(i,j)$, measured in flow units, domain $\mathbb{Z}_{\ge0}$, $\forall(i,j)\in E$. Kotlin registers only entries present in `capacities`; Rust registers the full matrix and fixes non-edges to zero ranges.

**$f$**: total source-to-sink flow, measured in flow units, domain $mathbb{Z}_{\ge0}$. Kotlin uses scalar `UIntVar("flow")`; Rust uses a one-element `VariableCombination1D<UInteger>`.

### 2. Auxiliary Variables

**$In_i$** and **$Out_i$** are registered linear intermediate symbols derived from edge flows. They are not independent decisions.

---

## 4. Predicates

### 1. Network Membership Predicates

**`IsEdge(i,j)`**: true when $(i,j)\in E$ and the capacity map has an entry.

**`IsNormal(i)`**: true when $i\in V\setminus\{r,t\}$.

---

## 5. Sets

### 1. Node and Edge Categories

**$V$**: universal node set, $V=\{0,1,\ldots,8\}$.

**$E$**: exactly the twelve directed edges listed in the data table.

**$V^N$**: normal-node subset, $V^N=V\setminus\{r,t\}$.

### 2. Entity Pairs / Relations

**$NetworkEdge$**: relation $E$ connecting a tail node to a head node.

Non-edges $(i,j)\notin E$ have no business flow variable in Kotlin and a fixed-zero matrix item in Rust.

---

## 6. Intermediate Values

### 1. Inflow at a Node

**Description**: Total flow entering node $i$ along modeled incoming edges.

$$
In_i=\sum_{j:(j,i)\in E}x_{ji},\qquad \forall i\in V.
$$

### 2. Outflow at a Node

**Description**: Total flow leaving node $i$ along modeled outgoing edges.

$$
Out_i=\sum_{j:(i,j)\in E}x_{ij},\qquad \forall i\in V.
$$

### 3. Balance Residual

**Description**: The difference between outflow and inflow; it equals $f$ at the root, $-f$ at the sink, and zero at normal nodes.

$$
Balance_i=Out_i-In_i=
\begin{cases}
f,&i=r,\\
-f,&i=t,\\
0,&i\in V^N.
\end{cases}
$$

---

## 7. Assertions

### 1. Capacity Dominates Edge Flow

**Description**: No modeled edge carries more flow than its capacity.

$$
\forall(i,j)\in E\;(0\le x_{ij}\le c_{ij}).
$$

### 2. Normal-Node Conservation

**Description**: A normal node neither creates nor consumes net flow.

$$
\forall i\in V^N\;(Out_i=In_i).
$$

### 3. Root/Sink Flow Agreement

**Description**: The same non-negative scalar $f$ leaves the root and enters the sink.

$$
Out_r-In_r=f\wedge In_t-Out_t=f\wedge f\ge0.
$$

### 4. Sample Maximum

**Description**: For the supplied network the current model has maximum flow $f=40$. One feasible witness sends 10 on $0\to1\to4\to6\to7\to8$, 10 on $0\to2\to5\to8$, and 20 on $0\to3\to7\to8$; the remaining edge flows are zero. The incoming capacity to node 7 limits the usable $7\to8$ flow to 30, so the witness is optimal.

$$
f^{*}=40.
$$

---

## 8. Constraints

### 1. Edge Capacity (边容量约束)

**Description**: Each modeled edge flow is non-negative and cannot exceed the capacity. Kotlin imposes the upper bound while registering each edge item; Rust encodes it in the range generator.

$$
s.t. \quad 0\le x_{ij}\le c_{ij},\qquad \forall(i,j)\in E.
$$

### 2. Root Balance (源点流量平衡约束)

**Description**: Net flow leaving the root equals the total flow variable.

$$
s.t. \quad Out_r-In_r=f.
$$

### 3. Sink Balance (汇点流量平衡约束)

**Description**: Net flow entering the sink equals the same total flow variable.

$$
s.t. \quad In_t-Out_t=f.
$$

### 4. Normal-Node Conservation (中间节点守恒约束)

**Description**: Every normal node has equal inflow and outflow. Kotlin adds both inequalities; Rust adds one `Equal` relation after combining the two expressions.

$$
s.t. \quad Out_i=In_i,\qquad \forall i\in V^N.
$$

Implementation forms:

$$
s.t. \quad Out_i\ge In_i\ \wedge\ Out_i\le In_i,qquad \forall i\in V^N\quad\text{(Kotlin)},
$$

or one equality in Rust.

### 5. Flow Domain (总流量定义域约束)

**Description**: Total flow is an integral non-negative quantity.

$$
s.t. \quad f\in\mathbb{Z}_{\ge0}.
$$

---

## 9. Objective Function (if applicable)

**Description**: Maximize the total flow delivered from root to sink.

$$
\max f.
$$

---

## 10. Algorithm References

No standalone algorithm document is referenced. This is the standard capacitated maximum-flow linear model.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| None | — | — | The page documents the optimization formulation directly. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Root/source | $r$ | Node 0 where flow originates. |
| Sink/end | $t$ | Node 8 where flow terminates. |
| Edge flow | $x_{ij}$ | Integral flow on a modeled directed edge. |
| Capacity | $c_{ij}$ | Upper bound on an edge flow. |
| Inflow | $In_i$ | Sum of incoming edge flows. |
| Outflow | $Out_i$ | Sum of outgoing edge flows. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Keep only the twelve listed edges as business edges | Add variables for every node pair | The source capacity map defines the graph; absent entries are not edges | 2026-09-08 |
| Express normal conservation as equality | Keep only one inequality direction | Kotlin's two inequalities and Rust's combined equality have the same semantics | 2026-09-08 |
| Use an explicit total flow variable | Maximize root outflow directly | Both current sources expose `flow` as a model variable | 2026-09-08 |

### Minimal current model-building snippets

The node and capacity data come from `Demo11.kt`/`demo11.rs`. The tabs include variables, inflow/outflow expressions, objective, and all user-level balance constraints; range registration carries edge capacities.

::: code-group
```kotlin [Kotlin]
// `nodes`, `capacities`, `rootNode`, `endNode`, and `flt64Converter` come from Demo11.kt.
val metaModel = LinearMetaModel<Flt64>("demo11", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
for (from in nodes) for (to in nodes) {
    capacities[from]?.get(to)?.let { capacity ->
        x[from, to].range.leq(capacity)
        metaModel.add(x[from, to])
    }
}
val flow = UIntVar("flow")
metaModel.add(flow)
val flowIn = LinearIntermediateSymbols1<Flt64>("flow_in", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, i]), name = "flow_in_$i")
}
val flowOut = LinearIntermediateSymbols1<Flt64>("flow_out", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[i, _a]), name = "flow_out_$i")
}
metaModel.add(flowIn)
metaModel.add(flowOut)
metaModel.maximize(flow, "flow")
metaModel.addConstraint(flowOut[rootNode] - flowIn[rootNode] eq flow)
metaModel.addConstraint(flowIn[endNode] - flowOut[endNode] eq flow)
for (node in nodes.filterIsInstance<NormalNode>()) {
    metaModel.addConstraint(flowOut[node] geq flowIn[node])
    metaModel.addConstraint(flowOut[node] leq flowIn[node])
}
```

```rust [Rust]
// `data` comes from MaxFlowData::sample() in demo11.rs.
let node_count = data.nodes.len();
let mut model = MetaModel::<f64>::new("demo11");
let arc_vars = VariableCombination2D::<UInteger>::with_name_and_range_generator(
    Shape::new([node_count, node_count]), "x",
    |_index, vector| format!("{}_{}", vector[0], vector[1]),
    |_index, vector| data.capacities.iter()
        .find(|arc| arc.from == vector[0] && arc.to == vector[1])
        .map(|arc| VariableRange::bounded(0.0, arc.capacity))
        .unwrap_or_else(|| VariableRange::fixed(0.0)),
);
let arc_idx = model.register_combination(&arc_vars)?;
let flow_vars = VariableCombination1D::<UInteger>::new(Shape::new([1]), "flow");
let flow_idx = model.register_combination(&flow_vars)?;
model.add_linear_objective(&[(flow_idx[0], 1.0)], "flow");
model.set_objective_category(ObjectiveCategory::Maximum);
let flow_out = flat_map1_indexed("flow_out", &data.nodes, |node, _| {
    let monomials = (0..node_count).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, arc_idx[&[node, j]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |i, _| format!("{}", i));
let flow_in = flat_map1_indexed("flow_in", &data.nodes, |node, _| {
    let monomials = (0..node_count).map(|i|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, arc_idx[&[i, node]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |i, _| format!("{}", i));
model.add_symbol_combination(&flow_out)?;
model.add_symbol_combination(&flow_in)?;
// add_constraints() combines `flow_out - flow_in`, adds `-flow` at root,
// `+flow` at sink, and an Equal 0 balance for each normal node.
```
:::

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 2026-09-08 | Reorganized the page into the domain-model template; added quantified intermediate values, bilingual constraint names, and equivalent Kotlin/Rust tabs | Match the current Demo11 implementations and preserve their conservation-form difference |

## Source and verification

- [Kotlin implementation: `Demo11.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo11.kt)
- [Rust counterpart: `demo11.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo11.rs)
- [Kotlin structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
