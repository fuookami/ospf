# 示例 2：产品分配

## 1. 概述

本限界上下文将每种产品恰好分配给一家企业，同时限制每家企业至多生产一种产品，并最小化总成本。

### 1. 依赖上下文

1. 核心线性分配上下文（二元变量、线性表达式、约束和求解器适配器）。

Kotlin 与 Rust 代码是建模片段；企业/产品数据和成本映射来自链接的 Demo2 实现。

---

## 2. 概念 / 实体

### 1. 企业

企业是一个生产方，在当前模型中至多接收一种产品。

**$Cost_{cp}$**：企业 $c$ 生产产品 $p$ 的成本，定义在允许的分配关系上。

当前成本矩阵（行是企业，列是产品）如下：

|  | P1 | P2 | P3 | P4 |
| :---: | ---: | ---: | ---: | ---: |
| C1 | 920 | 480 | 650 | 340 |
| C2 | 870 | 510 | 700 | 350 |
| C3 | 880 | 500 | 720 | 400 |
| C4 | 930 | 490 | 680 | 410 |

### 2. 产品

产品是必须分配给一家企业的需求对象。

**$ProductID_p$**：产品 $p$ 的标识；当前数据包含 P1 至 P4。

---

## 3. 变量

### 1. 决策变量

**$x_{cp}$**：分配变量，无量纲二元变量，定义域为 $\{0,1\}$；当企业 $c$ 生产产品 $p$ 时取 $1$，$\forall(c,p)\in A$。

### 2. 辅助变量

无。企业分配计数和产品分配计数注册为线性中间值。

---

## 4. 谓词

### 1. 分配关系

> 谓词用于给实体对及其分配状态分类。

**CostDefined**$(c,p)$：企业-产品对 $(c,p)$ 存在成本项。

**Assigned**$(c,p)$：产品 $p$ 分配给企业 $c$，等价于 $x_{cp}=1$。

---

## 5. 集合

### 1. 企业类别

**$C$**：企业全集。

### 2. 产品类别

**$P$**：产品全集。

### 3. 实体对 / 关系

**$A=C\times P$**：允许的企业-产品分配对。当前实例允许所有组合。

**$A^{CostDefined}$**：满足 CostDefined 的子集，$A^{CostDefined}=\{(c,p)\in A\mid Cost_{cp}\text{ 已定义}\}$，表示在源码模型中产生成本和变量的组合。

当前矩阵满足 $A^{CostDefined}=A$。如果未来出现禁配组合，变量和求和都必须限制在 $A^{CostDefined}$ 上，这与 Kotlin 的映射过滤逻辑以及 Rust 的矩形数据假设一致。

---

## 6. 中间值

### 1. 总分配成本

**说明**：总分配成本是所有被选择企业-产品对的成本之和，也是模型最小化的量。

$$
Cost(x)=\sum_{(c,p)\in A}Cost_{cp}x_{cp}.
$$

### 2. 企业分配计数

**说明**：企业分配计数记录企业 $c$ 被分配的产品数量。

$$
Assignment^{Company}_c(x)=\sum_{p\in P:(c,p)\in A}x_{cp},\qquad \forall c\in C.
$$

### 3. 产品分配计数

**说明**：产品分配计数记录产品 $p$ 被分配的企业数量。

$$
Assignment^{Product}_p(x)=\sum_{c\in C:(c,p)\in A}x_{cp},\qquad \forall p\in P.
$$

---

## 7. 断言

### 1. 二元分配

**说明**：每个允许的组合要么被选择，要么不被选择；模型不包含分数分配。

$$
\forall(c,p)\in A\; (x_{cp}=0\vee x_{cp}=1).
$$

### 2. 当前成本数据完整

**说明**：当前四乘四数据表为每个允许的组合定义了成本。

$$
\forall(c,p)\in A\; CostDefined(c,p).
$$

---

## 8. 约束

> 两族约束都是硬性分配规则。

### 1. At Most One Product per Company [每家公司至多一个产品]

**业务说明**：一家企业不能接收多于一个产品分配。

$$
s.t.\quad Assignment^{Company}_c(x)=\sum_{p\in P:(c,p)\in A}x_{cp}\le 1,\qquad \forall c\in C.
$$

### 2. Exactly One Company per Product [每个产品恰好一家企业]

**业务说明**：每个产品必须恰好分配给一家企业。

$$
s.t.\quad Assignment^{Product}_p(x)=\sum_{c\in C:(c,p)\in A}x_{cp}=1,\qquad \forall p\in P.
$$

由于当前实例有四家企业和四种产品，上述两族约束还会推出每家企业恰好获得一种产品；这是当前基数带来的推论，不是额外约束。

---

## 9. 目标函数（如适用）

**说明**：最小化所有被选择企业-产品分配的总成本。

$$
\min\; Cost(x)=\sum_{(c,p)\in A}Cost_{cp}x_{cp}.
$$

---

## 10. 算法引用

