# 示例 10：旅行商问题

有一位商人，他想访问中国的某些城市：

|       |    上海    |    合肥    |    广州    |    成都    |    北京    |
| :---: | :--------: | :--------: | :--------: | :--------: | :--------: |
| 上海  |    $-$     | $472\,km$  | $1529\,km$ | $2095\,km$ | $1244\,km$ |
| 合肥  | $472\,km$  |    $-$     | $1257\,km$ | $1615\,km$ | $1044\,km$ |
| 广州  | $1529\,km$ | $1257\,km$ |    $-$     | $1954\,km$ | $2174\,km$ |
| 成都  | $2095\,km$ | $1615\,km$ | $1954\,km$ |    $-$     | $1854\,km$ |
| 北京  | $1244\,km$ | $1044\,km$ | $2174\,km$ | $1854\,km$ |    $-$     |

给出最小距离的路线，并满足以下条件：

1. 每个城市只能访问一次；
2. 从某城市出发，最后回到该城市。

## 数学模型

### 1. 变量

$x_{ij}$ ：从城市 $i$ 到城市 $j$ 。

$u_{i}$ ：城市 $i$ 是否出现孤立子圈的判定。

### 中间值

#### 1. 总距离

$$
Distance = \sum_{i \in C}\sum_{j \in C} Distrance_{ij} \cdot x_{ij}
$$

#### 2. 是否从某座城市出发过

$$
Depart_{i} = \sum_{j \in C} x_{ij}, \; \forall i \in C
$$

#### 3. 是否到达过某座城市

$$
Reached_{j} = \sum_{i \in C} x_{ij}, \; \forall j \in C
$$

### 目标函数

#### 1. 总距离最小

$$
min \quad Distance
$$

### 约束

#### 1. 必须从每个城市出发过

$$
s.t. \quad Depart_{i} = 1, \; \forall i \in C
$$

#### 2. 必须到达过每个城市

$$
s.t. \quad Reached_{j} = 1, \; \forall j \in C
$$

#### 3. 不可出现孤立子圈

$$
s.t. \quad u_{i} - u_{j} + |C| \cdot x_{ij} \leq |C| - 1, \; \forall (i, \, j) \in ((C - C^{Begin})^{2} - \Delta (C - C^{Begin}))
$$

## 期望结果

北京 -> 上海 -> 合肥 -> 广州 -> 成都 -> 北京。

## 代码实现

::: code-group

```kotlin
data class City(
    val name: String
) : AutoIndexed(City::class)

val beginCity = "北京"
val cities = ... \\ 城市列表
val distances = ... \\ 距离矩阵

// 创建模型实例
val metaModel = LinearMetaModel("demo10")

// 创建变量
val x = BinVariable2("x", Shape2(cities.size, cities.size))
for (city1 in cities) {
    for (city2 in cities) {
        val xi = x[city1, city2]
        xi.name = "${x.name}_(${city1.name},${city2.name})"
        if (city1 != city2) {
            metaModel.add(xi)
        } else {
            xi.range.eq(false)
        }
    }
}

val u = IntVariable1("u", Shape1(cities.size))
for (city in cities) {
    val ui = u[city]
     ui.name = "${u.name}_${city.name}"
    if (city.name != beginCity) {
        ui.range.set(ValueRange(Int64(-cities.size.toLong()), Int64(cities.size.toLong())).value!!)
        metaModel.add(ui)
     } else {
        ui.range.eq(Int64.zero)
    }
}

// 定义中间值
val distance = LinearExpressionSymbol(sum(cities.flatMap { city1 ->
    cities.mapNotNull { city2 ->
        if (city1 == city2) {
             null
        } else {
            distances[city1 to city2]?.let { it * x[city1, city2] }
        }
    }
}), "distance")

val depart = LinearIntermediateSymbols1("depart", Shape1(cities.size)) { i, _ ->
     val city = cities[i]
    LinearExpressionSymbol(sum(x[city, _a]), "depart_${city.name}")
}

val reached = LinearIntermediateSymbols1("reached", Shape1(cities.size)) { i, _ ->
    val city = cities[i]
    LinearExpressionSymbol(sum(x[_a, city]), "reached_${city.name}")
}

// 定义目标函数
metaModel.minimize(distance, "distance")

// 定义约束
for (city in cities) {
    metaModel.addConstraint(
        depart[city] eq Flt64.one,
        "depart_${city.name}"
    )
}

for (city in cities) {
    metaModel.addConstraint(
        reached[city] eq Flt64.one,
        "reached_${city.name}"
    )
}

val notBeginCities = cities.filter { it.name != beginCity }
for (city1 in notBeginCities) {
    for (city2 in notBeginCities) {
        if (city1 != city2) {
            metaModel.addConstraint(
                u[city1] - u[city2] + cities.size * x[city1, city2] leq cities.size - 1,
                "child_route_(${city1.name},${city2.name})"
            )
        }
     }
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
val route: MutableMap<City, City> = hashMapOf()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one 
        && token.variable.belongsTo(x)
    ) {
        val vector = token.variable.vectorView
        val city1 = cities[vector[0]]
        val city2 = cities[vector[1]]
        route[city1] = city2
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo10.kt)
