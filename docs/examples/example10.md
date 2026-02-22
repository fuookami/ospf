# Example 10: Traveling Salesman Problem

A merchant needs to visit several cities in China:

|       | Shanghai | Hefei | Guangzhou | Chengdu | Beijing |
| :---: | :------: | :---: | :-------: | :-----: | :-----: |
| Shanghai |    $-$     | $472\,km$  | $1529\,km$ | $2095\,km$ | $1244\,km$ |
| Hefei | $472\,km$  |    $-$     | $1257\,km$ | $1615\,km$ | $1044\,km$ |
| Guangzhou | $1529\,km$ | $1257\,km$ |    $-$     | $1954\,km$ | $2174\,km$ |
| Chengdu | $2095\,km$ | $1615\,km$ | $1954\,km$ |    $-$     | $1854\,km$ |
| Beijing | $1244\,km$ | $1044\,km$ | $2174\,km$ | $1854\,km$ |    $-$     |

Determine the route with minimum distance, while satisfying the following conditions:

1. Each city can only be visited once;
2. Start from a city and return to the same city.

## Mathematical Model

### Variables

$x_{ij}$: Travel from city $i$ to city $j$.

$u_{i}$: Determination of whether city $i$ appears in an isolated sub-cycle.

### Intermediate Values

#### 1. Total Distance

$$
\text{Distance} = \sum_{i \in C}\sum_{j \in C} \text{Distance}_{ij} \cdot x_{ij}
$$

#### 2. Whether Departed from a City

$$
\text{Depart}_{i} = \sum_{j \in C} x_{ij}, \; \forall i \in C
$$

#### 3. Whether Reached a City

$$
\text{Reached}_{j} = \sum_{i \in C} x_{ij}, \; \forall j \in C
$$

### Objective Function

#### 1. Minimize Total Distance

$$
\min \quad \text{Distance}
$$

### Constraints

#### 1. Must Depart from Each City

$$
\text{s.t.} \quad \text{Depart}_{i} = 1, \; \forall i \in C
$$

#### 2. Must Reach Each City

$$
\text{s.t.} \quad \text{Reached}_{j} = 1, \; \forall j \in C
$$

#### 3. No Isolated Sub-Cycles Allowed

$$
\text{s.t.} \quad u_{i} - u_{j} + |C| \cdot x_{ij} \leq |C| - 1, \; \forall (i, \, j) \in ((C - C^{\text{Begin}})^{2} - \Delta (C - C^{\text{Begin}}))
$$

## Expected Results

Beijing -> Shanghai -> Hefei -> Guangzhou -> Chengdu -> Beijing.

## Code Implementation

::: code-group

```kotlin
data class City(
    val name: String
) : AutoIndexed(City::class)

val beginCity = "Beijing"
val cities: List<City> = ... // City list
val distances: Map<Pair<City, City>, Flt64> = ... // Distance matrix

// Create model instance
val metaModel = LinearMetaModel("demo10")

// Create variables
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

// Define intermediate values
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

// Define objective function
metaModel.minimize(distance, "distance")

// Define constraints
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

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val route: MutableMap<City, City> = hashMapOf()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        val vector = token.variable.vectorView
        val city1 = cities[vector[0]]
        val city2 = cities[vector[1]]
        route[city1] = city2
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo10.kt)
