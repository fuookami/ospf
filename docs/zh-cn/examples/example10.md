# 示例 10：带 MTZ 顺序约束的旅行商问题

## 1. 概览

本有界上下文以北京为锚点，在五座城市中选择一条闭合路线并恰好访问每座城市一次，使用 Miller–Tucker–Zemlin（MTZ）顺序变量消除子环，同时最小化有向旅行距离。Kotlin 与 Rust 使用独立的建模 API，但数据与数学模型相同。

源码城市列表为上海、合肥、广州、成都和北京，有向距离表如下：

| 出发地 | 上海 | 合肥 | 广州 | 成都 | 北京 |
| :---: | ---: | ---: | ---: | ---: | ---: |
| 上海 | -- | 472 | 1520 | 2095 | 1244 |
| 合肥 | 472 | -- | 1257 | 1615 | 1044 |
| 广州 | 1529 | 1257 | -- | 1954 | 2174 |
| 成都 | 2095 | 1615 | 1954 | -- | 1854 |
| 北京 | 1244 | 1044 | 2174 | 1854 | -- |

该矩阵是有向的：广州到上海为 1529，而反向为 1520。

### 1. 依赖上下文

1. 无。路线模型是自包含的。

---

## 2. 概念 / 实体

### 1. 城市（City）

城市是必须恰好有一条选中出弧和一条选中入弧的路线节点。

**$C$**：城市类别；样例包含上海、合肥、广州、成都和北京。

**$b$**：锚定的起始城市，即北京。

### 2. 有向弧（Directed Arc）

有向弧连接两个不同城市，并具有源码中的距离。

**$d_{ij}$**：城市 $i$ 到城市 $j$ 的距离；源码表是有向的，因此 $d_{ij}$ 与 $d_{ji}$ 可能不同。

---

## 3. 变量

### 1. 决策变量

**$x_{ij}$**：弧选择二元变量，无量纲，定义域为 ${0,1}$，表示是否使用 $i\to j$，$\forall(i,j)\in A$，其中 $A=\{(i,j)\in C^2\mid i\ne j\}$。

**$u_i$**：MTZ 顺序/势变量，无量纲，对 $i\in C\setminus\{b\}$ 的定义域为 $[-|C|,|C|]\cap\mathbb{Z}$；源码固定 $u_b=0$。它不是孤立子环指示变量。

### 2. 辅助变量

`Depart_i`、`Reached_i` 和 `Distance` 是由 $x$ 派生的中间/目标符号。模型没有另外创建二元子环指示变量。

---

## 4. 谓词

### 1. 弧谓词

**`DistinctArc(i,j)`**：当 $i\ne j$ 时为真；只有这些弧可以被选择。

**`NonAnchor(i)`**：当 $i\in C\setminus\{b\}$ 时为真；MTZ 成对约束只对非锚点城市生成。

---

## 5. 集合

### 1. 城市类别

**$C$**：城市全集。

**$C^N$**：非锚点城市集合，$C^N=C\setminus\{b\}$。

**$A$**：有向非自环弧集合，$A=\{(i,j)\in C\times C\mid i\ne j\}$。

### 2. 实体对 / 关系

**$TourArc$**：关系 $A$，连接出发城市和到达城市。

**$MTZPair$**：满足 $(i,j)\in C^N\times C^N$ 且 $i\ne j$ 的有序城市对。

---

## 6. 中间值

### 1. 每城出发数

**说明**：从城市 $i$ 出发的选中弧数量。

$$
Depart_i=\sum_{j:(i,j)\in A}x_{ij},\qquad \forall i\in C。
$$

### 2. 每城到达数

**说明**：进入城市 $i$ 的选中弧数量。

$$
Reached_i=\sum_{j:(j,i)\in A}x_{ji},\qquad \forall i\in C。
$$

### 3. 总距离

**说明**：所有选中弧的有向距离之和；Kotlin 排除对角项，Rust 将对角项固定为零。

$$
Distance=\sum_{(i,j)\in A}d_{ij}x_{ij}。
$$

### 4. MTZ 左侧表达式

**说明**：非锚点城市对使用的表达式，用于阻止不包含北京的选中环路。

$$
MTZ_{ij}=u_i-u_j+|C|x_{ij},\qquad \forall(i,j)\in MTZPair。
$$

---

## 7. 断言

### 1. 一进一出路线度数

**说明**：每个可行解中，每座城市恰好有一条选中出弧和一条选中入弧。

$$
\forall i\in C\;(Depart_i=1\wedge Reached_i=1)。
$$

### 2. 选中弧使顺序增加

**说明**：对于选中的非锚点弧，MTZ 约束迫使目的城市的顺序至少比起点大一。

