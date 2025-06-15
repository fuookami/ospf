# 示例 17：车辆路径问题

## 问题描述

车辆路径问题（ vehicle routing problem， $VRP$ ）是比较经典的运筹学优化问题，在离散组合优化中研究较多，并在物流行业有着很强的应用价值，通过优化车辆行驶路径，能有效节省物流配送成本。车辆路线问题最早是由 $Dantzig$ 和 $Ramser$ 于1959年首次提出，它是指一定数量有一定数量（ $n$ 个）的客户，各自有不同数量的货物需求（ $q_i$ ），配送中心或车场（ $depot$ ）向客户提供货物，由一个车队（ $K$ 辆车）负责分送货物，组织适当的行车路线，目标是使得客户的需求得到满足，并能在一定的约束下（例如车辆存在载重上限 $Q_k$ 、里程长度上限 $L$ ），达到总旅行成本最小、耗费时间最少等目的。

对于 $VRPTW$ 问题来说，首先给出基本定义，在一个 $G(V, A)$ 图，其中 $V = \{ 0, 1, 2, \dots, n + 1 \}$ 为图中所有节点的集合，为了方便建模，虚拟其配送中心为 $0$ 和 $n + 1$ ，表示为起点和终点。 $A$ 为图中所有弧的集合， $(i, j) \in A$ ，且 $\forall i, j \in V, \; i \neq j$ 。弧 $(i, j)$ 的单位运输费用为 $c_{ij}$ ，单位运输时间为 $t_{ij}$ ，节点 $i$ 的物资需求为 $q_{i}$ ，且节点 $i$ 的服务时间窗为 $[e_{i}, l_{i}]$ ，节点 $i$ 的需要的服务时长为 $h_i$ 。在配送中心（ $depot$ ）的车辆集合为 $K$ ，每辆车 $k \in K$ 的载重量为 $Q_k$ ，同时我们规定每辆车的固定使用成本为 $F_k$ 。

## 数学模型

### 集合

$V$ ：节点集合。

$O$ ：起点的集合。

$E$ ：终点的集合。

$K$ ：配送中心（ $depot$ ）的车辆集合。

### 参数

$c_{ij}$ ：弧 $(i, j)$ 的运输费用。

$t_{ij}$ ：弧 $(i, j)$ 运输时间，与 $c_{ij}$ 正相关。

$h_{i}$ ：节点 $i$ 需要的服务时长。

$q_{i}$ ：节点 $i$ 的物资需求数量。

$[e_{i}, \; l_{i}]$ ：节点 $i$ 的服务时间窗。

$Q_{k}$ ：每辆车 $k \in K$ 的载重量。

$F_{k}$ ：每辆车的固定使用成本。

### 变量

$x_{ijk}$ ：车辆 $k$ 是否经过弧 $(i, j)$ ，即代表车辆 $k$ 是否服务节点 $i$ 和节点 $j$ 。

$s_{ik}$ ：车辆 $k$ 在节点 $i$ 的开始服务时间。

### 中间值

中间值

#### 1. 从配送中心出发（隐含车辆是否使用）

$$
Origin_{k} = \sum_{i \in O} \sum_{j \in V} x_{ijk}, \; \forall k \in K
$$

#### 2. 返回配送中心

$$
Destination_{k} = \sum_{i \in V} \sum_{j \in E} x_{ijk}, \; \forall k \in K
$$

#### 3. 车辆进入节点的流量

$$
In_{jk} = \sum_{i \in V - E}  x_{ijk}, \; \forall j \in (V - (O \cup E)), \; \forall k \in K
$$

#### 4. 车辆流出节点的流量

$$
Out_{ik} = \sum_{j \in V - O} x_{ijk}, \; \forall i \in (V - (O \cup E)), \; \forall k \in K
$$

#### 5. 节点得到的服务

$$
Service_{i} = \sum_{k \in K} \sum_{j \in V - O} x_{ijk}, \; \forall i \in (V - (O \cup E))
$$

#### 6. 车辆载重量限制

$$
Capcity_{k} = \sum_{i \in V} \sum_{j \in V} q_{j} \cdot x_{ijk}, \; \forall k \in K
$$

### 目标

#### 1. 车辆使用成本最少

$$
min \quad \sum_{k \in K} Origin_{k} \cdot F_k
$$

#### 2. 运输成本最低

$$
min \quad \sum_{k \in K} \sum_{i \in V} \sum_{j \in V} c_{ij} \cdot x_{ijk}
$$

### 约束

#### 1. 表示车辆从配送中心出发只能到达一个节点（车辆必须从配送中心出发或者不使用）

