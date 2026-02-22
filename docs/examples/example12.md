# Example 12: Portfolio Optimization Problem

## Problem Description

There are $n$ assets $S_{i}$ ($i=1, 2, \cdots, n$) available in the market for investment, using funds amounting to $M$ for a one-period investment. The average return rate for purchasing $S_{i}$ during this period is $r_{i}$, the risk loss rate is $q_{i}$. The more diversified the investment, the smaller the total risk. The overall risk can be obtained by weighting the investment amount of $S_{i}$ with its investment risk.

When purchasing $S_{i}$, a transaction fee is charged at rate $p_{i}$. When the purchase amount does not exceed a given value $u_{i}$, the transaction fee is calculated based on purchasing $u_{i}$. Additionally, assume the bank deposit interest rate during the same period is $r_{0}$, which has neither transaction fees nor risk. ($r_{0} = 5\%$)

| $S_{i}$ | $r_{i}$(%) | $q_{i}$(%) | $p_{i}$(%) | $u_{i}$(yuan) |
| :-----: | :--------: | :--------: | :--------: | :-----------: |
| $S_{1}$ |    $28$    |   $4.0$    |    $8$     |    $103$      |
| $S_{2}$ |    $21$    |   $1.5$    |    $2$     |    $198$      |
| $S_{3}$ |    $23$    |   $5.5$    |   $4.5$    |    $52$       |
| $S_{4}$ |    $25$    |   $2.6$    |    $4$     |    $40$       |

Design an investment portfolio strategy with given funds of $1000000$ yuan, selectively purchasing assets or bank deposits, to maximize net return while controlling risk within $2\%$.

## Mathematical Model

### Variables

$x_{i}$: Purchase funds for investment product $i$.

### Intermediate Values

#### 1. Whether Purchased

$$
\text{Assignment}_{i} = \text{If}(x_{i} > 0), \; \forall i \in S
$$

#### 2. Transaction Fee

$$
\text{Premium}_{i} = \text{Max}(p_{i} \cdot x_{i}, \, u_{i} \cdot \text{Assignment}_{i}), \; \forall i \in S
$$

#### 3. Return

$$
\text{Yield} = \sum_{i \in S} (r_{i} \cdot x_{i} - \text{Premium}_{i})
$$

#### 4. Risk

$$
\text{Risk} = \frac{\sum_{i \in S} q_{i} \cdot x_{i}}{M}
$$

### Objective Function

#### 1. Maximize Return

$$
\max \quad \text{Yield}
$$

### Constraints

#### 1. Sum of Funds Equals $M$

$$
\text{s.t.} \quad \sum_{i \in S}(1 + p_{i}) \cdot x_{i} = M
$$

#### 2. Risk Limit

$$
\text{s.t.} \quad \sum_{i \in S} \text{Risk}_{i} \leq \text{Risk}^{\text{Max}}
$$

## Expected Results

Invest $494505$ yuan in $S_{4}$ and $476191$ yuan in $S_{2}$.

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

data class Product(
    val yield: Flt64,
    val risk: Flt64,
    val premium: Flt64,
    val minPremium: Flt64
) : AutoIndexed(Product::class)

val products: List<Product> = ... // Product list
val funds = Flt64(1000000.0)
val maxRisk = Flt64(0.02)

// Create model instance
val metaModel = LinearMetaModel("demo12")

// Define variables
val x = UIntVariable1("x", Shape1(products.size))
metaModel.add(x)

// Define intermediate values
val assignment = LinearIntermediateSymbols1(
    "assignment",
    Shape1(products.size)
) { i, _ ->
    BinaryzationFunction(
        x = LinearPolynomial(x[i]),
        name = "assignment_$i"
    )
}
metaModel.add(assignment)

val premium = LinearIntermediateSymbols1(
    "premium",
    Shape1(products.size)
) { i, _ ->
    val product = products[i]
    MaxFunction(
        listOf(
            LinearPolynomial(product.premium * x[i]),
            LinearPolynomial(product.minPremium * assignment[i])
        ),
        "premium_$i"
    )
}
metaModel.add(premium)

val risk = LinearExpressionSymbol(
    sum(products.map { p -> p.risk * x[p] / funds }),
    "risk"
)
metaModel.add(risk)

val yield = LinearExpressionSymbol(
    sum(products.map { p -> p.yield * x[p] - premium[p] }),
    "yield"
)
metaModel.add(yield)

// Define objective function
metaModel.maximize(yield, "yield")

// Define constraints
metaModel.addConstraint(
    sum(products.map { p -> x[p] + premium[p] }) eq funds,
    "funs"
)

metaModel.addConstraint(
    risk leq maxRisk,
    "risk"
)

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution = HashMap<Product, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
        val vector = token.variable.vectorView
        val product = products[vector[0]]
        solution[product] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo12.kt)
