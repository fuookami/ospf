# 示例 15：供给运输问题

## 问题描述

MG汽车制造公司有三个生产厂分别位于洛杉矶，底特律和新奥尔良，在丹佛和迈阿密设有两个分销中心。生产M1，M2，M3和M4四种型号的汽车，各厂生产能力与各分销中心的运输能力与需求量如下：

|  生产厂  | 车型 M1 | 车型 M2 | 车型 M3 | 车型 M4 |
| :------: | :-----: | :-----: | :-----: | :-----: |
|  洛杉矶  |   --    |   --    |   700   |   300   |
|  底特律  |   500   |   600   |   --    |   400   |
| 新奥尔良 |   800   |   400   |   --    |   --    |

| 销售中心 | 车型 M1 | 车型 M2 | 车型 M3 | 车型 M4 |
| :------: | :-----: | :-----: | :-----: | :-----: |
|   丹佛   |   700   |   500   |   500   |   600   |
|  迈阿密  |   600   |   500   |   200   |   100   |

所有型号汽车的运输费用都是8美分每英里，生产厂与销售中心每辆车的运输费用如下：

|          | 丹佛  | 迈阿密 |
| :------: | :---: | :----: |
|  洛杉矶  |  80   |  215   |
|  底特律  |  100  |  108   |
| 新奥尔良 |  102  |   68   |

另外，可以按一定比例从其他厂满足分销中心对于某种汽车的需求量：

| 分销中心 | 需求比例 | 可互换型号 |
| :------: | :------: | :--------: |
|   丹佛   |    10    |   M1，M2   |
|          |    20    |   M3，M4   |
|  迈阿密  |    10    |   M1，M2   |
|          |    5     |   M2，M4   |

给出最优的运输方案，要求：

1）各分销中心的需求得到满足；
2）各生产厂运输量不超过生产车型上限。

## 数学模型

### 变量

$x_{ijk}$：生产厂 $i$ 向分销中心 $j$ 供应车型 $k$ 的量。

$y_{j, (k, k^{\prime})}$：替换分销中心 $j$ 一定比例的 $k$ 车型为 $k^{\prime}$ 车型。

### 中间值

#### 1. 分销中心各车型供应量

$$
Receive_{jk} = \sum_{i \in P} x_{ijk}, \; \forall j \in DC, \; \forall k \in M
$$

#### 2. 分销中心各车型需求量

$$
\begin{align} ExDemand_{jk} = \quad
& Demend_{jk} - \sum_{k^{\prime} \in RF_{jk}} Demend_{jk} \cdot Rate_{j, (k, k^{\prime})} \cdot y_{(j, k, k^{\prime})} \\
& + \sum_{k^{\prime} \in RT_{jk}} Demend_{jk^{\prime}}\cdot Rate_{j, (k^{\prime}, k)} \cdot y_{(j, k^{\prime}, k)}, \; \forall j \in DC, \; \forall k \in M
\end{align}
$$

#### 3. 生产厂运输总量

$$
Trans_{ik} = \sum_{j \in DC} x_{ijk}, \; \forall i \in P, \; \forall k \in M
$$

#### 4. 运输总费用

$$
Cost = \sum_{i \in P} \sum_{j \in DC} \sum_{k\in M} c_{ij} x_{ijk}
$$

### 目标函数

#### 1. 运输总费用最小

$$
min \quad Cost
$$

### 约束

#### 1. 满足分销中心需求

$$
s.t. \quad Receive_{jk} = Demand_{jk}, \; \forall j \in DC, \; \forall k \in M
$$

#### 2. 生产厂运输量不超过生产能力

$$
s.t. \quad Trans_{ik} \leq Produce_{ik}, \; \forall i \in P, \; \forall k \in M
$$

#### 3. 车型替换互斥

$$
s.t. \quad y_{(j, k, k^{\prime})} + y_{(j, k^{\prime}, k)} \leq 1, \; \forall j \in DC, \; \forall (k, k^{\prime}) \in R_{j}
$$

## 期望结果

车型 M1：

|          | 丹佛  | 迈阿密 |
| :------: | :---: | :----: |
|  洛杉矶  |   -   |   -    |
|  底特律  |  500  |   -    |
| 新奥尔良 |  200  |  600   |

车型 M2：

|          | 丹佛  | 迈阿密 |
| :------: | :---: | :----: |
|  洛杉矶  |   -   |   -    |
|  底特律  |  500  |  100   |
| 新奥尔良 |   -   |  400   |

车型 M3：

|          | 丹佛  | 迈阿密 |
| :------: | :---: | :----: |
|  洛杉矶  |  500  |  200   |
|  底特律  |   -   |   -    |
| 新奥尔良 |   -   |   -    |

车型 M4：

|          | 丹佛  | 迈阿密 |
| :------: | :---: | :----: |
|  洛杉矶  |  300  |   -    |
|  底特律  |  300  |  100   |
| 新奥尔良 |   -   |   -    |

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

data class CarModel(
    val name: String
) : AutoIndexed(CarModel::class)

data class Replacement(
    val c1: CarModel,
    val c2: CarModel,
    val maximum: Flt64
)