$$
\forall(i,j)\in MTZPair\;(x_{ij}=1\Longrightarrow u_j\ge u_i+1)。
$$

### 3. 单一锚定路线

**说明**：度数等式、固定锚点 $u_b=0$ 及 MTZ 成对不等式排除不包含北京的环，留下一个经过所有城市的哈密顿环。

$$
\text{可行路线}\Longrightarrow\text{包含全部城市及 }b\text{ 的单一闭环}。
$$

### 4. 数值结果范围

**说明**：源码把选中弧提取为城市到城市的映射，而当前结构测试只检查模型构建。因此本文不声称某个固定的路线方向或距离数值。

---

## 8. 约束

### 1. No Self-Arc（禁止自环约束）

**说明**：城市不能前往自身。Kotlin 将对角项固定为 false 且不注册，Rust 注册对角项但设为固定零范围。

$$
s.t. \quad x_{ii}=0，\qquad \forall i\in C。
$$

### 2. One Departure per City（每城一条出弧约束）

**说明**：每座城市恰好选择一条出弧。

$$
s.t. \quad Depart_i=1，\qquad \forall i\in C。
$$

### 3. One Arrival per City（每城一条入弧约束）

**说明**：每座城市恰好选择一条入弧。

$$
s.t. \quad Reached_i=1，\qquad \forall i\in C。
$$

### 4. MTZ Subtour Elimination（MTZ 子环消除约束）

**说明**：非锚点城市之间的选中弧必须增加顺序；未选中的弧使用放松后的不等式。

$$
s.t. \quad u_i-u_j+|C|x_{ij}\le |C|-1，\qquad \forall(i,j)\in MTZPair。
$$

### 5. Order Domain and Anchor（顺序变量定义域与锚点约束）

**说明**：非锚点顺序变量使用源码显式范围，北京固定为零。

$$
s.t. \quad -|C|\le u_i\le |C|，\quad u_i\in\mathbb{Z},\ \forall i\in C^N；\qquad u_b=0。
$$

---

## 9. 目标函数（如适用）

**说明**：最小化选中闭合路线的有向距离。

$$
\min Distance。
$$

---

## 10. 算法引用

MTZ 是在本文中直接引用的标准子环消除公式；当前示例上下文不需要独立算法文件。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| Miller–Tucker–Zemlin（MTZ） | 本文第 8.4 节 | 第 8.4 节 | 使用顺序势变量排除非锚点子环。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 城市 | $i,j\in C$ | 路线节点。 |
| 路线弧 | $x_{ij}$ | 从 $i$ 到 $j$ 的二元旅行决策。 |
| 出发数 | $Depart_i$ | 城市 $i$ 的选中出弧数量。 |
| 到达数 | $Reached_i$ | 城市 $i$ 的选中入弧数量。 |
| 顺序势 | $u_i$ | 用于消除子环的 MTZ 变量。 |
| 锚点 | $b$ | 北京，路线的参考城市。 |

---

## 12. 设计决策

| 决策 | 替代方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用有向距离 | 将距离表对称化 | 源码中广州到上海为 1529，反向为 1520 | 2026-09-08 |
| 使用北京作为顺序锚点 | 让所有顺序变量自由变化 | 当前源码把北京项固定为零 | 2026-09-08 |
| 保留两端注册差异 | 声称 Kotlin/Rust 注册细节完全相同 | Kotlin 省略对角/自由锚点注册，Rust 注册固定范围 | 2026-09-08 |

### 当前最小模型构建片段

城市列表和距离表来自 `Demo10.kt`/`demo10.rs`；下面代码同时展示变量、中间表达式、目标和约束，数据设置由源码提供并省略。

