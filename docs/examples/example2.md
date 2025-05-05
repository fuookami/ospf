# Example 2: Assignment Problem

## Problem Description

There are several companies and products. Each company incurs a specific cost to produce each product.

|       | Product A | Product B | Product C | Product D |
| :---: | :-------: | :-------: | :-------: | :-------: |
| Co. A |   $920$   |   $480$   |   $650$   |   $340$   |
| Co. B |   $870$   |   $510$   |   $700$   |   $350$   |
| Co. C |   $880$   |   $500$   |   $720$   |   $400$   |
| Co. D |   $930$   |   $490$   |   $680$   |   $410$   |

The goal is to assign distinct companies to produce different products such that the total cost is minimized.

## Mathematical Model

### Variables

$x_{cp}$ ：whether company $c$ is assigned to produce product $p$ 。

### Intermediate Expressions

#### 1. Total Cost

$$
Cost = \sum_{c \in C} \sum_{p \in P} Cost_{cp} \cdot x_{cp}
$$

#### 2. Company Assignment

$$
Assignment^{Company}_{c} = \sum_{p \in P} x_{cp}, \; \forall c \in C
$$

#### 3. Product Assignment

$$
Assignment^{Product}_{p} = \sum_{p \in P} x_{cp}, \; \forall p \in P
$$

### Objective Function

#### 1. Minimize Total Cost

$$
min \quad Cost
$$

### Constraints

#### 1. Company Assignment Limit

$$
s.t. \quad Assignment^{Company}_{c} \leq 1, \; \forall c \in C
$$

#### 2. Product Assignment Limit

$$
s.t. \quad Assignment^{Product}_{p} = 1, \; \forall p \in P
$$

## Expected Result

Company A produces product C, company B produces product D, company C produces product A, and company D produces product B.

## Code Implementation

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data class Company(
    val cost: Map<Product, Flt64>
) : AutoIndexed(Company::class)

private val products = listOf(Product(), Product(), Product(), Product())
private val companies = listOf(
    Company(
        mapOf(
            products[0] to Flt64(920.0),
            products[1] to Flt64(480.0),
            products[2] to Flt64(650.0),
            products[3] to Flt64(340.0)
        )
    ),
    Company(
        mapOf(
            products[0] to Flt64(870.0),
            products[1] to Flt64(510.0),
            products[2] to Flt64(700.0),
            products[3] to Flt64(350.0)
        )
    ),
    Company(
        mapOf(
            products[0] to Flt64(880.0),
            products[1] to Flt64(500.0),
            products[2] to Flt64(720.0),
            products[3] to Flt64(400.0)
        )
    ),
    Company(
        mapOf(
            products[0] to Flt64(930.0),
            products[1] to Flt64(490.0),
            products[2] to Flt64(680.0),
            products[3] to Flt64(410.0)
        )
    )
)

// create a model instance
val metaModel = LinearMetaModel("demo2")

// define variables
x = BinVariable2("x", Shape2(companies.size, products.size))
for (c in companies) {
    for (p in products) {
        x[c, p].name = "${x.name}_${c.index},${p.index}"
    }
}
metaModel.add(x)

// define intermediate expressions
cost = LinearExpressionSymbol(flatSum(companies) { c ->
    products.map { p ->
        c.cost[p]?.let { it * x[c, p] }
    }
}, "cost")
metaModel.add(cost)

assignmentCompany = LinearIntermediateSymbols(
    "assignment_company",
    Shape1(companies.size)
)
for (c in companies) {
    assignmentCompany[c].asMutable() += sumVars(products) { p -> 
        c.cost[p]?.let { x[c, p] } 
    }
}
metaModel.add(assignmentCompany)

assignmentProduct = flatMap(
    "assignment_product",
    products,
    { p -> sumVars(companies) { c -> c.cost[p]?.let { x[c, p] } } }
)
metaModel.add(assignmentProduct)

// define objective function
metaModel.minimize(cost)

// define constraints
for (c in companies) {
    metaModel.addConstraint(assignmentCompany[c] leq 1)
}
for (p in products) {
    metaModel.addConstraint(assignmentProduct[p] eq 1)
}

// solve the model
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// parse results
val solution = ArrayList<Pair<Company, Product>>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one
        && token.variable.belongsTo(x)
    ) {
        val company = companies[token.variable.vectorView[0]]
        val product = products[token.variable.vectorView[1]]
        solution.add(company to product)
    }
}

```

:::

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo2.kt)
