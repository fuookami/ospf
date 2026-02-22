# Example 2: Assignment Problem

## Problem Description

There are several enterprises and several products. Each enterprise incurs a certain cost when producing each product.

|        | Product A | Product B | Product C | Product D |
| :----: | :-------: | :-------: | :-------: | :-------: |
| Enterprise A | $920$  | $480$  | $650$  | $340$  |
| Enterprise B | $870$  | $510$  | $700$  | $350$  |
| Enterprise C | $880$  | $500$  | $720$  | $400$  |
| Enterprise D | $930$  | $490$  | $680$  | $410$  |

The objective is to assign different enterprises to produce different products, minimizing the total cost.

## Mathematical Model

### Variables

$x_{cp}$: indicates whether enterprise $c$ is assigned to produce product $p$.

### Intermediate Values

#### 1. Total Cost

$$
\text{Cost} = \sum_{c \in C} \sum_{p \in P} \text{Cost}_{cp} \cdot x_{cp}
$$

#### 2. Whether an Enterprise is Assigned

$$
\text{Assignment}^{\text{Company}}_{c} = \sum_{p \in P} x_{cp}, \; \forall c \in C
$$

#### 3. Whether a Product is Assigned

$$
\text{Assignment}^{\text{Product}}_{p} = \sum_{c \in C} x_{cp}, \; \forall p \in P
$$

### Objective Function

#### 1. Minimize Total Cost

$$
\min \quad \text{Cost}
$$

### Constraints

#### 1. Each Enterprise Produces at Most One Product

$$
\text{s.t.} \quad \text{Assignment}^{\text{Company}}_{c} \leq 1, \; \forall c \in C
$$

#### 2. Each Product Must Be Produced

$$
\text{s.t.} \quad \text{Assignment}^{\text{Product}}_{p} = 1, \; \forall p \in P
$$

## Expected Results

Enterprise A produces Product C, Enterprise B produces Product D, Enterprise C produces Product A, Enterprise D produces Product B.

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

class Product : AutoIndexed(Product::class)

data class Company(
    val cost: Map<Product, Flt64>
) : AutoIndexed(Company::class)

val products: List<Product> = ...  // Product list
val companies: List<Company> = ... // Company list

// Create model instance
val metaModel = LinearMetaModel("demo2")

// Define variables
val x = BinVariable2("x", Shape2(companies.size, products.size))
for (c in companies) {
    for (p in products) {
        x[c, p].name = "${x.name}_${c.index},${p.index}"
    }
}
metaModel.add(x)

// Define intermediate values
val cost = LinearExpressionSymbol(flatSum(companies) { c ->
    products.map { p ->
        c.cost[p]?.let { it * x[c, p] }
    }
}, "cost")
metaModel.add(cost)

val assignmentCompany = LinearIntermediateSymbols(
    "assignment_company",
    Shape1(companies.size)
)
for (c in companies) {
    assignmentCompany[c].asMutable() += sumVars(products) { p -> 
        c.cost[p]?.let { x[c, p] } 
    }
}
metaModel.add(assignmentCompany)

val assignmentProduct = flatMap(
    "assignment_product",
    products,
    { p -> sumVars(companies) { c -> c.cost[p]?.let { x[c, p] } } }
)
metaModel.add(assignmentProduct)

// Define objective function
metaModel.minimize(cost)

// Define constraints
for (c in companies) {
    metaModel.addConstraint(assignmentCompany[c] leq 1)
}
for (p in products) {
    metaModel.addConstraint(assignmentProduct[p] eq 1)
}

// Invoke solver
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution = ArrayList<Pair<Company, Product>>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        val company = companies[token.variable.vectorView[0]]
        val product = products[token.variable.vectorView[1]]
        solution.add(company to product)
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo2.kt)
