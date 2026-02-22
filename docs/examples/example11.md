# Example 11: Maximum Flow Problem

## Problem Description

Given a directed connected graph $G=(V,E)$, where $V$ is the set of nodes in the graph, $E$ is the set of arcs in the graph, with $r$ as the source node and $e$ as the sink node. The weight of each arc represents the maximum flow that can pass through that arc:

|       |   0   |   1   |   2   |   3   |   4   |   5   |   6   |   7   |   8   |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
|   0   |   -   | $15$  | $10$  | $40$  |   -   |   -   |   -   |   -   |   -   |
|   1   |   -   |   -   |   -   |   -   | $15$  |   -   |   -   |   -   |   -   |
|   2   |   -   |   -   |   -   |   -   |   -   | $10$  | $35$  |   -   |   -   |
|   3   |   -   |   -   |   -   |   -   |   -   |   -   | $30$  | $20$  |   -   |
|   4   |   -   |   -   |   -   |   -   |   -   |   -   | $10$  |   -   |   -   |
|   5   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   | $10$  |
|   6   |   -   |   -   |   -   |   -   |   -   |   -   |   -   | $10$  |   -   |
|   7   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   | $45$  |
|   8   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |

## Mathematical Model

### Variables

$x_{ij} \in \mathbb{Z}^{+}$: Flow units circulating from node $i$ to $j$.

$f \in \mathbb{Z}^{+}$: Flow.

### Intermediate Values

#### 1. Total Inflow

$$
\text{In}_{i} = \sum_{j \in V} x_{ji}, \; \forall i \in V
$$

#### 2. Total Outflow

$$
\text{Out}_{i} = \sum_{j \in V} x_{ij}, \; \forall i \in V
$$

### Objective Function

#### 1. Maximize Flow Throughput

$$
\max \quad f
$$

### Constraints

#### 1. Flow Out from Source Node $r$

$$
\text{Out}_{r} - \text{In}_{r} = f
$$

#### 2. Flow In to Sink Node $e$

$$
\text{In}_{e} - \text{Out}_{e} = f
$$

#### 3. Flow Balance Constraint for Non-Source/Sink Nodes

$$
\text{Out}_{i} - \text{In}_{i} = 0, \; \forall i \in (V - \{r, e\})
$$

#### 4. Arc Capacity Constraint

$$
x_{ij} \leq c_{ij}, \; \forall i \in V, \; \forall j \in V
$$

## Expected Results

|       |   0   |   1   |   2   |   3   |   4   |   5   |   6   |   7   |   8   |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
|   0   |   -   | $10$  | $10$  | $30$  |   -   |   -   |   -   |   -   |   -   |
|   1   |   -   |   -   |   -   |   -   | $10$  |   -   |   -   |   -   |   -   |
|   2   |   -   |   -   |   -   |   -   |   -   | $10$  |  $0$  |   -   |   -   |
|   3   |   -   |   -   |   -   |   -   |   -   |   -   |  $0$  | $20$  |   -   |
|   4   |   -   |   -   |   -   |   -   |   -   |   -   | $10$  |   -   |   -   |
|   5   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   | $10$  |
|   6   |   -   |   -   |   -   |   -   |   -   |   -   |   -   | $10$  |   -   |
|   7   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   | $30$  |
|   8   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |   -   |

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

sealed class Node : AutoIndexed(Node::class)
class RootNode : Node()
class EndNode : Node()
class NormalNode : Node()

val nodes: List<Node> = ... // Node list
val capacities: Map<Node, Map<Node, Flt64>> = ... // Capacity matrix

// Create model instance
val metaModel = LinearMetaModel("demo11")

// Create variables
val x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
for (node1 in nodes) {
    for (node2 in nodes) {
        if (node1 == node2) {
            continue
        }
        capacities[node1]?.get(node2)?.let {
            val xi = x[node1, node2]
            xi.range.leq(it)
            metaModel.add(xi)
        }
    }
}

val flow = UIntVar("flow")
metaModel.add(flow)

// Define intermediate values
val flowIn = LinearIntermediateSymbols1("flow_in", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, i]), "flow_in_$i")
}
metaModel.add(flowIn)

val flowOut = LinearIntermediateSymbols1("flow_out", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[i, _a]), "flow_out_$i")
}
metaModel.add(flowOut)

// Define objective function
metaModel.maximize(flow, "flow")

// Define constraints
val rootNode = nodes.first { it is RootNode }
metaModel.addConstraint(
    flowOut[rootNode] - flowIn[rootNode] eq flow,
    "flow_${rootNode.index}"
)

val endNode = nodes.first { it is EndNode }
metaModel.addConstraint(
    flowIn[endNode] - flowOut[endNode] eq flow,
    "flow_${endNode.index}"
)

 for (node in nodes.filterIsInstance<NormalNode>()) {
    metaModel.addConstraint(
        flowOut[node] eq flowIn[node],
        "flow_${node.index}"
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
val solution: MutableMap<Node, MutableMap<Node, UInt64>> = HashMap()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val node1 = nodes[vector[0]]
        val node2 = nodes[vector[1]]
        solution.getOrPut(node1) { mutableMapOf() }[node2] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo11.kt)
