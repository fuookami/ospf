# 示例 9：选址问题

## 问题描述

在一个按照东西和南北方向划分成规整街区的城市里，居民点散乱地分布在不同的街区中。用 $x$ 坐标表示东西向，用 $y$ 坐标表示南北向。各居民点的位置可以由坐标 $(x, \, y)$ 表示。

为建邮局选址，使得居民点到邮局之距离的总和最小。距离使用曼哈顿距离。

|       | 居民点 A | 居民点 B | 居民点 C | 居民点 D | 居民点 E | 居民点 F |
| :---: | :------: | :------: | :------: | :------: | :------: | :------: |
|  $x$  | $9\,km$  | $2\,km$  | $3\,km$  | $3\,km$  | $5\,km$  | $4\,km$  |
|  $y$  | $2\,km$  | $1\,km$  | $8\,km$  | $-2\,km$ | $9\,km$  | $-2\,km$ |

## 数学模型

### 变量

$x, \, y$ ：邮局坐标。

### 中间值

#### 1. 东西向距离

$$
dx_{s} = Slack(x, \, x_{s}) = |x - x_{s}|, \; \forall s \in S
$$

#### 2. 南北向距离

$$
dy_{s} = Slack(y, \, y_{s}) = |y - y_{s}|, \; \forall s \in S
$$

#### 3. 距离

$$
Distance_{s} = dx_{s} + dy_{s}, \; \forall s \in S
$$

### 目标函数

#### 1. 距离之和最小

$$
min \quad \sum_{s \in S} Distance_{s}
$$

## 期望结果

邮局应该设置在 $(3, \, 1)$ 处。

## 代码实现

::: code-group

```kotlin
data class Settlement(
    val x: Flt64,
    val y: Flt64
) : AutoIndexed(Settlement::class)

val settlements = ... // 居民点列表

// 创建模型实例
val metaModel = LinearMetaModel("demo9")

// 定义变量
val x = IntVar("x")
val y = IntVar("y")
metaModel.add(x)
metaModel.add(y)

// 定义中间值
val dx = LinearIntermediateSymbols1("dx", Shape1(settlements.size)) { i, _ ->
    SlackFunction(
        type = UInteger,
        x = LinearPolynomial(x),
        y = LinearPolynomial(settlements[i].x),
        name = "dx_$i"
    )
}
metaModel.add(dx)

val dy = LinearIntermediateSymbols1("dy", Shape1(settlements.size)) { i, _ ->
    SlackFunction(
        type = UInteger,
        x = LinearPolynomial(y),
        y = LinearPolynomial(settlements[i].y),
        name = "dy_$i"
    )
}
metaModel.add(dy)

val distance = LinearIntermediateSymbols1("distance", Shape1(settlements.size)) { i, _ ->
    LinearExpressionSymbol(
        dx[i] + dy[i],
        name = "distance_$i"
    )
}
metaModel.add(distance)

// 定义目标函数
metaModel.minimize(sum(distance[_a]))

// 调用求解器求解
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// 解析结果
val position = ArrayList<Flt64>()
for (token in metaModel.tokens.tokens) {
    if (token.variable.belongsTo(x)) {
        position.add(token.result!!)
    }
}
for (token in metaModel.tokens.tokens) {
    if (token.variable.belongsTo(y)) {
        position.add(token.result!!)
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo9.kt)
