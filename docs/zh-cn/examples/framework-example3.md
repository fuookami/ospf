# 复杂示例 3：一维分切问题

## 问题描述

原材料长 $1000$ 个单位，需要把原材料切分以下的成品材料，且满足每种成品材料对应的需求量，目标是最小化所需原材料的数量：

|        | 长度  | 需求量 |
| :----: | :---: | :----: |
| 成品 1 | $450$ |  $97$  |
| 成本 2 | $360$ | $610$  |
| 成品 3 | $310$ | $395$  |
| 成品 4 | $140$ | $211$  |

## 数学模型

### RMP

#### 变量

$x_{ij} \in N$：使用第 $i$ 次迭代第 $j$ 个切割方案的数量。

#### 中间值

##### 1. 原材料总使用量

$$
Cost_{i} = Cost_{i - 1} + \sum_{j \in N_{i}} x_{ij}, \; \forall j \in N_{i}
$$

##### 2. 成品材料生产量

$$
Output_{ip} = Output_{i - 1, \, p} + \sum_{j \in N_{i}} Amount_{ijp} \cdot x_{ij}, \; \forall j \in N_{i}, \; \forall p \in P
$$

#### 目标函数

##### 1. 原材料使用量最小

$$
Min \quad Cost_{i}
$$

#### 约束

##### 1. 成品材料生产量要满足需求量

$$
s.t. \quad Output_{ip} \geq Demand_{p}, \; \forall p \in P
$$

### SP

#### 变量

$y_{p} \in N$：切割 $p$ 成品材料的数量。

#### 中间值

##### 1. 原材料使用量

$$
Use = \sum_{p \in P} l_{p} \cdot y_{p}
$$

#### 目标函数

##### 1. Reduced Cost 最小

$$
Min \quad 1 - \sum_{p \in P} \lambda_{p} \cdot y_{p}
$$

#### 约束

##### 1. 原材料使用量不能大于原材料的长度

$$
s.t. \quad Use \leq L
$$

## 代码实现

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

    // 初始化主问题模型
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

    // 添加列（切割方案）
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

    // 添加列（切割方案）
    fun addColumns(cuttingPlans: List<CuttingPlan>) {
        for (cuttingPlan in cuttingPlans) {
            addColumn(cuttingPlan, false)
        }
        metaModel.flush()
    }

    // 求解线性松弛模型
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

    // 求解混合整数模型
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

    // 提炼影子价格（对偶值）表
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

// 初始解生成器
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

    // 求解子问题
    suspend operator fun invoke(
        iteration: UInt64,
        length: UInt64,
        products: List<Product>,
        shadowPrice: ShadowPriceMap
    ): Ret<CuttingPlan> {
        // 构建子问题模型实例
        val model = LinearMetaModel("demo1-sp-$iteration")

        // 定义变量
        val y = UIntVariable1("y", Shape1(products.size))
        for (product in products) {
            y[product].name = "${y.name}_${product.index}"
        }
        model.add(y)

        // 定义中间值
        val use = LinearExpressionSymbol(sum(products) { p -> p.length * y[p] }, "use")
        model.add(use)

        // 定义目标函数和约束
        model.minimize(Flt64.one - sum(products) { p -> shadowPrice(p) * y[p] })
        model.addConstraint(use leq length, "use")

        // 求解并解析
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

### 应用

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*

class CSP {
    private val length = UInt64(1000UL)
    private val products: List<Product> = ... // 产品列表

    suspend operator fun invoke(): Try {
        // 生成初始解
        val initialCuttingPlans = InitialSolutionGenerator(length, products)
        when (initialCuttingPlans) {
            is Failed -> {
                return Failed(initialCuttingPlans.error)
            }

            is Ok -> {}
        }
        // 初始化主问题
        val rmp = RMP(length, products, initialCuttingPlans.value)
        val sp = SP()
        var i = UInt64.zero
        while (true) {
            // 求解线性松弛主问题获取影子价格表
            val spm = rmp(i)
            when (spm) {
                is Failed -> {
                    return Failed(spm.error)
                }

                is Ok -> {}
            }
            // 求解子问题生成新的列
            val newCuttingPlan = sp(i, length, products, spm.value)
            when (newCuttingPlan) {
                is Failed -> {
                    return Failed(newCuttingPlan.error)
                }

                is Ok -> {}
            }
            // 如果生成的列并不能优化主问题，则停止迭代
            if (reducedCost(newCuttingPlan.value, spm.value) geq Flt64.zero
                || !rmp.addColumn(newCuttingPlan.value)
            ) {
                break
            }
            ++i
        }
        // 最终求解整数解
        when (val solution = rmp()) {
            is Ok -> {}

            is Failed -> {
                return Failed(solution.error)
            }
        }
        return ok
    }

    // 计算切割方案的 Reduced Cost
    private fun reducedCost(cuttingPlan: CuttingPlan, shadowPrices: ShadowPriceMap) = Flt64.one -
            cuttingPlan.products.sumOf { (shadowPrices(it.key) * it.value.toFlt64()) }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo3)
