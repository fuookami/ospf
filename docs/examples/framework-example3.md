# Complex Example 3: One-Dimensional Cutting Problem

## Problem Description

Raw material length is $1000$ units. The raw material needs to be cut into the following finished products to meet the corresponding demand for each finished product. The objective is to minimize the number of raw materials required:

|        | Length | Demand |
| :----: | :---: | :----: |
| Product 1 | $450$ |  $97$  |
| Product 2 | $360$ | $610$  |
| Product 3 | $310$ | $395$  |
| Product 4 | $140$ | $211$  |

## Mathematical Model

### RMP (Restricted Master Problem)

#### Variables

$x_{ij} \in \mathbb{N}$: Number of times to use the $j$-th cutting plan in the $i$-th iteration.

#### Intermediate Values

##### 1. Total Raw Material Usage

$$
\text{Cost}_{i} = \text{Cost}_{i - 1} + \sum_{j \in N_{i}} x_{ij}, \; \forall j \in N_{i}
$$

##### 2. Finished Product Production Quantity

$$
\text{Output}_{ip} = \text{Output}_{i - 1, \, p} + \sum_{j \in N_{i}} \text{Amount}_{ijp} \cdot x_{ij}, \; \forall j \in N_{i}, \; \forall p \in P
$$

#### Objective Function

##### 1. Minimize Raw Material Usage

$$
\min \quad \text{Cost}_{i}
$$

#### Constraints

##### 1. Finished Product Production Must Meet Demand

$$
\text{s.t.} \quad \text{Output}_{ip} \geq \text{Demand}_{p}, \; \forall p \in P
$$

### SP (Subproblem)

#### Variables

$y_{p} \in \mathbb{N}$: Number of times to cut product $p$.

#### Intermediate Values

##### 1. Raw Material Usage

$$
\text{Use} = \sum_{p \in P} l_{p} \cdot y_{p}
$$

#### Objective Function

##### 1. Minimize Reduced Cost

$$
\min \quad 1 - \sum_{p \in P} \lambda_{p} \cdot y_{p}
$$

#### Constraints

##### 1. Raw Material Usage Cannot Exceed Raw Material Length

$$
\text{s.t.} \quad \text{Use} \leq L
$$

## Code Implementation

### Domain

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.framework.model.*

data class Product(
    val length: UInt64,
    val demand: UInt64
) : AutoIndexed(Product::class)

data class CuttingPlan(
    val products: Map<Product, UInt64>
) : AutoIndexed(CuttingPlan::class)

class ShadowPriceMap : AbstractShadowPriceMap<Product, ShadowPriceMap>()
```

:::

### RMP

::: code-group

```kotlin
import java.util.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.framework.solver.*

data class ProductDemandShadowPriceKey(
    val product: Product
) : ShadowPriceKey(ProductDemandShadowPriceKey::class)

class RMP(
    private val length: UInt64,
    private val products: List<Product>,
    initialCuttingPlans: List<CuttingPlan>
) {
    private val cuttingPlans: MutableList<CuttingPlan> = ArrayList()
    private val x: MutableList<UIntVar> = ArrayList()
    private val rest = LinearExpressionSymbol(MutableLinearPolynomial(), "rest")
    private val yield = LinearExpressionSymbols1("output", Shape1(products.size)) { _, v ->
        LinearExpressionSymbol(MutableLinearPolynomial(), "output_${v[0]}")
    }
    private val metaModel = LinearMetaModel("demo1")
    private val solver: ColumnGenerationSolver = GurobiColumnGenerationSolver()

    // Initialize master problem model
    init {
        metaModel.add(rest)
        metaModel.add(yield)

        metaModel.minimize(rest)
        metaModel.registerConstraintGroup("product_demand")

        for (product in products) {
            metaModel.addConstraint(yield[product] geq product.demand, "product_demand_${product.index}")
        }

        addColumns(initialCuttingPlans)
    }

    // Add column (cutting plan)
    fun addColumn(cuttingPlan: CuttingPlan, flush: Boolean = true): Boolean {
        if (cuttingPlans.find { it.products == cuttingPlan.products } != null) {
            return false
        }

        cuttingPlans.add(cuttingPlan)
        val x = UIntVar("x_${cuttingPlan.index}")
        x.range.leq(cuttingPlan.products.maxOf { (product, amount) -> product.demand / amount + UInt64.one })
        this.x.add(x)
        metaModel.add(x)

        rest.asMutable() += (length - cuttingPlan.products.sumOf { it.key.length * it.value }) * x
        rest.flush()
        for ((product, amount) in cuttingPlan.products) {
            yield[product].asMutable() += amount * x
            yield[product].flush()
        }
        if (flush) {
            metaModel.flush()
        }
        return true
    }

    // Add columns (cutting plans)
    fun addColumns(cuttingPlans: List<CuttingPlan>) {
        for (cuttingPlan in cuttingPlans) {
            addColumn(cuttingPlan, false)
        }
        metaModel.flush()
    }

    // Solve linear relaxation model
    suspend operator fun invoke(iteration: UInt64): Ret<ShadowPriceMap> {
        return when (val result = solver.solveLP("demo1-rmp-$iteration", metaModel)) {
            is Ok -> {
                Ok(extractShadowPriceMap(result.value.dualSolution))
            }

            is Failed -> {
                Failed(result.error)
            }
        }
    }

    // Solve mixed integer model
    suspend operator fun invoke(): Ret<Map<CuttingPlan, UInt64>> {
        return when (val result = solver.solveMILP("demo1-rmp-ip", metaModel)) {
            is Ok -> {
                Ok(analyzeSolution(result.value.solution))
            }

            is Failed -> {
                Failed(result.error)
            }
        }
    }

    // Extract shadow price (dual value) map
    private fun extractShadowPriceMap(dualResult: List<Flt64>): ShadowPriceMap {
        val ret = ShadowPriceMap()

        for ((i, j) in metaModel.indicesOfConstraintGroup("product_demand")!!.withIndex()) {
            ret.put(ShadowPrice(ProductDemandShadowPriceKey(products[i]), dualResult[j]))
        }
        ret.put { map, args ->
            map.map[ProductDemandShadowPriceKey(args)]?.price ?: Flt64.zero
        }

        return ret
    }

    private fun analyzeSolution(result: List<Flt64>): Map<CuttingPlan, UInt64> {
        ...
    }
}
```

:::

### SP

::: code-group

```kotlin
import java.util.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*
import fuookami.ospf.kotlin.framework.solver.*

