# 示例 4：带物料和差异限制的生产

## 1. 概述

本限界上下文决定两种产品的生产量，在物料可用量、产品产量和产品间差异规则下最大化利润。

### 1. 依赖上下文

1. 核心连续线性优化上下文（实数变量、线性中间符号、约束和求解器适配器）。

Kotlin 与 Rust 实现的数学模型接近，但变量定义域不同。以下代码是建模片段，数据来自链接的 Demo4 实现。

---

## 2. 概念 / 实体

### 1. 物料

物料是具有可用量上限的消耗资源。

**$Available_m$**：物料 $m$ 的可用量。

当前物料数据为 $Available_A=24$、$Available_B=8$。

### 2. 产品

产品具有利润、最大产量和物料用量系数。

**$Profit_p$**：产品 $p$ 的单位利润。

**$Yield^{Max}_p$**：产品 $p$ 的最大产量。

**$Use_{pm}$**：每生产一单位产品 $p$ 消耗的物料 $m$ 数量。

当前产品数据如下：

| 产品 | 利润 | 最大产量 | 每单位 A 用量 | 每单位 B 用量 |
| :---: | ---: | ---: | ---: | ---: |
| P1 | 5 | 3 | 6 | 1 |
| P2 | 4 | 2 | 4 | 2 |

源码设置 $Diff^{Max}=1$（maxDiff = 1）。

---

## 3. 变量

### 1. 决策变量

**$x_p$**：生产量，连续生产数量；在当前 Kotlin 实现中定义域为 $\mathbb{R}$，表示产品 $p$ 的产量，$\forall p\in P$。

Kotlin 源码使用 RealVariable1，仅设置上界 $x_p\le Yield^{Max}_p$。Rust 对照实现使用 UContinuous，定义域为 $0\le x_p\le Yield^{Max}_p$；这一实现差异在下文明确记录。

### 2. 辅助变量

无。物料使用量和产品间差异都是派生的线性量。

---

## 4. 谓词

### 1. 生产状态

> 谓词用于给产品和有序产品对分类。

**WithinYield**$(p)$：产品 $p$ 的产量不超过其配置上限。

**MaterialFeasible**$(m)$：物料 $m$ 的总使用量不超过可用量。

**OrderedDistinct**$(p,q)$：$p$、$q$ 是不同产品，且有序对受差异规则约束。

---

## 5. 集合

### 1. 物料类别

**$M$**：物料全集。

### 2. 产品类别

**$P$**：产品全集。

### 3. 实体对 / 关系

**$D=\{(p,q)\in P\times P\mid p\ne q\}$**：不同产品的有序对集合。

当前实现对每两个不同产品都包含 $(p,q)$ 和 $(q,p)$ 两个方向。

---

## 6. 中间值

### 1. 总利润

**说明**：总利润是每种产品单位利润乘以生产量后的总和，并作为目标表达式。

$$
Profit(x)=\sum_{p\in P}Profit_px_p.
$$

### 2. 物料使用量

**说明**：物料使用量记录所有产品对每种物料的总消耗。

$$
Use_m(x)=\sum_{p\in P}Use_{pm}x_p,\qquad \forall m\in M.
$$

### 3. 有序生产差

**说明**：有序生产差是差异约束使用的派生量，不是单独注册的决策变量。

$$
Difference_{pq}(x)=x_p-x_q,\qquad \forall(p,q)\in D.
$$

---

## 7. 断言

### 1. 当前数据非负

**说明**：当前利润、容量、产量上限、物料用量系数和差异上限都是非负数据。

$$
\forall p\in P\;(Profit_p\ge0\wedge Yield^{Max}_p\ge0)
\;\wedge\;
\forall m\in M\;Available_m\ge0
\;\wedge\;
\forall(p,m)\in P\times M\;Use_{pm}\ge0.
$$

### 2. 反向有序对

**说明**：每个不同产品对都有两个方向，因此单向差异约束族等价于绝对差约束。

$$
\forall(p,q)\in D\; \bigl((q,p)\in D\bigr).
$$

---

## 8. 约束

> 以下是当前 Kotlin 模型中的约束；Rust 对照实现还在变量定义域中编码了非负性。

### 1. Maximum Product Yield [最大产品产量]

**业务说明**：每种产品的生产量不得超过配置的最大产量。

$$
s.t.\quad x_p\le Yield^{Max}_p,\qquad \forall p\in P.
$$

### 2. Material Availability [物料可用量上限]

**业务说明**：每种物料的总使用量不得超过其可用量。

$$
s.t.\quad Use_m(x)\le Available_m,\qquad \forall m\in M.
$$

### 3. Pairwise Production Difference [产品间产量差上限]

**业务说明**：对于每个不同产品的有序对，一个产品的生产量最多比另一个高 $Diff^{Max}$。

$$
s.t.\quad Difference_{pq}(x)=x_p-x_q\le Diff^{Max},\qquad \forall(p,q)\in D.
$$

**推论**：由于两个方向都存在，产品间差异约束族等价于绝对差上限。

$$
\forall(p,q)\in D\quad |x_p-x_q|\le Diff^{Max}.
$$

当前 Kotlin 模型没有添加 $x_p\ge0$。增加该业务规则会改变 Kotlin 实现；Rust 对照实现已经使用有界定义域 $0\le x_p\le Yield^{Max}_p$。

---

## 9. 目标函数（如适用）

**说明**：最大化生产总利润。

$$
\max\; Profit(x)=\sum_{p\in P}Profit_px_p.
$$

