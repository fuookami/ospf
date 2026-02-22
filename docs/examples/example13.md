# Example 13: Two-Stage Transportation Problem

## Problem Description

Transport goods from three distribution centers to five dealers. Transportation costs are determined based on the distance from origin to destination, independent of truck load capacity. The summarized distances between distribution centers and dealers, along with corresponding monthly supply and demand, are shown in the following table:

| Distribution Center | Dealer A | Dealer B | Dealer C | Dealer D | Dealer E | Supply |
| :-----------------: | :------: | :------: | :------: | :------: | :------: | :----: |
| 1 | $100\,mile$ | $150\,mile$ | $200\,mile$ | $140\,mile$ | $35\,mile$ | $400\,t$ |
| 2 | $50\,mile$ | $70\,mile$ | $60\,mile$ | $65\,mile$ | $80\,mile$ | $200\,t$ |
| 3 | $40\,mile$ | $90\,mile$ | $100\,mile$ | $150\,mile$ | $130\,mile$ | $150\,t$ |
| Demand | $100\,t$ | $200\,t$ | $150\,t$ | $160\,t$ | $140\,t$ | -- |

The transportation cost per truck per mile is fixed. Determine the optimal transportation plan while satisfying the following conditions:

1. Transportation volume from distribution centers does not exceed their supply;
2. Each dealer's demand must be met;
3. Truck load does not exceed the upper limit of $18t$.

## Mathematical Model

### Variables

$x_{ij}$: Transportation volume from distribution center $i$ to dealer $j$.

$y_{ij}$: Number of trucks used for transportation from distribution center $i$ to dealer $j$.

### Intermediate Values

#### 1. Total Transportation Volume from Distribution Center

$$
\text{Trans}_{i} = \sum_{j \in D} x_{ij}, \; \forall i \in DC
$$

#### 2. Total Volume Received by Dealer

$$
\text{Receive}_{j} = \sum_{i \in DC} x_{ij}, \; \forall j \in D
$$

#### 3. Total Transportation Cost

$$
\text{Cost} = \sum_{i \in DC, j \in D} d_{ij} \cdot y_{ij}
$$

### Objective Function

#### 1. Minimize Transportation Cost

$$
\min \quad \text{Cost}
$$

### Constraints

#### 1. Transportation Volume Does Not Exceed Distribution Center Supply

$$
\text{s.t.} \quad \text{Trans}_{i} \leq \text{Supply}_{i}, \; \forall i \in DC
$$

#### 2. Meet Dealer Demand

$$
\text{s.t.} \quad \text{Receive}_{j} \geq \text{Demand}_{j}, \; \forall j \in D
$$

#### 3. Truck Load Does Not Exceed Upper Limit

$$
\text{s.t.} \quad y_{ij} \cdot \text{Capacity} \geq x_{ij}, \; \forall i \in DC, \; \forall j \in D
$$

## Expected Results

| Distribution Center | Dealer A | Dealer B | Dealer C | Dealer D | Dealer E |
| :-----------------: | :------: | :------: | :------: | :------: | :------: |
| 1 | $100\,t$ | $-$ | $-$ | $160\,t$ | $140\,t$ |
| 2 | $-$ | $50\,t$ | $150\,t$ | $-$ | $-$ |
| 3 | $-$ | $150\,t$ | $-$ | $-$ | $-$ |

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

data class Dealer(
    val demand: UInt64
) : AutoIndexed(Dealer::class)

data class DistributionCenter(
    val supply: UInt64,
    val distance: Map<Dealer, UInt64>
) : AutoIndexed(DistributionCenter::class)

val carCapacity = UInt64(18)
val dealers: List<Dealer> = ... // Dealer list
val distributionCenters: List<DistributionCenter> = ... // Distribution center list

// Create model instance
val metaModel = LinearMetaModel("demo13")

// Define variables
val x = UIntVariable2("x", Shape2(dealers.size, distributionCenters.size))
metaModel.add(x)

val y = UIntVariable2("y", Shape2(dealers.size, distributionCenters.size))
metaModel.add(y)

// Define intermediate values
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

// Define objective function
metaModel.minimize(cost, "cost")

// Define constraints
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

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
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

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo13.kt)
