# Example 14: Transshipment Problem

## Problem Description

There are two production sites transporting goods to four dealers. Transportation can be directly from production sites to dealers, or from production sites to transshipment centers, then from transshipment centers to various dealers in batches. The unit transportation costs between locations are shown in the following table:

| Location | Shanghai | Tianjin | Nanjing | Jinan | Nanchang | Qingdao |
| :------: | :------: | :-----: | :-----: | :---: | :------: | :-----: |
| Guangzhou |  $2$  |  $3$  |  --   |  --   |  --   |  --   |
| Dalian |  $3$  |  $1$  |  --   |  --   |  --   |  $4$  |
| Shanghai |  --   |  --   |  $2$  |  $6$  |  $3$  |  $6$  |
| Tianjin |  --   |  --   |  $4$  |  $4$  |  $6$  |  $5$  |

Production site storage capacity:

| Production Site | Guangzhou | Dalian |
| :-------------: | :-------: | :----: |
| Storage Capacity |   $600$   | $400$  |

Sales location demand:

| Sales Location | Nanjing | Jinan | Nanchang | Qingdao |
| :------------: | :-----: | :---: | :------: | :-----: |
| Demand | $200$ | $150$ |  $350$  | $300$  |

Determine the transportation plan with minimum total transportation cost, while satisfying the following conditions:

1. Total transportation volume does not exceed the total storage capacity of production sites;
2. Transportation materials to each sales location meet or exceed demand;
3. Inflow and outflow at transshipment centers remain balanced.

## Mathematical Model

### Variables

$x_{ij}$: Goods transportation volume from point $i$ to point $j$.

### Intermediate Values

#### 1. Total Transportation Cost

$$
\text{Cost} = \sum_{i \in (P \cup T)} \sum_{j \in (T \cup S)} c_{ij} \cdot x_{ij}
$$

#### 2. Total Outflow from Each Point

$$
\text{Out}_{i} = \sum_{j \in (T \cup S)} x_{ij}, \; \forall i \in ( P \cup T )
$$

#### 3. Total Inflow to Each Point

$$
\text{In}_{i} = \sum_{j \in (P \cup T)} x_{ij}, \; \forall i \in ( S \cup T )
$$

### Objective Function

#### 1. Minimize Total Transportation Cost

$$
\min \quad \text{Cost}
$$

### Constraints

#### 1. Total Transportation Volume Does Not Exceed Production Site Storage Capacity

$$
\text{s.t.} \quad \text{Out}_{i} \leq \text{Store}_{i}, \; \forall i \in P
$$

#### 2. Sales Location Inflow Meets Demand

$$
\text{s.t.} \quad \text{In}_{i} \geq \text{Require}_{i}, \; \forall i \in S
$$

#### 3. Transshipment Center Inflow-Outflow Balance

$$
\text{s.t.} \quad \text{In}_{i} = \text{Out}_{i}, \; \forall i \in T
$$

## Expected Results

| Location | Shanghai | Tianjin | Nanjing | Jinan | Nanchang | Qingdao |
| :------: | :------: | :-----: | :-----: | :---: | :------: | :-----: |
| Guangzhou |  $2$  |  $3$  |  $-$  |  $-$  |  $-$  |  $-$  |
| Dalian |  $3$  |  $1$  |  $-$  |  $-$  |  $-$  |  $4$  |
| Shanghai |  $-$  |  $-$  |  $2$  |  $6$  |  $3$  |  $6$  |
| Tianjin |  $-$  |  $-$  |  $4$  |  $4$  |  $6$  |  $5$  |

## Code Implementation

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

sealed interface Node : Indexed {
    val name: String
}

class Product(
    override val name: String,
    val storage: UInt64
) : Node, AutoIndexed(Node::class)

class Sale(
    override val name: String,
    val demand: UInt64
) : Node, AutoIndexed(Node::class)

class Distribution(
    override val name: String
) : Node, AutoIndexed(Node::class)

val nodes: List<Node> = ... // Node list
val unitCost: Map<Node, Map<Node, UInt64>> = ... // Unit transportation cost

// Create model instance
val metaModel = LinearMetaModel("demo14")

// Define variables
val x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
for (node1 in nodes) {
    for (node2 in nodes) {
        val thisUnitCost = unitCost[node1]?.get(node2)
        if (thisUnitCost != null) {
            if (node1 is Product) {
                x[node1, node2].range.leq(node1.storage)
            }
            metaModel.add(x[node1, node2])
        } else {
            x[node1, node2].range.eq(UInt64.zero)
        }
    }
}

// Define intermediate values
val cost = LinearExpressionSymbol(
    sum(nodes.flatMap { node1 ->
        nodes.mapNotNull { node2 ->
            unitCost[node1]?.get(node2)?.let {
                it * x[node1, node2]
            }
        }
    }), "cost"
)
metaModel.add(cost)

val transOut = LinearIntermediateSymbols1("out", Shape1(nodes.size)) { i, _ ->
    val node = nodes[i]
    LinearExpressionSymbol(sum(x[node, _a]), "out_${node.name}")
}
metaModel.add(transOut)

val transIn = LinearIntermediateSymbols1("in", Shape1(nodes.size)) { i, _ ->
    val node = nodes[i]
    LinearExpressionSymbol(sum(x[_a, node]), "in_${node.name}")
}
metaModel.add(transIn)

// Define objective function
metaModel.minimize(cost)

// Define constraints
for (node in nodes.filterIsInstance<Product>()) {
    metaModel.addConstraint(
        transOut[node] leq node.storage,
        "out_${node.name}"
    )
}

for (node in nodes.filterIsInstance<Sale>()) {
    metaModel.addConstraint(
        transIn[node] geq node.demand,
        "in_${node.name}"
    )
}

for (node in nodes.filterIsInstance<Distribution>()) {
    metaModel.addConstraint(
        transOut[node] eq transIn[node],
        "balance_${node.name}"
    )
}

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution: MutableMap<Node, MutableMap<Node, UInt64>> = hashMapOf()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val from = nodes[vector[0]]
        val to = nodes[vector[1]]
        solution.getOrPut(from) { hashMapOf() }[to] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo14.kt)
