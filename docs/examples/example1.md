# Example 1: Assignment Problem

## Problem Description

There are several enterprises, each with its own capital amount, liability amount, and profit amount.

|        | Capital Amount | Liability Amount | Profit Amount |
| :----: | :------------: | :--------------: | :-----------: |
| Enterprise A |  $3.48$  |  $1.28$  | $5400$ |
| Enterprise B |  $5.62$  |  $2.53$  | $2300$ |
| Enterprise C |  $7.33$  |  $1.02$  | $4600$ |
| Enterprise D |  $6.27$  |  $3.55$  | $3300$ |
| Enterprise E |  $2.14$  |  $0.53$  | $980$  |

The objective is to select a subset of these enterprises to maximize the total profit amount, while satisfying the following conditions:

1. The total capital amount of selected enterprises is greater than $10$;
2. The total liability amount of selected enterprises is less than $5$.

## Mathematical Model

### Variables

$x_{c}$: indicates whether enterprise $c$ is selected.

### Intermediate Values

#### 1. Total Capital Amount

$$
\text{Capital} = \sum_{c \in C} \text{Capital}_{c} \cdot x_{c}
$$

#### 2. Total Liability Amount

$$
\text{Liability} = \sum_{c \in C} \text{Liability}_{c} \cdot x_{c}
$$

#### 3. Total Profit Amount

$$
\text{Profit} = \sum_{c \in C} \text{Profit}_{c} \cdot x_{c}
$$

### Objective Function

#### 1. Maximize Total Profit

$$
\max \quad \text{Profit}
$$

### Constraints

#### 1. Total Capital Amount Must Exceed Minimum

$$
\text{s.t.} \quad \text{Capital} \geq \text{Capital}^{\text{Min}}
$$

#### 2. Total Liability Amount Must Be Below Maximum

$$
\text{s.t.} \quad \text{Liability} \leq \text{Liability}^{\text{Max}}
$$

## Expected Results

Select enterprises A, B, and C.

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

data class Company(
    val capital: Flt64,
    val liability: Flt64,
    val profit: Flt64
) : AutoIndexed(Company::class)

val companies: List<Company> = ... // Company list
val minCapital = Flt64(10.0)
val maxLiability = Flt64(5.0)

// Create model instance
val metaModel = LinearMetaModel("demo1")

// Define variables
val x = BinVariable1("x", Shape1(companies.size))
for (c in companies) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// Define intermediate values
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

// Define objective function
metaModel.maximize(profit)

// Define constraints
metaModel.addConstraint(capital geq minCapital)
metaModel.addConstraint(liability leq maxLiability)

// Invoke solver
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution = ArrayList<Company>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        solution.add(companies[token.variable.index])
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo1.kt)