没有引用独立算法文档。这是一个直接的二元线性分配模型。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 不适用 | — | — | 不需要领域专用算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 企业 | $c\in C$ | 至多接收一种产品的生产方。 |
| 产品 | $p\in P$ | 必须分配给一家企业的产品。 |
| 分配 | $x_{cp}$ | 企业 $c$ 与产品 $p$ 的二元决策。 |
| 组合成本 | $Cost_{cp}$ | 将产品 $p$ 分配给企业 $c$ 的成本。 |
| 企业计数 | $Assignment^{Company}_c$ | 分配给企业 $c$ 的产品数量。 |
| 产品计数 | $Assignment^{Product}_p$ | 分配产品 $p$ 的企业数量。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用企业-产品对的二元变量 | 数量分配变量 | 源码使用 BinVariable2；每种产品整体分配给一家企业。 | 当前实现 |
| 表达式只使用已定义成本对 | 将缺失成本视为零 | Kotlin 使用 mapNotNull/let 忽略未定义组合；当前数据定义了所有组合。 | 当前实现 |
| 分开说明两端 API | 将 Kotlin 调用复制到 Rust | Kotlin 使用 BinVariable2 和中间符号；Rust 使用 VariableCombination2D、SymbolCombination 和 MetaModel。 | 当前实现 |

### 当前最小实现片段

以下代码摘录自 Demo2，是非独立片段。companies、products、flt64Converter 和模型注册由链接源码提供；片段不虚构跨语言共用 API。

::: code-group

```kotlin [Kotlin]
// Fragment from Demo2.initVariable/initSymbol/initConstraint.
// Data source: Demo2's private products, companies, and cost maps.
val x = BinVariable2("x", Shape2(companies.size, products.size))
val cost = LinearExpressionSymbol(
    flatSum(companies) { c ->
        products.mapNotNull { p -> c.cost[p]?.let { it * x[c, p] } }
    },
    name = "cost"
)
val assignmentCompany = LinearIntermediateSymbols(
    "assignment_company",
    Shape1(companies.size),
    Flt64
)
for (c in companies) {
    assignmentCompany[c].asMutable() +=
        sumVars(products) { p -> c.cost[p]?.let { x[c, p] } }
}
metaModel.add(x)
metaModel.add(cost)
metaModel.minimize(cost)
for (c in companies) {
    metaModel.addConstraint(assignmentCompany[c] leq 1)
}
```

```rust [Rust]
// Fragment from demo2.rs::TransportModel::register/add_constraints.
// Data source: build_companies/build_products; helper imports come from demo2.rs.
let x_shape = Shape::new([companies.len(), products.len()]);
let x_vars: VariableCombination2D<Binary> =
    VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
        format!("{}_{}", vector[0], vector[1])
    });
let x_idx = model.register_combination(&x_vars)?;
let cost = flat_map1_indexed(
    "cost",
    companies,
    |c, company| {
        let monomials: Vec<_> = products
            .iter()
            .enumerate()
            .map(|(p, _)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    company.cost_of(p),
                    x_idx[&[c, p]],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, company| company.name.clone(),
);
model.add_symbol_combination(&cost)?;
let assignment_company = flat_map1_indexed(
    "assign_company",
    companies,
    |c, _company| {
        let monomials: Vec<_> = products
            .iter()
            .enumerate()
            .map(|(p, _)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    1.0,
                    x_idx[&[c, p]],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, company| company.name.clone(),
);
model.add_symbol_combination(&assignment_company)?;
let assignment_product = flat_map1_indexed(
    "assign_product",
    products,
    |p, _product| {
        let monomials: Vec<_> = companies
            .iter()
            .enumerate()
            .map(|(c, _)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    1.0,
                    x_idx[&[c, p]],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, product| product.name.clone(),
);
model.add_symbol_combination(&assignment_product)?;
let mut cost_coeffs = Vec::new();
for c in 0..companies.len() {
    let poly = cost.symbol_polynomial(c);
    for monomial in poly.monomials() {
        cost_coeffs.push((monomial.var_index(), *monomial.coefficient()));
    }
}
let cost_input = LinearObjectiveInput::minimize("cost").terms(cost_coeffs.into_iter());
model.set_linear_objective_input(cost_input);
for c in 0..companies.len() {
    let coeffs = extract_coeffs(&transport.assignment_company[c]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::LessEqual,
        1.0,
        &format!("company_{}", c),
    )?;
}
for p in 0..products.len() {
    let coeffs = extract_coeffs(&transport.assignment_product[p]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::Equal,
        1.0,
        &format!("product_{}", p),
    )?;
}
```

:::

#### 源码与验证

- [Rust 对照实现：demo2.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo2.rs)

Rust 对照实现使用相同的数学模型和数据，但 Rust 的 MetaModel、变量组合和符号组合 API 与 Kotlin API 独立。

- [当前实现：Demo2.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo2.kt)
- [Core 结构构建测试：CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

源码是二元分配模型，不是数量分配模型。四种产品通过 Kotlin AutoIndexed 编号；Rust 使用显式向量索引。以上代码是建模摘录，不是可直接运行的完整程序。

---

## 13. 变更日志

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型模板重组示例，补充分配约束、断言及 Kotlin/Rust 片段。 | 明确组合覆盖范围和 API 边界。 |
