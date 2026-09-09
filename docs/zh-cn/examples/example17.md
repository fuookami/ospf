# 示例 17：带时间窗的车辆路径问题

## 问题与数据

本示例建模带服务时间窗的容量约束车辆路径问题。当前源码包含一个起点、100 个需求节点、一个终点和 25 辆相同车辆，共 102 个节点。起点和终点都位于 $(40,50)$，时间窗为 $[0,1236]$。每辆车容量为 200，固定使用成本为 500。每个需求节点都有源码提供的正整数需求量、时间窗和 90 个单位的服务时长。

源码使用欧氏几何：Node.distance 返回两个 Point2 位置之间的距离，Node.cost 和 Node.time 都返回该距离。因此旅行成本和旅行时间共用同一距离单位，没有独立的成本矩阵或速度矩阵。

## 集合与参数

令 $N$ 为全部 102 个节点，$O=\{o\}$ 为起点，$E=\{e\}$ 为终点，$D=N\setminus(O\cup E)$ 为 100 个需求节点，$K$ 为 25 辆车集合。源码实现的允许弧集合为

$$
A=\{(i,j)\in N^2:i\notin E,\ j\notin O,\ i\ne j\}.
$$

对需求节点 $j$，$q_j$ 是整数需求量，$h_j=90$ 是服务时长；$q_o=q_e=0$，起点和终点服务时长为零。每个节点有源码数据 $(position_i,[e_i,l_i])$。对车辆 $k$，$Q_k=200$，$F_k=500$。

## 决策变量

对 $(i,j)\in A$ 和 $k\in K$：

$$
x_{ijk}\in\{0,1\}
$$

表示车辆 k 是否使用弧 $i\to j$。源码在所有节点对和车辆上创建 BinVariable3，将不允许的项固定为 false，只把允许项注册到模型。

对每个 $i\in N,k\in K$，$s_{ik}\in\mathbb R_{\ge0}$ 是服务开始时间，由 URealVariable2 实现。每个 $s_{ik}$ 还会通过节点时间窗约束再次限制。

## 中间值

对每辆车 $k$ 和需求节点 $d$：

$$
Origin_k=\sum_{j:(o,j)\in A}x_{ojk},\qquad
Destination_k=\sum_{i:(i,e)\in A}x_{iek},
$$
$$
In_{dk}=\sum_{i:(i,d)\in A}x_{idk},\qquad
Out_{dk}=\sum_{j:(d,j)\in A}x_{djk}.
$$

对每个需求节点 $d$：

$$
Service_d=\sum_{k\in K}\sum_{j:(d,j)\in A}x_{djk}.
$$

对每辆车：

$$
Capacity_k=\sum_{i\in N}\sum_{j\in N}q_jx_{ijk}.
$$

源码注册两个目标：

$$
UsedCost=\sum_{k\in K}F_kOrigin_k,\qquad
TravelCost=\sum_{k\in K}\sum_{(i,j)\in A}distance_{ij}x_{ijk}.
$$

## 目标

Demo17 分别调用两次 minimize，先注册 UsedCost，再注册 TravelCost。源码没有构造加权和，也没有定义二者之间的标量系数；具体的多目标处理交由当前模型/求解器策略。

## 约束与定义域

车辆使用和路线流量：

$$
Origin_k\le1,\qquad Destination_k\le1\quad(\forall k\in K),
$$
$$
In_{dk}=Out_{dk}\quad(\forall d\in D,\ k\in K),
$$
$$
Service_d=1\quad(\forall d\in D).
$$

源码用大于等于和小于等于两条约束实现 $In=Out$ 等式。对每个 $i,j\in N$ 和 $k\in K$，添加使用源码大 M 的时间蕴含约束：

$$
s_{ik}+h_i+distance_{ij}-M(1-x_{ijk})\le s_{jk},
\qquad M=1236.
$$

对每个节点和车辆：

$$
e_i\le s_{ik}\le l_i\quad(\forall i\in N,\ k\in K),
$$

对每辆车：

$$
Capacity_k\le Q_k=200.
$$

不允许的弧在表达式注册前被固定为零。源码仍然对所有节点对创建时间约束，固定为零的项使不允许弧上的蕴含约束失效。

## 实现差异与注意事项

源码依次通过 initVariable、initSymbol、initObject、initConstraint、solve 和 analyzeSolution 构建模型。它使用当前 core 符号，并用配置为 300 秒时间限制的 ScipLinearSolver 求解。两个目标注册、1236 的大 M、允许弧过滤、非负实数时间变量都是实现事实。本示例不是允许选客的通用 VRPTW 模型：每个需求节点都必须恰好服务一次。

