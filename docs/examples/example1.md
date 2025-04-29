# Example 1: Assignment Problem

## 1. Problem Description

Several companies have their own capital, liabilities, and profits.

|       | CAPITAL | LIABILITIES | PROFIT |
| :---: | :-----: | :---------: | :----: |
| Co. A | $3.48$  |   $1.28$    | $5400$ |
| Co. B | $5.62$  |   $2.53$    | $2300$ |
| Co. C | $7.33$  |   $1.02$    | $4600$ |
| Co. D | $6.27$  |   $3.55$    | $3300$ |
| Co. E | $2.14$  |   $0.53$    | $980$  |

Select a subset of companies to maximize total profit, while satisfying:

1. Total capital is greater than or equal to $10$ ；
2. Total liabilities is less than or equal to $5$ 。

## 2. Mathematical Model

### 1) Variables

$x_{c}$ ：to select company $c$ 。

### 2) Intermediate Expressions

#### (1) Total Capital

$$
Capital = \sum_{c \in C} Capital_{c} \cdot x_{c}
$$

#### (2) Total Liabilities

$$
Liability = \sum_{c \in C} Liability_{c} \cdot x_{c}
$$

#### (3) Total Profit

$$
Profit = \sum_{c \in C} Profit_{c} \cdot x_{c}
$$

### 3) Objective Function

#### (1) Maximize Total Profit

$$
max \quad Profit
$$

### 4) Constraints

#### (1) Minimum Capital Limit

$$
s.t. \quad Capital \geq Capital^{Min}
$$

#### (2) Maximum Liability Limit

$$
s.t. \quad Liability \leq Liability^{Max}
$$

## 3. Expected Result

Optimal selection: companies A, B and C 。

## 4. Code Implementation

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

data class Company(
    val capital: Flt64,
    val liability: Flt64,
    val profit: Flt64
) : AutoIndexed(Company::class)

val companies = listOf(
    Company(Flt64(3.48), Flt64(1.28), Flt64(5400.0)),
    Company(Flt64(5.62), Flt64(2.53), Flt64(2300.0)),
    Company(Flt64(7.33), Flt64(1.02), Flt64(4600.0)),
    Company(Flt64(6.27), Flt64(3.55), Flt64(3300.0)),
    Company(Flt64(2.14), Flt64(0.53), Flt64(980.0))
)
val minCapital = Flt64(10.0)
val maxLiability = Flt64(5.0)

// create a model instance
val metaModel = LinearMetaModel("demo1")

// define variables
val x = BinVariable1("x", Shape1(companies.size))
for (c in companies) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// define intermediate expressions
val capital = LinearExpressionSymbol(
    sum(companies) { it.capital * x[it] }, 
    "capital"
)
metaModel.add(capital)
val liability = LinearExpressionSymbol(
    sum(companies) { it.liability * x[it] }, 
    "liability"
)
metaModel.add(liability)
val profit = LinearExpressionSymbol(
    sum(companies) { it.profit * x[it] }, 
    "profit"
)
metaModel.add(profit)

// define objective function
metaModel.maximize(profit)

// define constraints
metaModel.addConstraint(capital geq minCapital)
metaModel.addConstraint(liability leq maxLiability)

// solve the model
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// parse results
val solution = ArrayList<Company>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one) {
        solution.add(companies[token.variable.index])
    }
}
```

:::

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo1.kt)
