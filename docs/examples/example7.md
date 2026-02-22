# Example 7: Transportation Problem

## Problem Description

There are several warehouses and several stores. Each warehouse has a corresponding goods quantity, each store has a corresponding goods demand, and there is a cost for transporting $1\,kg$ of goods from each warehouse to each store.

|        | Warehouse A | Warehouse B | Warehouse C |
| :----: | :---------: | :---------: | :---------: |
| Goods Quantity | $510\,kg$ | $470\,kg$ | $520\,kg$ |

|            | Store A | Store B | Store C | Store D |
| :--------: | :-----: | :-----: | :-----: | :-----: |
| Goods Demand | $200\,kg$ | $400\,kg$ | $600\,kg$ | $300\,kg$ |

|        | Store A | Store B | Store C | Store D |
| :----: | :-----: | :-----: | :-----: | :-----: |
| Warehouse A |  $12$  |  $13$  |  $21$  |  $7$   |
| Warehouse B |  $14$  |  $17$  |  $8$   |  $18$  |
| Warehouse C |  $10$  |  $11$  |  $9$   |  $15$  |

Determine the goods transportation quantity from each warehouse to each store to minimize total cost.

## Mathematical Model

### Variables

$x_{ws}$: Goods transportation quantity from warehouse $w$ to store $s$.

### Intermediate Values

#### 1. Total Cost

$$
\text{Cost} = \sum_{w \in W}\sum_{s \in S} \text{Cost}_{ws} \cdot x_{ws}
$$

#### 2. Warehouse Shipment Quantity

$$
\text{Shipment}_{w} = \sum_{s \in S} x_{ws}, \; \forall w \in W
$$

#### 3. Store Purchase Quantity

$$
\text{Purchase}_{s} = \sum_{w \in W} x_{ws}, \; \forall s \in S
$$

### Objective Function

#### 1. Minimize Total Cost

$$
\min \quad \text{Cost}
$$

### Constraints

#### 1. Warehouse Shipment Quantity Cannot Exceed Goods Quantity

$$
\text{s.t.} \quad \text{Shipment}_{w} \leq \text{Storage}_{w}, \; \forall w \in W
$$

#### 2. Store Purchase Quantity Must Meet Demand

$$
\text{s.t.} \quad \text{Purchase}_{s} \geq \text{Demand}_{s}, \; \forall s \in S
$$

## Expected Results

|        | Store A | Store B | Store C | Store D |
| :----: | :-----: | :-----: | :-----: | :-----: |
| Warehouse A | $200\,kg$ | $10\,kg$  |  $0\,kg$  | $300\,kg$ |
| Warehouse B |  $0\,kg$  |  $0\,kg$  | $470\,kg$ |  $0\,kg$  |
| Warehouse C |  $0\,kg$  | $390\,kg$ | $130\,kg$ |  $0\,kg$  |

## Code Implementation

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data class Store(
    val demand: Flt64
) : AutoIndexed(Store::class)

data class Warehouse(
    val stowage: Flt64,
    val cost: Map<Store, Flt64>
) : AutoIndexed(Warehouse::class)

val stores: List<Store> = ... // Store list
val warehouses: List<Warehouse> = ... // Warehouse list

// Create model instance
val metaModel = LinearMetaModel("demo7")

// Define variables
val x = UIntVariable2("x", Shape2(warehouses.size, stores.size))
for (w in warehouses) {
    for (s in stores) {
        x[w, s].name = "${x.name}_(${w.index},${s.index})"
    }
}
metaModel.add(x)

// Define intermediate values
val cost = LinearExpressionSymbol(sum(warehouses.map { w ->
    sum(stores.filter { w.cost.contains(it) }.map { s ->
        w.cost[s]!! * x[w, s]
    })
}), "cost")
metaModel.add(cost)

val shipment = LinearIntermediateSymbols1(
    "shipment",
    Shape1(warehouses.size)
) { i, _ ->
    val w = warehouses[i]
    LinearExpressionSymbol(
        sum(stores.filter { w.cost.contains(it) }.map { s -> x[w, s] }),
        "shipment_${w.index}"
    )
}
metaModel.add(shipment)

val purchase = LinearIntermediateSymbols1(
    "purchase",
    Shape1(stores.size)
) { i, _ ->
    val s = stores[i]
    LinearExpressionSymbol(
        sum(warehouses.filter { w -> w.cost.contains(s) }.map { w -> x[w, s] }),
        "purchase_${s.index}"
    )
}
metaModel.add(purchase)

// Define objective function
metaModel.minimize(cost, "cost")

// Define constraints
for(w in warehouses){
    metaModel.addConstraint(
        shipment[w] leq w.stowage,
        "stowage_${w.index}"
    )
}

for(s in stores){
    metaModel.addConstraint(
        purchase[s] geq s.demand,
        "demand_${s.index}"
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
val solution = stores.associateWith { warehouses.associateWith { Flt64.zero }.toMutableMap() }
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
        val warehouse = warehouses[token.variable.vectorView[0]]
        val store = stores[token.variable.vectorView[1]]
        solution[store]!![warehouse] = solution[store]!![warehouse]!! + token.result!!
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo7.kt)
