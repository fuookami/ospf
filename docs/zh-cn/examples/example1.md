# 示例 1：资本投资选择

## 1. 概述

本限界上下文为投资组合选择企业，在满足总资本最低要求和总负债上限的同时最大化总利润。

### 1. 依赖上下文

1. 核心线性优化上下文（模型变量、线性表达式、约束和求解器适配器）。

本文描述数学模型。Kotlin 与 Rust 代码是建模片段；示例数据由链接的 `Demo1` 实现提供。

---

## 2. 概念 / 实体

### 1. 企业

企业是一个候选投资对象，其三个数值属性都是只读输入数据。

**$Capital_c$**：企业 $c$ 提供的资本额。

**$Liability_c$**：企业 $c$ 承担的负债额。

**$Profit_c$**：企业 $c$ 带来的利润额。

当前数据如下：

| 企业 | 资本 | 负债 | 利润 |
| :---: | ---: | ---: | ---: |
| A | 3.48 | 1.28 | 5400 |
| B | 5.62 | 2.53 | 2300 |
| C | 7.33 | 1.02 | 4600 |
| D | 6.27 | 3.55 | 3300 |
| E | 2.14 | 0.53 | 980 |

资本与负债使用同一模型单位；利润使用数据表中的利润单位。

---

## 3. 变量

### 1. 决策变量

**$x_c$**：企业选择变量，无量纲二元变量，定义域为 $\{0,1\}$；当且仅当选择企业 $c$ 时取 $1$，$\forall c\in C$。

### 2. 辅助变量

无。资本、负债和利润是注册到模型中的线性表达式，不是求解器决策变量。

---

## 4. 谓词

### 1. 企业状态

> 谓词用于给实体集合分类；每个谓词定义一个子集。

**Selected**$(c)$：企业 $c$ 属于已选择的投资组合。

**Unselected**$(c)$：企业 $c$ 未被选择。

---

## 5. 集合

### 1. 企业类别

**$C$**：候选企业的全集。

**$C^{Selected}$**：满足 `Selected` 的子集，$C^{Selected}=\{c\in C\mid x_c=1\}$，表示已选择的投资组合。

**$C^{Unselected}$**：满足 `Unselected` 的子集，$C^{Unselected}=\{c\in C\mid x_c=0\}$，表示未纳入组合的企业。

### 2. 实体对 / 关系

不需要实体对关系；每个决策只针对一家企业。

---

## 6. 中间值

### 1. 总资本

**说明**：总资本是所有被选择企业的资本之和，也是与最低资本阈值比较的量。

$$
Capital(x)=\sum_{c\in C}Capital_cx_c.
$$

### 2. 总负债

**说明**：总负债是所有被选择企业的负债之和，也是与最大负债阈值比较的量。

$$
Liability(x)=\sum_{c\in C}Liability_cx_c.
$$

### 3. 总利润

**说明**：总利润是所有被选择企业的利润之和，并作为优化目标。

$$
Profit(x)=\sum_{c\in C}Profit_cx_c.
$$

---

## 7. 断言

### 1. 二元选择

**说明**：每家企业要么被选择，要么未被选择；模型不允许分数选择。

$$
\forall c\in C\; (x_c=0\vee x_c=1).
$$

### 2. 投资组合划分

**说明**：已选择与未选择两个子集共同划分候选企业全集。

$$
C^{Selected}\cap C^{Unselected}=\emptyset
\quad\wedge\quad
C^{Selected}\cup C^{Unselected}=C.
$$

---

## 8. 约束

> 两条约束都是硬约束，且包含边界值。

### 1. Minimum Capital Requirement [最低资本要求]

**业务说明**：投资组合的总资本必须达到最低要求，等于阈值时也满足。

$$
s.t.\quad Capital(x)\ge Capital^{Min},\qquad Capital^{Min}=10.
$$

### 2. Maximum Liability Limit [最大负债上限]

**业务说明**：投资组合的总负债不得超过允许上限，等于阈值时也满足。

$$
s.t.\quad Liability(x)\le Liability^{Max},\qquad Liability^{Max}=5.
$$

当前模型没有数量、分散化或平衡约束。

---

## 9. 目标函数（如适用）

**说明**：最大化所选企业贡献的利润。

$$
\max\; Profit(x)=\sum_{c\in C}Profit_cx_c.
$$

对于表中数据，一组最优投资组合为 $\{A,B,C\}$，资本为 $16.43$，负债为 $4.83$，利润为 $12300$。

---

## 10. 算法引用