## 预期结果

成功求解后会返回覆盖全部 100 个需求节点的路线和服务时间，并满足每个源码时间窗及每辆车容量。当前构建测试只验证模型构建，不断言路线列表、目标值或唯一最优解。实例可能求解耗时较长，并受五分钟求解器时间限制影响。

## 当前 Kotlin 最小示例

~~~kotlin
import kotlin.time.Duration.Companion.seconds
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.algebra.value_range.*
import fuookami.ospf.kotlin.math.geometry.*
import fuookami.ospf.kotlin.math.geometry.point2
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.math.symbol.polynomial.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.config.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

val model = LinearMetaModel<Flt64>("demo17", converter = flt64Converter)
val x = BinVariable3("x", Shape3(nodes.size, nodes.size, vehicles.size))
for (from in nodes) for (to in nodes) for (vehicle in vehicles) {
    val xi = x[from, to, vehicle]
    if (from !is EndNode && to !is OriginNode && from != to) model.add(xi)
    else xi.range.eq(false)
}
val s = URealVariable2("s", Shape2(nodes.size, vehicles.size))
model.add(s)
val origin = LinearIntermediateSymbols1<Flt64>("origin", Shape1(vehicles.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<OriginNode>().flatMap { node -> x[node, _a, vehicles[i]] }),
        name = "origin_$i"
    )
}
val destination = LinearIntermediateSymbols1<Flt64>("destination", Shape1(vehicles.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<EndNode>().flatMap { node -> x[_a, node, vehicles[i]] }),
        name = "destination_$i"
    )
}
val service = LinearIntermediateSymbols1<Flt64>("service", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.filterIsNotInstance<OriginNode, Node>().flatMap { node -> x[nodes[i], node, _a] }),
        name = "service_$i"
    )
}
val capacity = LinearIntermediateSymbols1<Flt64>("capacity", Shape1(vehicles.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.flatMap { from ->
            nodes.mapNotNull { to -> (to as? DemandNode)?.demand?.let { it * x[from, to, vehicles[i]] } }
        }),
        name = "capacity_$i"
    )
}
model.add(origin)
model.add(destination)
model.add(service)
model.add(capacity)
model.minimize(sum(vehicles.map { it.fixedUsedCost * origin[it] }), "used cost")
model.minimize(
    sum(nodes.flatMap { from -> nodes.map { to -> from.cost(to) * sum(x[from, to, _a]) } }),
    "trans cost"
)
for (vehicle in vehicles) model.addConstraint(origin[vehicle] leq 1)
for (node in nodes.filterIsInstance<DemandNode>()) {
    model.addConstraint(service[node] eq 1)
    for (vehicle in vehicles) {
        model.addConstraint(inFlow[node, vehicle] geq outFlow[node, vehicle])
        model.addConstraint(inFlow[node, vehicle] leq outFlow[node, vehicle])
    }
}
for (vehicle in vehicles) {
    model.addConstraint(destination[vehicle] leq 1)
    model.addConstraint(capacity[vehicle] leq vehicle.capacity)
}
val m = nodes.filterIsInstance<EndNode>().maxOf { it.timeWindow.upperBound.value.unwrap() }
for (from in nodes) for (to in nodes) for (vehicle in vehicles) {
    model.addConstraint(
        s[from, vehicle] +
            ((from as? DemandNode)?.serviceTime ?: UInt64.zero).toFlt64() +
            from.time(to) -
            m.toFlt64() * (1 - x[from, to, vehicle]) leq s[to, vehicle]
    )
}
for (node in nodes) for (vehicle in vehicles) {
    model.addConstraint(s[node, vehicle] geq node.timeWindow.lowerBound.value.unwrap())
    model.addConstraint(s[node, vehicle] leq node.timeWindow.upperBound.value.unwrap())
}

suspend fun solve() = solveLinearMetaModel(
    ScipLinearSolver(config = SolverConfig(time = 300.seconds)),
    model
)
~~~

## 源码与验证

### Kotlin/Rust 对照

这是两个独立模型：Rust 是 4 个客户的紧凑 VRPTW，Kotlin 当前 core 实现是 100 个客户、102 个节点；两者不共享数据、Big-M 或目标组织，API 也独立。

- [Rust 对照实现：demo17.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo17.rs)

- [当前实现：Demo17.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo17.kt)
- [核心构建结构测试：CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

::: code-group

```kotlin [Kotlin]
// See the linked Kotlin implementation for the complete model.
`` 

```rust [Rust]
// See the linked Rust implementation for the equivalent model.
`` 

:::

