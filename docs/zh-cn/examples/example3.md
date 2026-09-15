# 示例 3：精确产量的物料规划

## 1. 概述

本限界上下文选择非负整数数量的原材料，使每种产品都恰好达到需求产量，同时最小化物料成本。

### 1. 依赖上下文

1. 核心整数线性优化上下文（整数变量、线性中间符号、约束和求解器适配器）。

Kotlin 与 Rust 代码是建模片段；物料和产品数据来自链接的 Demo3 实现。

---

## 2. 概念 / 实体

### 1. 产品目标

产品目标规定一种产品的需求产量。

**$Demand_p$**：产品 $p$ 的需求产量。Kotlin 源码中对应 Product.minYield 字段，Rust 源码中对应 ProductTarget.min_yield。

### 2. 物料

物料是一种可按整数数量使用、具有单位成本和按产品产量的原材料。

**$Cost_m$**：物料 $m$ 的单位成本。

**$Yield_{mp}$**：每使用一单位物料 $m$ 所产生的产品 $p$ 产量；当前数据中缺失的映射/向量项贡献为零。

当前产品目标如下：

| 产品 | P1 | P2 | P3 |
| :---: | ---: | ---: | ---: |
| 需求产量 | 15000 | 15000 | 10000 |

当前物料数据如下：

| 物料 | 成本 | P1 产量 | P2 产量 | P3 产量 |
| :---: | ---: | ---: | ---: | ---: |
| A | 115 | 30 | 10 | 0 |
| B | 97 | 15 | 0 | 20 |
| C | 82 | 0 | 25 | 15 |
| D | 76 | 15 | 15 | 15 |

---

## 3. 变量

### 1. 决策变量

**$x_m$**：物料用量，物料单位数量，定义域为 $\mathbb{Z}_{\ge0}$，表示使用物料 $m$ 的整数单位数，$\forall m\in M$。

### 2. 辅助变量

无。成本和每种产品的产量注册为线性表达式/中间符号。

---

## 4. 谓词

### 1. 物料与产量覆盖

> 谓词用于给物料以及物料-产品关系分类。

**YieldDefined**$(m,p)$：物料 $m$ 对产品 $p$ 存在显式的非零产量项。

**Used**$(m)$：解给物料 $m$ 分配了正数量，即 $x_m>0$。

---

## 5. 集合

### 1. 物料类别

**$M$**：原材料全集。

### 2. 产品类别

**$P$**：产品目标全集。

### 3. 实体对 / 关系

**$E$**：已定义的物料-产品产量关系，$E=\{(m,p)\in M\times P\mid YieldDefined(m,p)\}$。

$M\times P$ 中缺失的组合贡献为零；源码构造产量表达式时会跳过这些组合。

---

## 6. 中间值

### 1. 总物料成本

**说明**：总物料成本是每种物料单位成本乘以其整数用量后的总和。

$$
Cost(x)=\sum_{m\in M}Cost_mx_m.
$$

### 2. 产品产量

**说明**：产品产量是所有具有已定义产量项的物料对产品 $p$ 的产量之和，并为每个产品目标定义。

$$
Yield_p(x)=\sum_{m\in M:(m,p)\in E}Yield_{mp}x_m,\qquad \forall p\in P.
$$

---

## 7. 断言

### 1. 整数物料用量

**说明**：每种物料的用量都是非负整数。

$$
\forall m\in M\; (x_m\in\mathbb{Z}_{\ge0}).
$$

### 2. 数据非负

**说明**：当前成本、产量和需求都是非负物理量。

$$
\forall m\in M\;(Cost_m\ge0)
\;\wedge\;
\forall(m,p)\in E\;(Yield_{mp}\ge0)
\;\wedge\;
\forall p\in P\;(Demand_p\ge0).
$$

---

## 8. 约束

> 源码对每种产品有意注册两条约束。两条约束共同表达精确产量，而不只是最低产量。

### 1. Minimum Product Yield [最低产品产量]

**业务说明**：每种产品都必须达到至少其需求产量。

$$
s.t.\quad Yield_p(x)\ge Demand_p,\qquad \forall p\in P.
$$

### 2. Maximum Product Yield [最高产品产量]

**业务说明**：每种产品都不得超过其需求产量。

$$
s.t.\quad Yield_p(x)\le Demand_p,\qquad \forall p\in P.
$$

**推论**：两条硬约束对每个产品目标共同推出精确产量。

$$
\forall p\in P\;
\bigl(Yield_p(x)\ge Demand_p\;\wedge\;Yield_p(x)\le Demand_p\bigr)
\Rightarrow Yield_p(x)=Demand_p.
$$

---

## 9. 目标函数（如适用）

**说明**：最小化整数物料计划的成本。

$$
\min\; Cost(x)=\sum_{m\in M}Cost_mx_m.
$$