$$
s.t. \quad Origin_{k} \leq 1, \; \forall k \in K
$$

#### 2. 表示流量平衡约束，即车辆到达一个节点，则必须从该节点出来

$$
s.t. \quad In_{ik} - Out_{ik} = 0, \; \forall k \in K, \; \forall i \in (V - (O \cup E))
$$

#### 3. 表示如果车辆被使用，则必须回到配送中心

$$
s.t. \quad Destination_{k} \leq 1, \; \forall k \in K
$$

#### 4. 表示每个节点必须得到服务

$$
s.t. \quad Service_{i} = 1, \; \forall i \in (V - (O \cup E))
$$

#### 5. 表示每两个节点访问之间的到达时间关系（该约束同时起到了去除回路的作用，消除了单车辆行驶中生成的不可行回路）

$$
s.t. \quad s_{ik} + h_{i} + t_{ij} - M(1 - x_{ijk}) \leq s_{jk}, \; \forall k \in K, \; \forall i \in V, \; \forall j \in V
$$

#### 6. 表示节点接收服务的时间窗约束

$$
s.t. \quad e_{i} \leq s_{ik} \leq l_{i}, \; \forall i \in V
$$

#### 7. 表示不能违反的车辆载重约束

$$
s.t. \quad Capcity_{k} \leq Q_{k}, \; \forall k \in K
$$

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
    val demand: UInt64 get() = UInt64.zero
    val position: Point2
    val timeWindow: ValueRange<UInt64>

    fun distance(other: Node): Flt64 {
        return position.distance(other.position)
    }

    fun cost(other: Node): Flt64 {
        return distance(other)
    }

    fun time(other: Node): Flt64 {
        return distance(other)
    }
}

data class OriginNode(
    override val position: Point2,
    override val timeWindow: ValueRange<UInt64>
) : Node, AutoIndexed(Node::class)

data class EndNode(
    override val position: Point2,
    override val timeWindow: ValueRange<UInt64>
) : Node, AutoIndexed(Node::class)

data class DemandNode(
    override val position: Point2,
    override val timeWindow: ValueRange<UInt64>,
    override val demand: UInt64,
    val serviceTime: UInt64
) : Node, AutoIndexed(Node::class)

data class Vehicle(
    val capacity: UInt64,
    val fixedUsedCost: UInt64,
) : AutoIndexed(Vehicle::class)

val nodes: List<Node> = ... // 节点列表
val vehicles: List<Vehicle> = ... // 车辆列表

// 创建模型实例
val metaModel = LinearMetaModel("demo17")

// 定义变量
val x = BinVariable3(
    "x",
    Shape3(nodes.size, nodes.size, vehicles.size)
)
for (n1 in nodes) {
    for (n2 in nodes) {
        for (v in vehicles) {
            val xi = x[n1, n2, v]
            if (n1 !is EndNode && n2 !is OriginNode && n1 != n2) {
                xi.name = "${x.name}_(${n1.index},${n2.index},${v.index})"
                metaModel.add(xi)
            } else {
                xi.range.eq(false)
            }
        }
    }
}

val s = URealVariable2(
    "s",
    Shape2(nodes.size, vehicles.size)
)
for (n in nodes) {
    for (v in vehicles) {
        val si = s[n, v]
        si.name = "${s.name}_(${n.index}_${v.index})"
    }
}
metaModel.add(s)

// 定义中间值
val origin = LinearIntermediateSymbols1(
    "origin",
    Shape1(vehicles.size)
) { i, _ ->
    val v = vehicles[i]
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<OriginNode>().flatMap { n1 -> x[n1, _a, v] }), 
        "origin_${v.index}"
    )
}
metaModel.add(origin)

val destination = LinearIntermediateSymbols1(
    "destination",
    Shape1(vehicles.size)
) { i, _ ->
    val v = vehicles[i]
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<EndNode>().flatMap { n2 -> x[_a, n2, v] }), 
        "destination_${v.index}"
    )
}
metaModel.add(destination)

val inFlow = LinearIntermediateSymbols2(
    "in",
    Shape2(nodes.size, vehicles.size)
) { _, vec ->
    val n2 = nodes[vec[0]]
    val v = vehicles[vec[1]]
    if (n2 is OriginNode) {
        LinearExpressionSymbol(
            LinearPolynomial(), 
            "in_(${n2.index},${v.index})"
        )
    } else {
        LinearExpressionSymbol(
            sum(nodes.filterIsNotInstance<EndNode, Node>().map { n1 -> x[n1, n2, v] }), 
            "in_(${n2.index},${v.index})"
        )
    }
}
metaModel.add(inFlow)