没有引用独立算法文档。本模型是直接的线性 0–1 建模，通过核心线性模型 API 求解。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 不适用 | — | — | 不需要领域专用算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 企业 | $c\in C$ | 候选投资企业。 |
| 资本 | $Capital_c$ | 单家企业贡献的资本；$Capital(x)$ 是被选择企业的总资本。 |
| 负债 | $Liability_c$ | 单家企业贡献的负债；$Liability(x)$ 是被选择企业的总负债。 |
| 利润 | $Profit_c$ | 单家企业贡献的利润；$Profit(x)$ 是被选择企业的总利润。 |
| 选择 | $x_c$ | 表示是否选择企业 $c$ 的二元决策。 |
| 最低资本 | $Capital^{Min}$ | 总资本的要求下界。 |
| 最大负债 | $Liability^{Max}$ | 总负债的允许上界。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 理由 | 日期 |
|------|----------|------|------|
| 每家企业使用一个二元变量 | 连续份额或整数数量 | 源码使用 `BinVariable1` 表达全选或不选。 | 当前实现 |
| 两条边界都采用包含关系 | 严格不等式 | Kotlin 使用 `geq` 与 `leq`；Rust 模型使用 `GreaterEqual` 与 `LessEqual`。 | 当前实现 |
| Kotlin 与 Rust API 分开说明 | 将一端代码当作可移植代码 | 两端表达相同数学模型，但模型注册 API 独立。 | 当前实现 |

### 当前最小实现片段

以下代码有意保留为非独立建模片段。`companies`、`minCapital`、`maxLiability`、`flt64Converter` 和求解器设置均来自链接的 `Demo1` 源码；本文不暗示额外 API。

::: code-group

```kotlin [Kotlin]
// Fragment from Demo1.initVariable/initSymbol/initObject/initConstraint.
// Data source: Demo1's private companies list and thresholds.
val x = BinVariable1("x", Shape1(companies.size))
val capital = LinearExpressionSymbol(
    sum(companies) { it.capital * x[it] },
    name = "capital"
)
val liability = LinearExpressionSymbol(
    sum(companies) { it.liability * x[it] },
    name = "liability"
)
val profit = LinearExpressionSymbol(
    sum(companies) { it.profit * x[it] },
    name = "profit"
)
metaModel.add(x)
metaModel.add(capital)
metaModel.add(liability)
metaModel.add(profit)
metaModel.maximize(profit)
metaModel.addConstraint(capital geq minCapital)
metaModel.addConstraint(liability leq maxLiability)
```

```rust [Rust]
// Fragment from demo1.rs::PortfolioModel::register/add_constraints.
// Data source: demo1.rs::get_companies; Metric and helper imports are from that module.
let select = VariableCombination1D::new(
    Shape::new([companies.len()]),
    "select",
);
let select_idx = model.register_combination(&select)?;
let metrics_list = vec![Metric::Capital, Metric::Liability, Metric::Profit];
let metrics = flat_map1(
    "portfolio_metric",
    &metrics_list,
    |metric| {
        let monomials: Vec<_> = companies
            .iter()
            .enumerate()
            .map(|(i, company)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    metric.value(company),
                    select_idx[i],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, metric| metric.name().to_string(),
);
model.add_symbol_combination(&metrics)?;
let cap_coeffs = extract_coeffs(&metrics[0]);
model.add_linear_constraint(
    &cap_coeffs,
    ConstraintRelation::GreaterEqual,
    min_capital,
    "capital_constraint",
)?;
let lia_coeffs = extract_coeffs(&metrics[1]);
model.add_linear_constraint(
    &lia_coeffs,
    ConstraintRelation::LessEqual,
    max_liability,
    "liability_constraint",
)?;
let obj_coeffs = extract_coeffs(&metrics[2]);
model.add_linear_objective(&obj_coeffs, "total_profit");
model.set_objective_category(ObjectiveCategory::Maximum);
```

:::

#### 源码与验证

- [Rust 对照实现：`demo1.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo1.rs)

Rust 对照实现使用相同的数学模型和数据，但 Rust 的 `MetaModel`、变量组合和符号组合 API 与 Kotlin API 独立。

- [当前实现：`Demo1.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo1.kt)
- [Core 结构构建测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

上述最优解选择 A、B、C。代码片段对应当前的 `BinVariable1`、`LinearExpressionSymbol`、`LinearMetaModel<Flt64>`、与 `ScipLinearSolver` 配套的表达式，以及 Rust `MetaModel` API；它们是摘录，不是可直接运行的完整程序。

---

## 13. 变更日志

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型模板重组示例，补充带量词的约束、断言及 Kotlin/Rust 片段。 | 明确数学模型与当前实现边界。 |
