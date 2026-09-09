# 示例 6：有界多件背包

## 1. 概述

本限界上下文在每种货物的库存上限和总重量上限下决定各货物类型的整数件数，并最大化总价值。

### 1. 依赖上下文

1. 核心整数线性优化上下文（非负整数变量、线性表达式、约束和求解器适配器）。

Kotlin 与 Rust 代码是建模片段；货物数据和求解器设置来自链接的 Demo6 实现。

---

## 2. 概念 / 实体

### 1. 货物类型

货物类型可以选择多件，但库存有限。

**$Weight_c$**：货物类型 $c$ 的单位重量。

**$Value_c$**：货物类型 $c$ 的单位价值。

**$Amount^{Max}_c$**：货物类型 $c$ 的最大可用件数。

当前数据如下：

| 货物 | 重量 | 价值 | 可用数量 |
| :---: | ---: | ---: | ---: |
| 1 | 1 | 6 | 10 |
| 2 | 2 | 10 | 5 |
| 3 | 2 | 20 | 2 |

总重量上限为 $Weight^{Max}=8$。

---

## 3. 变量

### 1. 决策变量

**$x_c$**：货物件数变量，物品件数，定义域为 $\mathbb{Z}_{\ge0}$，表示选择货物类型 $c$ 的件数，$\forall c\in C$。

### 2. 辅助变量

无。总价值和总重量注册为线性表达式/中间值。

---

## 4. 谓词

### 1. 货物数量状态

> 谓词用于按照是否对计划有贡献给货物类型分类。

**SelectedType**$(c)$：货物类型 $c$ 的选择件数为正，$x_c>0$。

**UnusedType**$(c)$：货物类型 $c$ 未选择，$x_c=0$。

**AtStockLimit**$(c)$：货物类型 $c$ 的所有库存都被选择，$x_c=Amount^{Max}_c$。

---

## 5. 集合

### 1. 货物类型类别

**$C$**：货物类型全集。

**$C^{SelectedType}$**：满足 SelectedType 的子集，$C^{SelectedType}=\{c\in C\mid x_c>0\}$，表示至少贡献一件货物的类型。

**$C^{UnusedType}$**：满足 UnusedType 的子集，$C^{UnusedType}=\{c\in C\mid x_c=0\}$，表示未选择的货物类型。

### 2. 实体对 / 关系

不需要实体对关系；每条库存规则只作用于一种货物类型。

---

## 6. 中间值

### 1. 总价值

**说明**：总价值是每种货物单位价值乘以其选择件数后的总和。

$$
Value(x)=\sum_{c\in C}Value_cx_c.
$$

### 2. 总重量

**说明**：总重量是每种货物单位重量乘以其选择件数后的总和。

$$
Weight(x)=\sum_{c\in C}Weight_cx_c.
$$

---

## 7. 断言

### 1. 非负整数件数

**说明**：每种货物只能贡献零件或多件完整货物。

$$
\forall c\in C\; (x_c\in\mathbb{Z}_{\ge0}).
$$

### 2. 库存有界件数

**说明**：每种货物的选择件数不超过记录的库存量。

$$
\forall c\in C\; (0\le x_c\le Amount^{Max}_c).
$$

---

## 8. 约束

### 1. Total Weight Capacity [总重量容量上限]

**业务说明**：所有已选择货物的总重量不得超过背包容量。

$$
s.t.\quad Weight(x)=\sum_{c\in C}Weight_cx_c\le Weight^{Max}=8.
$$

### 2. Cargo Stock Limit [货物库存上限]

**业务说明**：每种货物类型的选择件数不得超过其可用库存。

$$
s.t.\quad x_c\le Amount^{Max}_c,\qquad \forall c\in C.
$$

下界 $x_c\ge0$ 由 Kotlin UIntVariable1 和 Rust UInteger 的变量定义域提供；两端当前源码都没有单独添加下界行。

---

## 9. 目标函数（如适用）

**说明**：最大化所选货物件数的总价值。

$$
\max\; Value(x)=\sum_{c\in C}Value_cx_c.
$$

一组展示用最优解为 $x=(4,0,2)$，总重量为 $8$，总价值为 $64$。

---

## 10. 算法引用

没有引用独立算法文档。这是一个直接的有界整数背包模型。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 不适用 | — | — | 不需要领域专用算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 货物类型 | $c\in C$ | 可以选择多件的物品类型。 |
| 单位重量 | $Weight_c$ | 类型 $c$ 一件货物的重量。 |
| 单位价值 | $Value_c$ | 类型 $c$ 一件货物的价值。 |
| 选择件数 | $x_c$ | 类型 $c$ 被选择的完整件数。 |
| 库存上限 | $Amount^{Max}_c$ | 类型 $c$ 的可用数量。 |
| 重量容量 | $Weight^{Max}$ | 允许的总选择重量上限。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用非负整数件数变量 | 二元变量 | 源码使用 UIntVariable1/UInteger，允许同一类型选择多件。 | 当前实现 |
| 将库存编码为变量上界 | 忽略库存或将每一件展开为二元物品 | Kotlin 设置每个变量的上界；Rust 为每种货物添加一条上界约束。 | 当前实现 |
| 与示例 5 保持模型区分 | 复用 0–1 子集语言 | 示例 6 允许重复件数，单独使用“子集”会产生误导。 | 当前实现 |

### 当前最小实现片段

以下代码摘录自 Demo6，是非独立片段。cargos、maxWeight、模型设置、converter 和求解器设置均来自链接源码。

::: code-group

```kotlin [Kotlin]
// Fragment from Demo6.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo6's private cargos list and maxWeight.
val x = UIntVariable1("x", Shape1(cargos.size))
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
for (c in cargos) {
    x[c].range.ls(c.amount)
}
metaModel.maximize(cargoValue, "value")
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    name = "weight"
)
```

```rust [Rust]
// Fragment from demo6.rs::IntegerKnapsackModel::register/add_constraints.
// Data source: build_cargos and max_weight in demo6.rs.
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
for (i, cargo) in cargos.iter().enumerate() {
    model.add_linear_constraint(
        &[(x_idx[i], 1.0)],
        ConstraintRelation::LessEqual,
        cargo.max_amount,
        &format!("upper_{}", i),
    )?;
}
```

:::

#### 源码与验证

- [Rust 对照实现：demo6.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo6.rs)

Rust 对照实现使用相同的有界整数数学模型和数据，但 Rust 的 MetaModel、变量组合和符号组合 API 与 Kotlin API 独立。

- [当前实现：Demo6.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo6.kt)
- [Core 结构构建测试：CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

源码使用 UIntVariable1/UInteger 而不是二元变量，设置每种类型的上界，并添加总重量不等式。以上片段是建模摘录，不是可直接运行的完整程序。

---

## 13. 变更日志

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型模板重组示例，补充带量词的库存断言、具名约束及 Kotlin/Rust 片段。 | 明确有界多件模型与 0–1 模型的区别。 |