val outFlow = LinearIntermediateSymbols2(
    "out",
    Shape2(nodes.size, vehicles.size)
) { _, vec ->
    val n1 = nodes[vec[0]]
    val v = vehicles[vec[1]]
    if (n1 is EndNode) {
        LinearExpressionSymbol(
            LinearPolynomial(), 
            "out_(${n1.index},${v.index})"
        )
    } else {
        LinearExpressionSymbol(
            sum(nodes.filterIsNotInstance<OriginNode, Node>().map { n2 -> x[n1, n2, v] }), 
            "out_(${n1.index},${v.index})"
        )
    }
}
metaModel.add(outFlow)

val service = LinearIntermediateSymbols1(
    "service",
    Shape1(nodes.size)
) { i, _ ->
    val n1 = nodes[i]
    if (n1 is OriginNode || n1 is EndNode) {
        LinearExpressionSymbol(
            LinearPolynomial(), 
            "service_(${n1.index})"
        )
    } else {
        LinearExpressionSymbol(
            sum(nodes.filterIsNotInstance<OriginNode, Node>().flatMap { n2 -> x[n1, n2, _a] }), 
            "service_(${n1.index})"
        )
    }
}
metaModel.add(service)

val capacity = LinearIntermediateSymbols1(
    "capacity",
    Shape1(vehicles.size)
) { i, _ ->
    val v = vehicles[i]
    LinearExpressionSymbol(
        sum(nodes.flatMap { n1 ->
            nodes.mapNotNull { n2 ->
                (n2 as? DemandNode)?.demand?.let { it * x[n1, n2, v] }
            }
        }),
        "capacity_${v.index}"
    )
}
metaModel.add(capacity)

// 定义目标函数
metaModel.minimize(
    sum(vehicles.map { v -> v.fixedUsedCost * origin[v] }),
    "used cost"
)

metaModel.minimize(
    sum(nodes.flatMap { n1 ->
        nodes.map { n2 ->
            n1.cost(n2) * sum(x[n1, n2, _a])
        }
    }),
    "trans cost"
)

// 定义约束
for (v in vehicles) {
    metaModel.addConstraint(
        origin[v] leq 1,
        "origin_${v.index}"
    )
}

for (n in nodes.filterIsInstance<DemandNode>()) {
    for (v in vehicles) {
        metaModel.addConstraint(
            inFlow[n, v] eq outFlow[n, v],
            "balance_${n.index}_${v.index}",
        )
    }
}

for (v in vehicles) {
    metaModel.addConstraint(
        destination[v] leq 1,
        "destination_${v.index}"
    )
}

for (n in nodes.filterIsInstance<DemandNode>()) {
    metaModel.addConstraint(
        service[n] eq 1,
        "service_${n.index}"
    )
}

val m = nodes.filterIsInstance<EndNode>().maxOf { it.timeWindow.upperBound.value.unwrap() }
for (n1 in nodes) {
    for (n2 in nodes) {
        for (v in vehicles) {
            metaModel.addConstraint(
                s[n1, v] + ((n1 as? DemandNode)?.serviceTime ?: UInt64.zero) + n1.time(n2) - m * (1 - x[n1, n2, v]) leq s[n2, v],
                "time_window_${n1.index}_${n2.index}_${v.index}"
            )
        }
    }
}

for (n in nodes) {
    for (v in vehicles) {
        metaModel.addConstraint(
            s[n, v] geq n.timeWindow.lowerBound.value.unwrap(),
            "time_window_lb_${n.index}_${v.index}"
        )
        metaModel.addConstraint(
            s[n, v] leq n.timeWindow.upperBound.value.unwrap(),
            "time_window_ub_${n.index}_${v.index}"
        )
    }
}

for (v in vehicles) {
    metaModel.addConstraint(
        capacity[v] leq v.capacity,
        "capacity_${v.index}"
    )
}

// 调用求解器求解
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {
        return Failed(ret.error)
    }
}

// 解析结果
val route: MutableMap<Vehicle, MutableList<Pair<Node, Node>>> = HashMap()
val time: MutableMap<Vehicle, MutableMap<Node, UInt64>> = HashMap()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val n1 = nodes[vector[0]]
        val n2 = nodes[vector[1]]
        val v = vehicles[vector[2]]
        route.getOrPut(v) { ArrayList() }.add(n1 to n2)
    }
}
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo s) {
        val vector = token.variable.vectorView
        val n = nodes[vector[0]]
        val v = vehicles[vector[1]]
        time.getOrPut(v) { HashMap() }[n] = token.result!!.round().toUInt64()
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo17.kt)