::: code-group
```kotlin [Kotlin]
// `cities`、`beginCity`、`distances` 和 `flt64Converter` 来自 Demo10.kt。
val metaModel = LinearMetaModel<Flt64>("demo10", converter = flt64Converter)
val x = BinVariable2("x", Shape2(cities.size, cities.size))
for (city1 in cities) for (city2 in cities) {
    if (city1 != city2) metaModel.add(x[city1, city2])
    else x[city1, city2].range.eq(false)
}
val u = IntVariable1("u", Shape1(cities.size))
for (city in cities) {
    if (city.name == beginCity) {
        u[city].range.eq(Int64.zero)
    } else {
        u[city].range.set(ValueRange(
            Int64(-cities.size.toLong()), Int64(cities.size.toLong())
        ).value!!)
        metaModel.add(u[city])
    }
}
val distance = LinearExpressionSymbol(
    sum(cities.flatMap { i -> cities.mapNotNull { j ->
        if (i == j) null else distances[i to j]?.let { it * x[i, j] }
    }}), name = "distance"
)
val depart = LinearIntermediateSymbols1<Flt64>("depart", Shape1(cities.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[cities[i], _a]), name = "depart_$i")
}
val reached = LinearIntermediateSymbols1<Flt64>("reached", Shape1(cities.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, cities[i]]), name = "reached_$i")
}
metaModel.add(distance); metaModel.add(depart); metaModel.add(reached)
metaModel.minimize(distance, "distance")
for (city in cities) {
    metaModel.addConstraint(depart[city] eq Flt64.one)
    metaModel.addConstraint(reached[city] eq Flt64.one)
}
for (i in cities.filter { it.name != beginCity }) for (j in cities.filter { it.name != beginCity }) {
    if (i != j) metaModel.addConstraint(
        u[i] - u[j] + Flt64(cities.size.toDouble()) * x[i, j]
            leq Flt64((cities.size - 1).toDouble())
    )
}
```

```rust [Rust]
// `data` 来自 demo10.rs 中的 TspData::sample()。
let city_count = data.cities.len();
let mut model = MetaModel::<f64>::new("demo10");
let x_vars: VariableCombination2D<Binary> =
    VariableCombination2D::with_name_and_range_generator(
    Shape::new([city_count, city_count]), "x",
    |_index, vector| format!("{}_{}", vector[0], vector[1]),
    |_index, vector| if vector[0] == vector[1] {
        VariableRange::fixed(0.0)
    } else { VariableRange::bounded(0.0, 1.0) },
);
let x_idx = model.register_combination(&x_vars)?;
let u_vars: VariableCombination1D<Integer> =
    VariableCombination1D::with_name_and_range_generator(
    Shape::new([city_count]), "u", |_index, vector| vector[0].to_string(),
    |_index, vector| if vector[0] == data.begin_idx {
        VariableRange::fixed(0.0)
    } else { VariableRange::bounded(-(city_count as f64), city_count as f64) },
);
let u_idx = model.register_combination(&u_vars)?;
let distance = flat_map1_indexed("distance", &data.cities, |i, _| {
    let monomials = (0..city_count).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(
            data.distances.get(i, j), x_idx[&[i, j]],
        )
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, city| city.name.clone());
model.add_symbol_combination(&distance)?;
let depart = flat_map1_indexed("depart", &data.cities, |i, _| {
    let monomials = (0..city_count).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, city| city.name.clone());
let reached = flat_map1_indexed("reached", &data.cities, |j, _| {
    let monomials = (0..city_count).map(|i|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, city| city.name.clone());
let mtz = flat_map2_indexed("mtz", &data.cities, &data.cities, |i, _, j, _| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, u_idx[&[i]]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, u_idx[&[j]]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(city_count as f64, x_idx[&[i, j]]),
    ], 0.0)
}, |_, city_i, _, city_j| format!("{}_{}", city_i.name, city_j.name));
model.add_symbol_combination(&depart)?;
model.add_symbol_combination(&reached)?;
model.add_symbol_combination(&mtz)?;
let mut dist_coeffs = Vec::new();
for i in 0..city_count {
    for monomial in distance.symbol_polynomial(i).monomials() {
        dist_coeffs.push((monomial.var_index(), *monomial.coefficient()));
    }
}
model.set_linear_objective_input(
    LinearObjectiveInput::minimize("distance").terms(dist_coeffs.into_iter())
);
for i in 0..city_count {
    model.add_linear_constraint(&extract_coeffs(&depart[i]), ConstraintRelation::Equal, 1.0,
        &format!("depart_{}", i))?;
    model.add_linear_constraint(&extract_coeffs(&reached[i]), ConstraintRelation::Equal, 1.0,
        &format!("arrive_{}", i))?;
}
for i in 0..city_count {
    if i == data.begin_idx { continue; }
    for j in 0..city_count {
        if j == data.begin_idx || i == j { continue; }
        model.add_linear_constraint(&extract_coeffs(&mtz[&[i, j]]),
            ConstraintRelation::LessEqual, city_count as f64 - 1.0,
            &format!("mtz_{}_{}", i, j))?;
    }
}
```
:::

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 2026-09-08 | 按领域模型模板重组页面；澄清 MTZ 语义、有向数据、带量词约束及 Kotlin/Rust 标签页 | 对齐当前 Demo10 实现，同时不声称测试固定了某条数值路线 |

## 源码与验证

- [Kotlin 实现：`Demo10.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo10.kt)
- [Rust 对照实现：`demo10.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo10.rs)
- [Kotlin 结构测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
