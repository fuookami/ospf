# 示例 16：跨月生产与库存

## 问题与数据

四个时期为三月、四月、五月和六月，其生产能力与需求量如下：

| 月份 | 三月 | 四月 | 五月 | 六月 |
| :---: | ---: | ---: | ---: | ---: |
| 生产能力 | 50 | 180 | 280 | 270 |
| 需求量 | 100 | 200 | 180 | 300 |

源码参数为单位生产成本 $C^p=40$、延迟交付成本 $C^d=2$ 和库存成本 $C^s=0.5$。总生产能力和总需求都为 780，但源码约束本身仍是不等式。

## 集合与参数

令 $M$ 为有序时期集合。`Produce` 上的 `productivity` 和 `demand` 分别是 $Productivity_i$ 与 $Demand_i$；$i<j$ 表示时期 $i$ 早于时期 $j$。

## 决策变量

 对所有 $(i,j)\in M\times M$，$x_{ij}\in\mathbb{Z}_{\ge0}$ 表示在时期 $i$ 生产、用于满足时期 $j$ 需求的数量。源码使用 UIntVariable2，没有固定非对角方向的变量。

## 中间值

$$
Produce_i=\sum_{j\in M}x_{ij},\qquad
Supply_i=\sum_{j\in M}x_{ji}.
$$

当前成本符号严格为：

$$
Cost^d=C^d\sum_{i<j}(j-i)^2x_{ji},\qquad
Cost^s=C^s\sum_{i<j}(j-i)x_{ij},\qquad
Cost^p=C^p\sum_{i\in M}x_{ii}.
$$

因此，晚生产后补早期需求使用正的平方延迟成本，早生产并向后存储使用正的线性库存成本，只有对角线生产量计入生产成本。最后一点是当前实现事实，并不是通常的“所有生产量都计生产成本”。

## 目标

$$
\min Cost^d+Cost^s+Cost^p.
$$

## 约束

$$
Supply_i\ge Demand_i\quad(\forall i\in M),\qquad
Produce_i\le Productivity_i\quad(\forall i\in M).
$$

旧页面的 $(i-j)x_{ji}$ 符号错误；其 $\sum_i Produce_i$ 生产成本也遗漏了 $C^p$ 和源码只计对角线的限制。

## 实现说明与预期结果

`Demo16` 使用 `UIntVariable2`、`LinearIntermediateSymbols1<Flt64>`、`LinearExpressionSymbol` 和当前 `LinearMetaModel<Flt64>`。应按此目标重新生成数值分配；旧表格不是当前“仅对角线生产计费”实现经过验证的最优解。结果至少应是非负整数矩阵，列和满足需求、行和不超过生产能力。

## 当前 Kotlin 最小示例

```kotlin
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

val model = LinearMetaModel<Flt64>("demo16", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(produces.size, produces.size))
val produce = LinearIntermediateSymbols1<Flt64>("produce", Shape1(produces.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[produces[i], _a]), name = "produce_${produces[i].month}")
}
val supply = LinearIntermediateSymbols1<Flt64>("supply", Shape1(produces.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, produces[i]]), name = "supply_${produces[i].month}")
}
val delay = LinearExpressionSymbol(
    sum(produces.withIndex().flatMap { (i, _) -> produces.withIndex().mapNotNull { (j, _) ->
        if (i < j) Flt64(j - i).sqr() * delayDeliveryPrice * x[produces[j], produces[i]] else null
    } }),
    name = "delay_delivery_cost"
)
val storage = LinearExpressionSymbol(
    sum(produces.withIndex().flatMap { (i, _) -> produces.withIndex().mapNotNull { (j, _) ->
        if (i < j) Flt64(j - i) * stowagePrice * x[produces[i], produces[j]] else null
    } }),
    name = "storage_cost"
)
val production = LinearExpressionSymbol(productPrice * sum(x[_a, _a]), name = "produce_cost")
model.add(x)
model.add(produce)
model.add(supply)
model.add(delay)
model.add(storage)
model.add(production)
model.minimize(delay + storage + production, "cost")
for (p in produces) {
    model.addConstraint(supply[p] geq p.demand)
    model.addConstraint(produce[p] leq p.productivity)
}

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

## 源码与验证

### Kotlin/Rust 对照

两端 API 独立且目标不完全相同：Rust 对所有 x_ij 收取生产成本，而 Kotlin 当前仅对角线生产量收取生产成本。

- [Rust 对照实现：demo16.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo16.rs)

- [当前实现：`Demo16.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo16.kt)
- [Core 结构构建测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

::: code-group

```kotlin [Kotlin]
// See the linked Kotlin implementation for the complete model.
`` 

```rust [Rust]
// See the linked Rust implementation for the equivalent model.
`` 

:::

