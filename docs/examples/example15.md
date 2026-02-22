# Example 15: Supply Transportation Problem

## Problem Description

MG Automobile Manufacturing Company has three production plants located in Los Angeles, Detroit, and New Orleans, with two distribution centers in Denver and Miami. The company produces four car models: M1, M2, M3, and M4. The production capacity of each plant and the transportation capacity and demand of each distribution center are as follows:

| Production Plant | Model M1 | Model M2 | Model M3 | Model M4 |
| :--------------: | :------: | :------: | :------: | :------: |
| Los Angeles |   --    |   --    |  $700$  |  $300$  |
| Detroit |  $500$  |  $600$  |   --    |  $400$  |
| New Orleans |  $800$  |  $400$  |   --    |   --    |

| Distribution Center | Model M1 | Model M2 | Model M3 | Model M4 |
| :-----------------: | :------: | :------: | :------: | :------: |
| Denver |  $700$  |  $500$  |  $500$  |  $600$  |
| Miami |  $600$  |  $500$  |  $200$  |  $100$  |

The transportation cost for all car models is 8 cents per mile. The transportation cost per car from production plants to distribution centers is as follows:

|          | Denver | Miami |
| :------: | :----: | :---: |
| Los Angeles | $80$  | $215$ |
| Detroit | $100$ | $108$ |
| New Orleans | $102$ | $68$  |

Additionally, a certain percentage of demand for specific car models at distribution centers can be satisfied by other models:

| Distribution Center | Demand Percentage | Interchangeable Models |
| :-----------------: | :---------------: | :--------------------: |
| Denver |   $10$   |   M1、M2   |
|        |   $20$   |   M3、M4   |
| Miami |   $10$   |   M1、M2   |
|       |   $5$    |   M2、M4   |

Determine the optimal transportation plan while satisfying the following conditions:

1. Demand at each distribution center is met;
2. Transportation volume from each production plant does not exceed the production capacity for each car model.

## Mathematical Model

### Variables

$x_{ijk}$: Quantity of car model $k$ supplied from production plant $i$ to distribution center $j$.

$y_{j, (k, k^{\prime})}$: Replacement of a certain percentage of car model $k$ with car model $k^{\prime}$ at distribution center $j$.

### Intermediate Values

#### 1. Supply of Each Car Model at Distribution Center

$$
\text{Receive}_{jk} = \sum_{i \in P} x_{ijk}, \; \forall j \in DC, \; \forall k \in M
$$

#### 2. Demand for Each Car Model at Distribution Center

$$
\begin{align} \text{ExDemand}_{jk} = \quad
& \text{Demand}_{jk} - \sum_{k^{\prime} \in RF_{jk}} \text{Demand}_{jk} \cdot \text{Rate}_{j, (k, k^{\prime})} \cdot y_{(j, k, k^{\prime})} \\
& + \sum_{k^{\prime} \in RT_{jk}} \text{Demand}_{jk^{\prime}}\cdot \text{Rate}_{j, (k^{\prime}, k)} \cdot y_{(j, k^{\prime}, k)}, \; \forall j \in DC, \; \forall k \in M
\end{align}
$$

#### 3. Total Transportation Volume from Production Plant

$$
\text{Trans}_{ik} = \sum_{j \in DC} x_{ijk}, \; \forall i \in P, \; \forall k \in M
$$

#### 4. Total Transportation Cost

$$
\text{Cost} = \sum_{i \in P} \sum_{j \in DC} \sum_{k\in M} c_{ij} x_{ijk}
$$

### Objective Function

#### 1. Minimize Total Transportation Cost

$$
\min \quad \text{Cost}
$$

### Constraints

#### 1. Meet Distribution Center Demand

$$
\text{s.t.} \quad \text{Receive}_{jk} = \text{Demand}_{jk}, \; \forall j \in DC, \; \forall k \in M
$$

#### 2. Production Plant Transportation Volume Does Not Exceed Production Capacity

$$
\text{s.t.} \quad \text{Trans}_{ik} \leq \text{Produce}_{ik}, \; \forall i \in P, \; \forall k \in M
$$

#### 3. Car Model Replacement Mutual Exclusivity

$$
\text{s.t.} \quad y_{(j, k, k^{\prime})} + y_{(j, k^{\prime}, k)} \leq 1, \; \forall j \in DC, \; \forall (k, k^{\prime}) \in R_{j}
$$

## Expected Results

Car Model M1:

|          | Denver | Miami |
| :------: | :----: | :---: |
| Los Angeles |  $-$  |  $-$   |
| Detroit | $500$ |  $-$   |
| New Orleans | $200$ | $600$  |

Car Model M2:

|          | Denver | Miami |
| :------: | :----: | :---: |
| Los Angeles |  $-$  |  $-$   |
| Detroit | $500$ | $100$  |
| New Orleans |  $-$  | $400$  |

Car Model M3:

|          | Denver | Miami |
| :------: | :----: | :---: |
| Los Angeles | $500$ | $200$  |
| Detroit |  $-$  |  $-$   |
| New Orleans |  $-$  |  $-$   |

Car Model M4:

|          | Denver | Miami |
| :------: | :----: | :---: |
| Los Angeles | $300$ |  $-$   |
| Detroit | $300$ | $100$  |
| New Orleans |  $-$  |  $-$   |

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

val carModels: List<CarModel> = ... // Car model list
val distributionCenters: List<DistributionCenter> = ... // Distribution center list
val manufacturers: List<Manufacturer> = ... // Manufacturer list

// Create model instance
val metaModel = LinearMetaModel("demo15")

// Define variables
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

// Define intermediate values
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

// Define objective function
metaModel.minimize(cost, "cost")

// Define constraints
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

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
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
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo15.kt)
