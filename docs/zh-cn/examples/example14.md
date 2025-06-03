# 示例 14：转运问题

## 问题描述

设有两个产地向四个经销商运输货物，运输可以是产地直接到经销商的运输，也可以是产地向转运中心运输货物，再由转运中心分批运至各经销商，各地之间的单位运价如表所示：

| 地点  | 上海  | 天津  | 南京  | 济南  | 南昌  | 青岛  |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| 广州  |   2   |   3   |  --   |  --   |  --   |  --   |
| 大连  |   3   |   1   |  --   |  --   |  --   |   4   |
| 上海  |  --   |  --   |   2   |   6   |   3   |   6   |
| 天津  |  --   |  --   |   4   |   4   |   6   |   5   |

产地存储货量：

|  产地  | 广州  | 大连  |
| :----: | :---: | :---: |
| 储存量 |  600  |  400  |

销售地需求量：

|  销地  | 南京  | 济南  | 南昌  | 青岛  |
| :----: | :---: | :---: | :---: | :---: |
| 需求量 |  200  |  150  |  350  |  300  |

求总运费最小的运输方案:

1) 运输量总量不超过产地的总存储量;
2) 各销地的运输物资不低于需求量；
3) 转运中心进出保持平衡

## 数学模型

### 变量

$x_{ij}$：点 $i$ 向点 $j$ 运输货量。

### 中间值

##### 1. 总运输成本

$$
Cost = \sum_{i \in (P \cup T)} \sum_{j \in (T \cup S)} c_{ij} \cdot x_{ij}
$$

##### 2. 各点运输总量

$$
Out_{i} = \sum_{j \in (T \cup S)} x_{ij}, \; \forall i \in ( P \cup T )
$$

##### 3. 各点接收总量

$$
In_{i} = \sum_{j \in (P \cup T)} x_{ij}, \; \forall i \in ( S \cup T )
$$

### 目标函数

#### 1. 总运费最小

$$
min \quad Cost
$$

### 约束

#### 1. 运输总量不超过产地总存储量

$$
s.t. \quad Qut_{i} \leq Store_{i}, \; \forall i \in P
$$

#### 2. 销地接收量满足需求

$$
s.t. \quad In_{i} \geq Require_{i}, \; \forall i \in S
$$

#### 3. 转运中心流入流出平衡

$$
s.t. \quad In_{i} = Out_{i}, \; \forall i \in T
$$

## 期望结果

| 地点  | 上海  | 天津  | 南京  | 济南  | 南昌  | 青岛  |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| 广州  |   2   |   3   |  --   |  --   |  --   |  --   |
| 大连  |   3   |   1   |  --   |  --   |  --   |   4   |
| 上海  |  --   |  --   |   2   |   6   |   3   |   6   |
| 天津  |  --   |  --   |   4   |   4   |   6   |   5   |

## 代码实现

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

val nodes: List<Node> = ... // 节点列表
val unitCost: Map<Node, Map<Node, UInt64>> = ... // 单位运价

// 创建模型实例
val metaModel = LinearMetaModel("demo14")

// 定义变量
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

// 定义中间值
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

// 定义目标函数
metaModel.minimize(cost)

// 定义约束
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

// 调用求解器求解
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// 解析结果
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

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo13.kt)
