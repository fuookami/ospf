# 示例 5：0–1 背包

## 1. 概述

本限界上下文在总重量上限下选择完整货物，使总价值最大化；不要求必须恰好填满容量。

### 1. 依赖上下文

1. 核心二元线性优化上下文（二元变量、线性表达式、约束和求解器适配器）。

Kotlin 与 Rust 代码是建模片段；货物数据和求解器设置来自链接的 Demo5 实现。

---

## 2. 概念 / 实体

### 1. 货物

货物是一个不可拆分的物品，每件至多选择一次。

**$Weight_c$**：货物 $c$ 的重量。

**$Value_c$**：货物 $c$ 的价值。

当前货物的 $(重量,价值)$ 为 $(2,6),(2,3),(6,5),(5,4),(4,6)$，且 $Weight^{Max}=10$。

---

## 3. 变量

### 1. 决策变量

**$x_c$**：货物选择变量，无量纲二元变量，定义域为 $\{0,1\}$；选择货物 $c$ 时取 $1$，$\forall c\in C$。

### 2. 辅助变量

无。总价值和总重量注册为线性表达式/中间值。

---

## 4. 谓词

### 1. 货物状态

> 谓词用于按照选择和容量状态给货物分类。

**Selected**$(c)$：货物 $c$ 被选择，等价于 $x_c=1$。

**NotSelected**$(c)$：货物 $c$ 未被选择，等价于 $x_c=0$。

**FitsCapacity**$(x)$：所选货物计划满足总重量上限。

---

## 5. 集合

### 1. 货物类别

**$C$**：货物全集。

**$C^{Selected}$**：满足 Selected 的子集，$C^{Selected}=\{c\in C\mid x_c=1\}$，表示被选择的货物。

**$C^{NotSelected}$**：满足 NotSelected 的子集，$C^{NotSelected}=\{c\in C\mid x_c=0\}$，表示未选择的货物。

### 2. 实体对 / 关系

不需要实体对关系；每个决策只针对一件货物。

---

## 6. 中间值

### 1. 总价值

**说明**：总价值是所有被选择货物的价值之和，并作为目标表达式。

$$
Value(x)=\sum_{c\in C}Value_cx_c.
$$

### 2. 总重量

**说明**：总重量是所有被选择货物的重量之和，并与容量上限比较。

$$
Weight(x)=\sum_{c\in C}Weight_cx_c.
$$

---

## 7. 断言

### 1. 整件选择

**说明**：每件货物要么选择一次，要么不选择；模型不表示分数选择或重复件数。

$$
\forall c\in C\; (x_c=0\vee x_c=1).
$$

### 2. 货物数据非负

**说明**：当前货物重量、价值和容量都是非负量。

$$
\forall c\in C\;(Weight_c\ge0\wedge Value_c\ge0)
\;\wedge\;
Weight^{Max}\ge0.
$$

---

## 8. 约束

### 1. Weight Capacity [重量容量上限]

**业务说明**：被选择货物的总重量不得超过可用容量；允许有未使用容量。

$$
s.t.\quad Weight(x)=\sum_{c\in C}Weight_cx_c\le Weight^{Max}=10.
$$

当前没有等式填充要求，也没有最低填充约束。

---

## 9. 目标函数（如适用）

**说明**：最大化所选货物的总价值。

$$
\max\; Value(x)=\sum_{c\in C}Value_cx_c.
$$

---

## 10. 算法引用

没有引用独立算法文档。这是一个直接的 0–1 背包模型。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 不适用 | — | — | 不需要领域专用算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 货物 | $c\in C$ | 可选择一次的不可拆分物品。 |
| 重量 | $Weight_c$ | 单件货物重量；$Weight(x)$ 是所选货物总重量。 |
| 价值 | $Value_c$ | 单件货物价值；$Value(x)$ 是所选货物总价值。 |
| 选择 | $x_c$ | 表示是否选择货物 $c$ 的二元决策。 |
| 容量 | $Weight^{Max}$ | 允许的总重量上限。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用二元货物变量 | 整数多件变量 | 源码使用 BinVariable1，将每条货物数据视为一件不可拆分物品。 | 当前实现 |
| 允许容量未使用 | 要求 Weight(x)=WeightMax | 源码只添加小于等于重量约束。 | 当前实现 |
| 分开说明 Kotlin 与 Rust 模型 API | 将一端当作可移植代码 | 两端共享数学模型，但使用独立的注册 API。 | 当前实现 |

### 当前最小实现片段

以下代码摘录自 Demo5，是非独立片段。cargos、maxWeight、模型设置、converter 和求解器设置均由链接源码提供。

::: code-group

```kotlin [Kotlin]
// Fragment from Demo5.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo5's private cargos list and maxWeight.
val x = BinVariable1("x", Shape1(cargos.size))
val cargoValue = LinearExpressionSymbol(
    sum(cargos) { c -> c.value * x[c] },
    name = "value"
)
val cargoWeight = LinearExpressionSymbol(
    sum(cargos) { c -> c.weight * x[c] },
    name = "weight"
)
metaModel.add(x)
metaModel.add(cargoValue)
metaModel.add(cargoWeight)
metaModel.maximize(cargoValue, "value")
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    name = "weight"
)
```

```rust [Rust]
// Fragment from demo5.rs::KnapsackModel::register/add_constraints.
// Data source: build_cargos and max_weight in demo5.rs.
let x = VariableCombination1D::new(Shape::new([cargos.len()]), "x");
let x_idx = model.register_combination(&x)?;
let cargo_value = flat_map1_indexed(
    "cargo_value",
    cargos,
    |i, cargo| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                cargo.value,
                x_idx[i],
            )],
            0.0,
        )
    },
    |_, cargo| cargo.name.clone(),
);
model.add_symbol_combination(&cargo_value)?;
let cargo_weight = flat_map1_indexed(
    "cargo_weight",
    cargos,
    |i, cargo| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                cargo.weight,
                x_idx[i],
            )],
            0.0,
        )
    },
    |_, cargo| cargo.name.clone(),
);
model.add_symbol_combination(&cargo_weight)?;
let val_coeffs = extract_coeffs(&cargo_value[0]);
model.add_linear_objective(&val_coeffs, "value");
model.set_objective_category(ObjectiveCategory::Maximum);
let wt_coeffs = extract_coeffs(&cargo_weight[0]);
model.add_linear_constraint(
    &wt_coeffs,
    ConstraintRelation::LessEqual,
    max_weight,
    "weight",
)?;
```

:::

#### 源码与验证

- [Rust 对照实现：demo5.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo5.rs)

Rust 对照实现使用相同的 0–1 数学模型和数据，但 Rust 的 MetaModel、变量组合和符号组合 API 与 Kotlin API 独立。

- [当前实现：Demo5.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo5.kt)
- [Core 结构构建测试：CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

该模型是 0–1 模型，不是示例 6 的有界多件模型。以上片段包含当前变量、中间表达式、目标和约束 API，但仍是摘录而非可直接运行的完整程序。

---

## 13. 变更日志

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型模板重组示例，补充带量词的断言、具名容量约束及 Kotlin/Rust 片段。 | 明确整件选择和容量语义。 |