对于表中数据，$(x_{P1},x_{P2})=(8/3,5/3)$ 是当前 Kotlin 模型的一个最优解；两者恰好为正，但 Kotlin 变量定义域仍允许负值。

---

## 10. 算法引用

没有引用独立算法文档。这是一个直接的连续线性模型。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 不适用 | — | — | 不需要领域专用算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 物料 | $m\in M$ | 具有可用量上限的消耗资源。 |
| 产品 | $p\in P$ | 具有利润和最大产量上限的产品。 |
| 生产量 | $x_p$ | 产品 $p$ 的连续产量。 |
| 物料使用量 | $Use_m(x)$ | 物料 $m$ 的总消耗。 |
| 产品间差异 | $Difference_{pq}(x)$ | 有序差 $x_p-x_q$。 |
| 最大产量 | $Yield^{Max}_p$ | 产品 $p$ 的产量上界。 |
| 差异上限 | $Diff^{Max}$ | 允许的最大有序生产差。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 理由 | 日期 |
|------|----------|------|------|
| 保留 Kotlin RealVariable1 的有符号定义域 | 显式添加非负下界 | 当前 Kotlin 源码仅设置 x[p].range.ls(maxYield)，因此仍允许负值。 | 当前实现 |
| 明确记录 Rust 的非负有界定义域差异 | 强制两端变量定义域一致 | Rust 使用 UContinuous 和 VariableRange::bounded(0.0, maxYield)，排除了 Kotlin 独有的负值区域。 | 当前实现 |
| 注册差异规则的两个方向 | 添加一个绝对值约束 | 源码遍历每个 p1 != p2 有序对，两个方向产生相同业务效果。 | 当前实现 |

### 当前最小实现片段

以下代码摘录自 Demo4，是非独立片段。products、materials、maxDiff、模型设置、converter 和求解器设置均来自链接源码。

::: code-group

```kotlin [Kotlin]
// Fragment from Demo4.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo4's private materials/products lists and maxDiff.
val x = RealVariable1("x", Shape1(products.size))
val profit = LinearExpressionSymbol(
    sum(products) { p -> p.profit * x[p] },
    name = "profit"
)
val use = LinearIntermediateSymbols1<Flt64>(
    "use",
    Shape1(materials.size)
) { m, _ ->
    val material = materials[m]
    LinearExpressionSymbol(
        sum(products.filter { it.use.contains(material) }) { p ->
            p.use[material]!! * x[p]
        },
        name = "use"
    )
}
metaModel.add(x)
metaModel.add(profit)
metaModel.add(use)
metaModel.maximize(profit, "profit")
for (p in products) {
    x[p].range.ls(p.maxYield)
}
for (m in materials) {
    metaModel.addConstraint(use[m] leq m.available)
}
for (p1 in products) {
    for (p2 in products) {
        if (p1.index != p2.index) {
            metaModel.addConstraint((x[p1] - x[p2]) leq maxDiff.toFlt64())
        }
    }
}
```

```rust [Rust]
// Fragment from demo4.rs::ProductionModel::register/add_constraints.
// Data source: build_materials/build_products; the Rust range is explicit.
let x = VariableCombination1D::with_range_generator(
    Shape::new([products.len()]),
    "x",
    |i, _| VariableRange::bounded(0.0, products[i].max_yield),
);
let x_idx = model.register_combination(&x)?;
let profit = flat_map1_indexed(
    "profit",
    products,
    |i, product| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                product.profit,
                x_idx[i],
            )],
            0.0,
        )
    },
    |_, product| product.name.clone(),
);
model.add_symbol_combination(&profit)?;
let r#use = flat_map1_indexed(
    "usage",
    materials,
    |m, _material| {
        let monomials: Vec<_> = products
            .iter()
            .enumerate()
            .map(|(p, product)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    product.usage_by_material[m],
                    x_idx[p],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, material| material.name.clone(),
);
model.add_symbol_combination(&r#use)?;
let profit_coeffs = extract_coeffs(&profit[0]);
model.add_linear_objective(&profit_coeffs, "profit");
model.set_objective_category(ObjectiveCategory::Maximum);
for (m, material) in materials.iter().enumerate() {
    let coeffs = extract_coeffs(&r#use[m]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::LessEqual,
        material.available,
        &format!("material_{}_{}", m, material.name),
    )?;
}
for p1 in 0..products.len() {
    for p2 in 0..products.len() {
        if p1 != p2 {
            let coefficients = vec![(x_idx[p1], 1.0), (x_idx[p2], -1.0)];
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("diff_{}_{}", p1, p2),
            )?;
        }
    }
}
```

:::

#### 源码与验证

- [Rust 对照实现：demo4.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo4.rs)

Rust 对照实现的数学模型接近但并不完全相同：Rust 将产量限制为 0 <= x <= maxYield，而当前 Kotlin 实现使用 RealVariable1，仅设置 x <= maxYield。

- [当前实现：Demo4.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo4.kt)
- [Core 结构构建测试：CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

Kotlin 源码使用 `LinearExpressionSymbol` 表示利润、`LinearIntermediateSymbols1` 表示物料使用量、`RealVariable1` 表示生产量。Rust 源码使用 `VariableCombination1D<UContinuous>` 和有界 `VariableRange`。以上片段是摘录，不是可直接运行的完整程序。

---

## 13. 变更日志

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型模板重组示例，补充带量词的约束、差异推论及 Kotlin/Rust 片段。 | 明确两端有符号/非负定义域差异。 |
