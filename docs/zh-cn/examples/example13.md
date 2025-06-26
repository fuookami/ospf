# 示例 13：二级运输问题

## 问题描述

从三个分销中心向五个经销商运输汽车，运输费用依据起点到终点的距离而定，与运输卡车满载无关。汇总从分销中心到经销商之间的里程以及相应的月供应量和需求量如下表所示：

| 分销中心 |   经销商A   |   经销商B   |   经销商C   |   经销商D   |   经销商E   |  供应量  |
| :------: | :---------: | :---------: | :---------: | :---------: | :---------: | :------: |
|    1     | $100\,mile$ | $150\,mile$ | $200\,mile$ | $140\,mile$ | $35\,mile$  | $400\,t$ |
|    2     | $50\,mile$  | $70\,mile$  | $60\,mile$  | $65\,mile$  | $80\,mile$  | $200\,t$ |
|    3     | $40\,mile$  | $90\,mile$  | $100\,mile$ | $150\,mile$ | $130\,mile$ | $150\,t$ |
|  需求量  |  $100\,t$   |  $200\,t$   |  $150\,t$   |  $160\,t$   |  $140\,t$   |    --    |

每辆卡车每英里运输费用固定，给出最优运输方案，要求：

1. 分销中心运输量不超过其供应量；
2. 各经销商的需求需得到满足；
3. 卡车装载量不超过上限 $18t$。

## 数学模型

### 变量

$x_{ij}$：分销中心 $i$ 向经销商 $j$ 的运输量。

$y_{ij}$：分销中心 $i$ 向经销商 $j$ 运输使用卡车数量。

### 中间值

#### 1. 分销中心运输总量

$$
Trans_{i} = \sum_{j \in D} x_{ij}, \; \forall i \in DC
$$

#### 2. 经销商接收总量

$$
Receive_{j} = \sum_{i \in DC} x_{ij}, \; \forall j \in D
$$

#### 3. 运输总费用

$$
Cost = \sum_{i \in DC, j \in D} d_{ij} \cdot y_{ij}
$$

### 目标函数

#### 1. 运输费用最小

$$
min \quad Cost
$$

### 约束

#### 1. 运输量不超过分销中心供应量

$$
s.t. \quad Trans_{i} \leq Supply_{i}, \; \forall i \in DC
$$

#### 2. 满足经销商需求

$$
s.t. \quad Receive_{j} \geq Demand_{j}, \; \forall j \in D
$$

#### 3. 卡车装载量不超过上限

$$
s.t. \quad y_{ij} \cdot Capacity \geq x_{ij}, \; \forall i \in DC, \; \forall j \in D
$$

## 期望结果

| 分销中心 | 经销商A  | 经销商B  | 经销商C  | 经销商D  | 经销商E  |
| :------: | :------: | :------: | :------: | :------: | :------: |
|    1     | $100\,t$ |   $-$    |   $-$    | $160\,t$ | $140\,t$ |
|    2     |   $-$    | $50\,t$  | $150\,t$ |   $-$    |   $-$    |
|    3     |   $-$    | $15\,0t$ |   $-$    |   $-$    |   $-$    |

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

data class Dealer(
    val demand: UInt64
) : AutoIndexed(Dealer::class)

data class DistributionCenter(
    val supply: UInt64,
    val distance: Map<Dealer, UInt64>
) : AutoIndexed(DistributionCenter::class)

val carCapacity = UInt64(18)
val dealers: List<Dealer> = ... // 经销商列表
val distributionCenters: List<DistributionCenter> = ... // 分销中心列表

// 创建模型实例
val metaModel = LinearMetaModel("demo13")

// 定义变量
val x = UIntVariable2("x", Shape2(dealers.size, distributionCenters.size))
metaModel.add(x)

val y = UIntVariable2("y", Shape2(dealers.size, distributionCenters.size))
metaModel.add(y)

// 定义中间值
val trans = LinearIntermediateSymbols1("trans", Shape1(distributionCenters.size)) { i, _ ->
    val distributionCenter = distributionCenters[i]
    LinearExpressionSymbol(
        sum(x[_a, distributionCenter]),
        "trans_${distributionCenter.index}"
    )
}
metaModel.add(trans)

val receive = LinearIntermediateSymbols1("receive", Shape1(dealers.size)) { i, _ ->
    val dealer = dealers[i]
    LinearExpressionSymbol(
        sum(x[dealer, _a]),
        "receive_${dealer.index}"
    )
}
metaModel.add(receive)

val cost = LinearExpressionSymbol(
    sum(dealers.flatMap { dealer ->
        distributionCenters.mapNotNull { distributionCenter ->
            val distance = distributionCenter.distance[dealer] ?: return@mapNotNull null
            distance * y[dealer, distributionCenter]
        }
    }),
    "cost"
)
metaModel.add(cost)

// 定义目标函数
metaModel.minimize(cost, "cost")

// 定义约束
for (distributionCenter in distributionCenters) {
    metaModel.addConstraint(
        trans[distributionCenter] leq distributionCenter.supply,
        "supply_${distributionCenter.index}"
    )
}

for (dealer in dealers) {
    metaModel.addConstraint(
        receive[dealer] geq dealer.demand,
        "demand_${dealer.index}"
    )
}

for (dealer in dealers) {
    for (distributionCenter in distributionCenters) {
        metaModel.addConstraint(
            x[dealer, distributionCenter] leq carCapacity * y[dealer, distributionCenter],
            "car_limit_${dealer.index}_${distributionCenter.index}"
        )
    }
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
val solution: MutableMap<DistributionCenter, MutableMap<Dealer, UInt64>> = hashMapOf()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val dealer = dealers[vector[0]]
        val distributionCenter = distributionCenters[vector[1]]
        solution.getOrPut(distributionCenter) { hashMapOf() }[dealer] = token.result!!.round().toUInt64()
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo13.kt)