一组展示用最优解为 $(x_A,x_B,x_C,x_D)=(284,8,232,424)$，对应产量 $(15000,15000,10000)$。

---

## 10. 算法引用

没有引用独立算法文档。本模型是直接的整数线性规划；产量表达式根据已定义的物料-产品项构造。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 不适用 | — | — | 不需要领域专用算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 物料 | $m\in M$ | 可以按整数单位使用的原材料。 |
| 产品目标 | $p\in P$ | 具有精确需求产量的产品。 |
| 物料用量 | $x_m$ | 物料 $m$ 的整数用量。 |
| 单位成本 | $Cost_m$ | 物料 $m$ 的每单位成本。 |
| 产量系数 | $Yield_{mp}$ | 一单位物料 $m$ 对产品 $p$ 的产量。 |
| 产品产量 | $Yield_p(x)$ | 产品 $p$ 的总产量。 |
| 需求 | $Demand_p$ | 产品 $p$ 的需求数量。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用非负整数物料变量 | 连续或有符号数量 | Kotlin 使用 UIntVariable1，Rust 使用 UInteger；物料用量不能为小数或负数。 | 当前实现 |
| 忽略未定义的产量项 | 显式添加零单项式 | Kotlin 过滤产量映射；Rust 在构造线性表达式前过滤零系数。 | 当前实现 |
| 保留产品产量上下界两条约束 | 只添加下界 | 当前源码同时添加 geq 和 leq，因此结果是精确产量。 | 当前实现 |

### 当前最小实现片段

以下代码摘录自 Demo3，是非独立片段。materials、products/targets、模型设置、converter 和求解器设置均由链接源码提供。

::: code-group

```kotlin [Kotlin]
// Fragment from Demo3.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo3's private materials and products lists.
val x = UIntVariable1("x", Shape1(materials.size))
val cost = LinearExpressionSymbol(
    sum(materials) { it.cost * x[it] },
    name = "cost"
)
val yield = LinearIntermediateSymbols1<Flt64>(
    "yield",
    Shape1(products.size)
) { p, _ ->
    val product = products[p]
    LinearExpressionSymbol(
        sum(materials.filter { it.yieldQuantity.contains(product) }) { m ->
            m.yieldQuantity[product]!! * x[m]
        },
        name = "yield_product"
    )
}
metaModel.add(x)
metaModel.add(cost)
metaModel.add(yield)
metaModel.minimize(cost)
for (p in products) {
    metaModel.addConstraint(yield[p.index] geq p.minYield)
    metaModel.addConstraint(yield[p.index] leq p.minYield)
}
```

```rust [Rust]
// Fragment from demo3.rs::BlendingModel::register/add_constraints.
// Data source: build_materials/build_product_targets; zero entries are filtered.
let x = VariableCombination1D::new(Shape::new([materials.len()]), "x");
let x_idx = model.register_combination(&x)?;
let cost = flat_map1_indexed(
    "cost",
    materials,
    |m_idx, m| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                m.unit_cost,
                x_idx[m_idx],
            )],
            0.0,
        )
    },
    |_, m| m.name.clone(),
);
model.add_symbol_combination(&cost)?;
let yields = flat_map1_indexed(
    "yield",
    targets,
    |p, _target| {
        let monomials: Vec<_> = materials
            .iter()
            .enumerate()
            .filter_map(|(m_idx, m)| {
                let coeff = m.yields[p];
                if coeff != 0.0 {
                    Some(ospf_rust_core::symbol::flatten::LinearMonomial::new(
                        coeff,
                        x_idx[m_idx],
                    ))
                } else {
                    None
                }
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, target| target.name.clone(),
);
model.add_symbol_combination(&yields)?;
let cost_coeffs = extract_coeffs(&cost[0]);
model.add_linear_objective(&cost_coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for (p, target) in targets.iter().enumerate() {
    let coeffs = extract_coeffs(&yields[p]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::GreaterEqual,
        target.min_yield,
        &format!("yield_{}_lb", target.name),
    )?;
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::LessEqual,
        target.min_yield,
        &format!("yield_{}_ub", target.name),
    )?;
}
```

:::

#### 源码与验证

- [Rust 对照实现：demo3.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo3.rs)

Rust 对照实现使用相同的数学模型和数据，但 Rust 的 MetaModel、变量组合和符号组合 API 与 Kotlin API 独立。

- [当前实现：Demo3.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo3.kt)
- [Core 结构构建测试：CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

Kotlin 源码将产品字段命名为 minYield，Rust 源码将对应字段命名为 min_yield。两端都为每个产品目标添加产量下界和上界。以上片段是建模摘录，不是可直接运行的完整程序。

---

## 13. 变更日志

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型模板重组示例，补充产量断言、带量词的约束及 Kotlin/Rust 片段。 | 明确精确产量语义和缺失项处理。 |
