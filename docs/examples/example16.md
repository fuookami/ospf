# Example 16: Production Inventory Problem

## Problem Description

A company produces backpacks. Product demand typically occurs from March to June each year. The estimated demand for these four months is $100$, $200$, $180$, and $300$ respectively. It is estimated that $50$, $180$, $280$, and $270$ backpacks can be produced in March to June respectively. Due to differences in production capacity each month, monthly demand can be satisfied through the following three methods:

1. Production cost for backpacks produced in the current month is $40$ dollars per backpack;
2. For excess products from previous months, storage cost is $0.5$ dollars per backpack per month;
3. Backorder, with a backorder cost of $2$ dollars per backpack per month.

Develop an optimal production plan for the four months to minimize cost, while satisfying the following conditions:

1. Demand for all four months is met;
2. Monthly production does not exceed estimated production capacity.

## Mathematical Model

### Variables

$x_{ij}$: Quantity of backpacks supplied from month $i$ to month $j$.

### Intermediate Values

#### 1. Total Monthly Production

$$
\text{Produce}_{i} = \sum_{j \in M}x_{ij}, \; \forall i \in M
$$

#### 2. Monthly Backpack Supply

$$
\text{Supply}_{j} = \sum_{i \in M} x_{ij}, \; \forall j \in M
$$

#### 3. Backorder Cost

$$
\text{Cost}^{d} = C^{d} \cdot \sum_{i \in M} \sum_{j \in M, i < j} (i - j) \cdot x_{ji}
$$

#### 4. Backpack Storage Cost

$$
\text{Cost}^{s} = C^{s} \cdot \sum_{i \in M} \sum_{j \in M, \; i < j} (j - i) \cdot x_{ij}
$$

#### 5. Backpack Production Cost

$$
\text{Cost}^{p} = \sum_{i \in M} \text{Produce}_{i}
$$

### Objective Function

#### 1. Minimize Total Backpack Production Cost

$$
\min \quad \text{Cost}^{d} + \text{Cost}^{s} + \text{Cost}^{p}
$$

### Constraints

#### 1. All Demand Must Be Satisfied

$$
\text{s.t.} \quad \text{Supply}_{i} = \text{Demand}_{i}, \; \forall i \in M
$$

#### 2. Monthly Production Does Not Exceed Expected Production Capacity

$$
\text{s.t.} \quad \text{Produce}_{i} \leq \text{Produce}^{e}_{i}, \; \forall i \in M
$$

## Expected Results

|       | March | April | May | June |
| :---: | :---: | :---: | :---: | :---: |
| March | $50$  |  $-$  |  $-$  |  $-$  |
| April | $50$  | $130$ |  $-$  |  $-$  |
| May |  $-$  | $70$  | $180$ | $30$  |
| June |  $-$  |  $-$  |  $-$  | $270$ |

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

data class Produce(
    val month: UInt64,
    val productivity: UInt64,
    val demand: UInt64
) : AutoIndexed(Produce::class)

val productPrice: Flt64 = Flt64(40.0)
val delayDeliveryPrice: Flt64 = Flt64(2.0)
val stowagePrice: Flt64 = Flt64(0.5)

val produces: List<Produce> = ... // Production capacity list

// Create model instance
val metaModel = LinearMetaModel("demo16")

// Define variables
val x = UIntVariable2("x", Shape2(produces.size, produces.size))
metaModel.add(x)

// Define intermediate values
val produce = LinearIntermediateSymbols1("produce", Shape1(produces.size)) { i, _ ->
    val p = produces[i]
    LinearExpressionSymbol(sum(x[p, _a]), "produce_${p.month}")
}
metaModel.add(produce)

val supply = LinearIntermediateSymbols1("supply", Shape1(produces.size)) { i, _ ->
    val p = produces[i]
    LinearExpressionSymbol(sum(x[_a, p]), "supply_${p.month}")
}
metaModel.add(supply)

val delayDeliveryCost = LinearExpressionSymbol(sum(produces.withIndex().flatMap { (i, _) ->
    produces.withIndex().mapNotNull { (j, _) ->
        if (i < j) {
            Flt64(j - i).sqr() * delayDeliveryPrice * x[j, i]
        } else {
            null
        }
    }
}), "delay_delivery_cost")
metaModel.add(delayDeliveryCost)
        
val storageCost = LinearExpressionSymbol(sum(produces.withIndex().flatMap { (i, _) ->
    produces.withIndex().mapNotNull { (j, _) ->
        if (i < j) {
            Flt64(j - i).sqr() * stowagePrice * x[i, j]
        } else {
            null
        }
    }
}), "storage_cost")
metaModel.add(storageCost)
        
val produceCost = LinearExpressionSymbol(productPrice * sum(x[_a, _a]), "produce_cost")
metaModel.add(produceCost)

// Define objective function
metaModel.minimize(
    delayDeliveryCost + storageCost + produceCost,
    "cost"
)

// Define constraints
for (p in produces) {
    metaModel.addConstraint(
        supply[p] geq p.demand,
        "demand_${p.month}"
    )
}

for (p in produces) {
    metaModel.addConstraint(
        produce[p] leq p.productivity,
        "productivity_${p.month}"
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
val solution = HashMap<UInt64, HashMap<UInt64, UInt64>>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val i = UInt64(vector[0])
        val j = UInt64(vector[1])
        solution.getOrPut(i) { HashMap() }[j] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo16.kt)
