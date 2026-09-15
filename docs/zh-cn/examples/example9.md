# 示例 9：整数曼哈顿距离设施选址

## 1. 概览

本有界上下文选择一个整数设施坐标，使其到六个定居点的曼哈顿距离总和最小。Kotlin 与 Rust 使用不同的绝对值辅助 API，因此显式坐标边界也不同。

源码提供的定居点坐标如下：

| 定居点 | $X_s$ | $Y_s$ |
| :---: | ---: | ---: |
| 1 | 9 | 2 |
| 2 | 2 | 1 |
| 3 | 3 | 8 |
| 4 | 3 | -2 |
| 5 | 5 | 9 |
| 6 | 4 | -2 |

### 1. 依赖上下文

1. 无。该示例是自包含的选址模型。

---

## 2. 概念 / 实体

### 1. 定居点（Settlement）

定居点是坐标平面中的固定观测点。

**$X_s$**：定居点 $s$ 的观测 x 坐标。

**$Y_s$**：定居点 $s$ 的观测 y 坐标。

### 2. 设施（Facility）

设施是求解器选择的点。

**$X$**：设施的 x 坐标。

**$Y$**：设施的 y 坐标。

---

## 3. 变量

### 1. 决策变量

**$X,Y$**：设施坐标，单位为坐标单位，取整数。Kotlin 创建标量 `IntVar("x")` 与 `IntVar("y")`，没有显式边界；Rust 创建整数组合变量并限制 $-100\le X,Y\le100$。

### 2. 辅助变量

**$DX_s,DY_s$**：由 Kotlin 的 `exampleAbsoluteSlack` 或 Rust 的 `AbsFunction` 生成的非负绝对距离分量。它们是机制/函数输出，不是用户选择的决策变量。

---

## 4. 谓词

### 1. 定居点坐标谓词

**`Observed(s)`**：定居点 $s$ 在输入数据中具有固定坐标 $(X_s,Y_s)$。

### 2. 坐标边界谓词

**`RustBounded`**：Rust 实现中为真，因为它把两个坐标配置在 $[-100,100]$；当前 Kotlin 实现中为假。

---

## 5. 集合

### 1. 定居点类别

**$S$**：定居点全集，样例中 $S=\{1,\ldots,6\}$。

**$S^{Observed}$**：提供观测坐标的定居点子集；本样例中 $S^{Observed}=S$。

### 2. 坐标分量

**$D^X$**：按 $S$ 索引的 x 轴绝对距离分量集合。

**$D^Y$**：按 $S$ 索引的 y 轴绝对距离分量集合。

### 3. 实体对 / 关系

**$DistanceTo$**：将设施 $(X,Y)$ 与每个定居点 $s$ 配对，形成其曼哈顿距离的关系。

---

## 6. 中间值

### 1. x 轴绝对距离

**说明**：设施与定居点 $s$ 之间的非负水平距离。辅助函数会创建对应的线性化机制约束。

$$
DX_s=|X-X_s|，\qquad \forall s\in S。
$$

### 2. y 轴绝对距离

**说明**：设施与定居点 $s$ 之间的非负垂直距离。

$$
DY_s=|Y-Y_s|，\qquad \forall s\in S。
$$

### 3. 定居点距离

**说明**：设施到定居点 $s$ 的曼哈顿距离。

$$
Distance_s=DX_s+DY_s，\qquad \forall s\in S。
$$

### 4. 总距离

**说明**：到所有定居点的曼哈顿距离之和，也是模型最小化的表达式。

$$
TotalDistance=\sum_{s\in S}Distance_s。
$$

---

## 7. 断言

### 1. 绝对距离分量非负

**说明**：每个辅助函数输出表示绝对差值，不能为负。

$$
\forall s\in S\;(DX_s\ge0\wedge DY_s\ge0)。
$$

### 2. 曼哈顿距离分解

**说明**：每个定居点距离等于水平分量与垂直分量之和。

$$
\forall s\in S\;(Distance_s=|X-X_s|+|Y-Y_s|)。
$$

### 3. 样例最优解集合

**说明**：给定六个点时，按坐标取整数中位数可以刻画最优解。

$$
X\in\{3,4\},\qquad Y\in\{1,2\},\qquad TotalDistance=32。
$$

---

## 8. 约束

### 1. Facility Integer Domain（设施整数定义域约束）

**说明**：设施坐标为整数。Kotlin 没有显式地理边界；Rust 为绝对值机制额外添加边界。

$$
s.t. \quad X,Y\in\mathbb{Z}，
$$

Rust 额外满足

$$
s.t. \quad -100\le X\le100，\quad -100\le Y\le100。
$$

### 2. Absolute-Distance Linearization（绝对距离线性化约束）

**说明**：源码辅助函数生成的机制约束在语义上表达每个坐标差的绝对值。`Demo9.kt` 和 `demo9.rs` 都没有手写该用户层不等式。

$$
s.t. \quad DX_s=|X-X_s|，\quad DY_s=|Y-Y_s|，\qquad \forall s\in S。
$$

### 3. Distance Definition（距离定义约束）

**说明**：每个定居点的曼哈顿距离由两个分量相加得到。

$$
s.t. \quad Distance_s=DX_s+DY_s，\qquad \forall s\in S。
$$

---

## 9. 目标函数（如适用）

**说明**：最小化总曼哈顿距离。

$$
\min TotalDistance。
$$

---

## 10. 算法引用

