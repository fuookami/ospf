# 示例 2: 指派问题

## 问题描述

有一些企业与一些产品，每个企业生产每个产品有一定的成本。

|       | 产品A | 产品B | 产品C | 产品D |
| :---: | :---: | :---: | :---: | :---: |
| 企业A | $920$ | $480$ | $650$ | $340$ |
| 企业B | $870$ | $510$ | $700$ | $350$ |
| 企业C | $880$ | $500$ | $720$ | $400$ |
| 企业D | $930$ | $490$ | $680$ | $410$ |

从这些企业中指定不同企业生产不同的产品，令总成本最小。

## 数学模型

### 变量

$x_{cp}$ ：选择企业 $c$ 生产产品 $p$ 。

### 中间值

#### 总成本

$$
Cost = \sum_{c \in C}\sum_{p \in P}Cost_{cp} \cdot x_{cp}
$$

#### 企业是否被指派

$$
Assignment^{Company}_{c} = \sum_{p \in P}x_{cp}, \; \forall c \in C
$$

#### 产品是否被指派

$$
Assignment^{Product}_{p} = \sum_{p \in P}x_{cp}, \; \forall p \in P
$$

### 目标函数

#### 总成本最小

$$
min \quad Cost
$$

### 约束

#### 每个企业最多生产一个产品

$$
s.t. \quad Assignment^{Company}_{c} \leq 1, \; \forall c \in C
$$

#### 每个产品必须要被生产

$$
s.t. \quad Assignment^{Product}_{p} = 1, \; \forall p \in P
$$

## 期望结果

企业 A 生产产品 C ，企业 B 生产产品 D ，企业 C 生产产品 A ，企业 D 生产产品 B 。

## 代码实现

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

// 创建模型实例
val metaModel = LinearMetaModel("demo2")

// 定义变量
x = BinVariable2("x", Shape2(companies.size, products.size))
for (c in companies) {
    for (p in products) {
        x[c, p].name = "${x.name}_${c.index},${p.index}"
    }
}
metaModel.add(x)

// 定义中间值
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

// 定义目标函数
metaModel.minimize(cost)

// 定义约束
for (c in companies) {
    metaModel.addConstraint(assignmentCompany[c] leq 1)
}
for (p in products) {
    metaModel.addConstraint(assignmentProduct[p] eq 1)
}

// 调用求解器求解
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// 解析结果
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

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo2.kt)