data class DistributionCenter(
    val name: String,
    val replacements: List<Replacement>,
    val demands: Map<CarModel, UInt64>
) : AutoIndexed(DistributionCenter::class)

data class Manufacturer(
    val name: String,
    val productivity: Map<CarModel, UInt64>,
    val logisticsCost: Map<DistributionCenter, UInt64>
) : AutoIndexed(Manufacturer::class)

val carModels: List<CarModel> = ... // 车型列表
val distributionCenters: List<DistributionCenter> = ... // 分销中心列表
val manufacturers: List<Manufacturer> = ... // 生产厂列表

// 创建模型实例
val metaModel = LinearMetaModel("demo15")

// 定义变量
val x = UIntVariable3("x", Shape3(manufacturers.size, distributionCenters.size, carModels.size))
for (m in manufacturers) {
    for (d in distributionCenters) {
        for (c in carModels) {
            val xi = x[m, d, c]
            if (m.productivity.containsKey(c)) {
                xi.name = "x_${m.name}_${d.name}_${c.name}"
                metaModel.add(xi)
            } else {
                 xi.range.eq(UInt64.zero)
            }
        }
    }
}

val y = distributionCenters.associateWith { d ->
    val y = PctVariable1("y_${d.name}", Shape1(d.replacements.size))
    for ((r, replacement) in d.replacements.withIndex()) {
        val yi = y[r]
        yi.range.leq(replacement.maximum)
        yi.name = "${y.name}_${r}"
    }
    y
}
metaModel.add(y.values)

// 定义中间值
val receive = LinearIntermediateSymbols2(
    "receive",
    Shape2(distributionCenters.size, carModels.size)
) { _, v ->
    val d = distributionCenters[v[0]]
    val c = carModels[v[1]]
    LinearExpressionSymbol(sum(x[_a, d, c]), "receive_${d.name}_${c.name}")
}
metaModel.add(receive)

val demand = LinearIntermediateSymbols2(
    "demand",
    Shape2(distributionCenters.size, carModels.size)
) { _, v ->
    val d = distributionCenters[v[0]]
    val c = carModels[v[1]]
    val replacedDemand = if (d.demands[c]?.let { it gr UInt64.zero } == true) {
        sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
            if (replacement.c1 != c) {
                return@mapNotNull null
            }
            d.demands[c]!!.toFlt64() * y[d]!![r]
        })
    } else {
        LinearPolynomial(Flt64.zero)
    }
    val replacedToDemand = sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
        if (replacement.c2 != c) {
            return@mapNotNull null
        }
        d.demands[replacement.c1]?.let {
            if (it gr UInt64.zero) {
                it.toFlt64() * y[d]!![r]
            } else {
                null
            }
        }
    })
    LinearExpressionSymbol((d.demands[c] ?: UInt64.zero) - replacedDemand + replacedToDemand, "demand_${d.name}_${c.name}")
}
metaModel.add(demand)

val trans = LinearIntermediateSymbols2(
    "trans",
    Shape2(manufacturers.size, carModels.size)
) { _, v ->
    val m = manufacturers[v[0]]
    val c = carModels[v[1]]
    LinearExpressionSymbol(sum(x[m, _a, c]), "trans_${m.name}_${c.name}")
}
metaModel.add(trans)

val cost = LinearExpressionSymbol(sum(manufacturers.flatMap { m ->
    distributionCenters.flatMap { d ->
        m.logisticsCost[d]?.let {
            carModels.mapNotNull { c ->
                if (m.productivity.containsKey(c)) {
                    it * x[m, d, c]
                } else {
                    null
                }
            }
        } ?: emptyList()
    }
}))
metaModel.add(cost)

// 定义目标函数
metaModel.minimize(cost, "cost")

// 定义约束
for (d in distributionCenters) {
    for (c in carModels) {
        d.demands[c]?.let {
            metaModel.addConstraint(
                receive[d, c] geq it,
                "demand_${d.name}_${c.name}"
            )
        }
    }
}

for (m in manufacturers) {
    for (c in carModels) {
        m.productivity[c]?.let {
            metaModel.addConstraint(
                trans[m, c] geq it,
                "produce_${m.name}_${c.name}"
            )
        }
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
val trans: MutableMap<Manufacturer, MutableMap<Pair<DistributionCenter, CarModel>, UInt64>> = HashMap()
val replacement: MutableMap<DistributionCenter, MutableMap<Pair<CarModel, CarModel>, UInt64>> = HashMap()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val m = manufacturers[vector[0]]
        val d = distributionCenters[vector[1]]
        val c = carModels[vector[2]]
        trans.getOrPut(m) { hashMapOf() }[d to c] = token.result!!.round().toUInt64()
    }
    for ((_, d) in distributionCenters.withIndex()) {
        val yi = y[d]!!
        if (token.result!! neq Flt64.zero && token.variable belongsTo yi) {
            val vector = token.variable.vectorView
            val r = d.replacements[vector[0]]
            replacement.getOrPut(d) { hashMapOf() }[r.c1 to r.c2] = (token.result!! * d.demands[r.c1]!!.toFlt64()).round().toUInt64()
        }
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo13.kt)