本文引用了辅助机制，但这些辅助机制在当前示例上下文中没有独立算法文档。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| `exampleAbsoluteSlack` | `ospf-kotlin-example/src/main/.../ExampleModeling.kt` | 第 6.1–6.2 节 | Kotlin 对 `SlackFunction` 的适配器，同时包含正、负偏差。 |
| `AbsFunction` | `ospf-rust-example/src/core/demo9.rs` | 第 6.1–6.2 节 | Rust 绝对值函数，在模型转换期间生成机制约束。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 定居点 | $s\in S$ | 具有观测坐标的固定点。 |
| 设施 | $(X,Y)$ | 模型选择的整数点。 |
| 分量距离 | $DX_s,DY_s$ | 某一坐标轴上的绝对差值。 |
| 曼哈顿距离 | $Distance_s$ | 两个分量距离之和。 |

---

## 12. 设计决策

| 决策 | 替代方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用绝对值辅助函数 | 在每个示例中手写 Big-M 公式 | 当前 Kotlin 与 Rust API 会生成机制约束 | 2026-09-08 |
| 保持 Kotlin 坐标无界 | 添加任意地理范围 | `Demo9.kt` 没有添加边界，补充边界会改变模型 | 2026-09-08 |
| 明确记录 Rust 边界 | 声称两端定义域相同 | `demo9.rs` 使用 `VariableRange::bounded(-100.0, 100.0)` | 2026-09-08 |

### 当前最小模型构建片段

定居点数据来自 `Demo9.kt`/`demo9.rs`。片段保留了源码辅助函数调用；数据和转换器声明省略并注明由源码提供。

::: code-group
```kotlin [Kotlin]
// `settlements`、`flt64Converter` 来自 Demo9.kt/ExampleModeling.kt。
val metaModel = LinearMetaModel<Flt64>("demo9", converter = flt64Converter)
val x = IntVar("x")
val y = IntVar("y")
metaModel.add(x)
metaModel.add(y)
val dx = LinearIntermediateSymbols1<Flt64>("dx", Shape1(settlements.size)) { i, _ ->
    exampleAbsoluteSlack(
        type = UInteger,
        x = flt64Linear(x),
        y = flt64Constant(settlements[i].x),
        name = "dx_$i"
    )
}
val dy = LinearIntermediateSymbols1<Flt64>("dy", Shape1(settlements.size)) { i, _ ->
    exampleAbsoluteSlack(
        type = UInteger,
        x = flt64Linear(y),
        y = flt64Constant(settlements[i].y),
        name = "dy_$i"
    )
}
val distance = LinearIntermediateSymbols1<Flt64>("distance", Shape1(settlements.size)) { i, _ ->
    LinearExpressionSymbol(dx[i] + dy[i], name = "distance_$i")
}
metaModel.add(dx)
metaModel.add(dy)
metaModel.add(distance)
metaModel.minimize(sum(distance[_a]), "total distance")
```

```rust [Rust]
// `settlements` 来自 demo9.rs；Rust 按源码保留坐标边界。
let mut model = MetaModel::<f64>::new("demo9");
let x: VariableCombination1D<Integer> =
    VariableCombination1D::with_range_generator(Shape::new([1]), "x", |_, _| {
    VariableRange::bounded(-100.0, 100.0)
});
let y: VariableCombination1D<Integer> =
    VariableCombination1D::with_range_generator(Shape::new([1]), "y", |_, _| {
    VariableRange::bounded(-100.0, 100.0)
});
let x_idx = model.register_combination(&x)?;
let y_idx = model.register_combination(&y)?;
let dx_fn = SymbolCombination::new(Shape::new([settlements.len()]), "dx", |i, _| {
    AbsFunction::named(
        &format!("dx_{}", settlements[i].name),
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[0])],
            -settlements[i].x,
        ),
    )
});
model.add_symbol_combination(&dx_fn)?;
let dy_fn = SymbolCombination::new(Shape::new([settlements.len()]), "dy", |i, _| {
    AbsFunction::named(
        &format!("dy_{}", settlements[i].name),
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, y_idx[0])],
            -settlements[i].y,
        ),
    )
});
model.add_symbol_combination(&dy_fn)?;
let dx_idx: Vec<_> = (0..settlements.len()).map(|i|
    dx_fn.symbol_polynomial(i).monomials()[0].var_index()
).collect();
let dy_idx: Vec<_> = (0..settlements.len()).map(|i|
    dy_fn.symbol_polynomial(i).monomials()[0].var_index()
).collect();
let distance_expr = flat_map1_indexed("distance", settlements, |i, settlement| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, dx_idx[i]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, dy_idx[i]),
    ], 0.0)
}, |_, settlement| settlement.name.clone());
model.add_symbol_combination(&distance_expr)?;
let mut objective = Vec::new();
for i in 0..settlements.len() {
    for monomial in distance_expr.symbol_polynomial(i).monomials() {
        objective.push((monomial.var_index(), *monomial.coefficient()));
    }
}
model.add_linear_objective(&objective, "distance");
model.set_objective_category(ObjectiveCategory::Minimum);
// AbsFunction 机制会在模型转换阶段自动生成绝对值线性化约束。
```
:::

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 2026-09-08 | 按领域模型模板重组页面；记录辅助函数生成的约束、Rust 独有边界及 Kotlin/Rust 标签页 | 在不声称两端定义域相同的前提下对照当前实现 |

## 源码与验证

- [Kotlin 实现：`Demo9.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo9.kt)
- [Kotlin 共享辅助函数：`ExampleModeling.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/ExampleModeling.kt)
- [Rust 对照实现：`demo9.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo9.rs)
- [Kotlin 结构测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