// Initial solution generator
object InitialSolutionGenerator {
    operator fun invoke(length: UInt64, products: List<Product>): Ret<List<CuttingPlan>> {
        val solution = ArrayList<CuttingPlan>()
        for (product in products) {
            val amount = length / product.length
            solution.add(CuttingPlan(mapOf(Pair(product, amount))))
        }
        return Ok(solution)
    }
}

class SP {
    private val solver: ColumnGenerationSolver = ScipColumnGenerationSolver()

    // Solve subproblem
    suspend operator fun invoke(
        iteration: UInt64,
        length: UInt64,
        products: List<Product>,
        shadowPrice: ShadowPriceMap
    ): Ret<CuttingPlan> {
        // Build subproblem model instance
        val model = LinearMetaModel("demo1-sp-$iteration")

        // Define variables
        val y = UIntVariable1("y", Shape1(products.size))
        for (product in products) {
            y[product].name = "${y.name}_${product.index}"
        }
        model.add(y)

        // Define intermediate values
        val use = LinearExpressionSymbol(sum(products) { p -> p.length * y[p] }, "use")
        model.add(use)

        // Define objective function and constraints
        model.minimize(Flt64.one - sum(products) { p -> shadowPrice(p) * y[p] })
        model.addConstraint(use leq length, "use")

        // Solve and parse
        return when (val result = solver.solveMILP("demo1-sp-$iteration", model)) {
            is Ok -> {
                Ok(analyze(model, products, result.value.solution))
            }

            is Failed -> {
                Failed(result.error)
            }
        }
    }

    private fun analyze(model: LinearMetaModel, products: List<Product>, result: List<Flt64>): CuttingPlan {
        ...
    }
}
```

:::

### Application

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*

class CSP {
    private val length = UInt64(1000UL)
    private val products: List<Product> = ... // Product list

    suspend operator fun invoke(): Try {
        // Generate initial solution
        val initialCuttingPlans = InitialSolutionGenerator(length, products)
        when (initialCuttingPlans) {
            is Failed -> {
                return Failed(initialCuttingPlans.error)
            }

            is Ok -> {}
        }
        // Initialize master problem
        val rmp = RMP(length, products, initialCuttingPlans.value)
        val sp = SP()
        var i = UInt64.zero
        while (true) {
            // Solve linear relaxation master problem to obtain shadow price map
            val spm = rmp(i)
            when (spm) {
                is Failed -> {
                    return Failed(spm.error)
                }

                is Ok -> {}
            }
            // Solve subproblem to generate new column
            val newCuttingPlan = sp(i, length, products, spm.value)
            when (newCuttingPlan) {
                is Failed -> {
                    return Failed(newCuttingPlan.error)
                }

                is Ok -> {}
            }
            // If the generated column does not optimize the master problem, stop iteration
            if (reducedCost(newCuttingPlan.value, spm.value) geq Flt64.zero
                || !rmp.addColumn(newCuttingPlan.value)
            ) {
                break
            }
            ++i
        }
        // Finally solve integer solution
        when (val solution = rmp()) {
            is Ok -> {}

            is Failed -> {
                return Failed(solution.error)
            }
        }
        return ok
    }

    // Calculate reduced cost of cutting plan
    private fun reducedCost(cuttingPlan: CuttingPlan, shadowPrices: ShadowPriceMap) = Flt64.one -
            cuttingPlan.products.sumOf { (shadowPrices(it.key) * it.value.toFlt64()) }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo3)
