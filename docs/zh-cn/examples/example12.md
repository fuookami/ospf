# 示例 12：带风险和交易费用的投资组合

## 1. 概览

本有界上下文把整数资金分配到五种投资产品（包含无风险存款），在含费用预算和归一化风险上限下最大化净收益。本文记录当前函数符号机制及 Kotlin/Rust 的 Big-M 差异。

当前产品参数如下：

| 产品 | 收益率（$r_i$） | 风险（$q_i$） | 费率（$p_i$） | 最低费用（$m_i$） |
| :---: | ---: | ---: | ---: | ---: |
| P1 | 0.28 | 0.04 | 0.08 | 103 |
| P2 | 0.21 | 0.015 | 0.02 | 198 |
| P3 | 0.23 | 0.05 | 0.045 | 52 |
| P4 | 0.25 | 0.026 | 0.04 | 40 |
| P5（存款） | 0.05 | 0 | 0 | 0 |

### 1. 依赖上下文

1. 无。投资组合模型是自包含的。

---

## 2. 概念 / 实体

### 1. 投资产品（Investment Product）

每种产品接受整数投资金额，并具有收益率、风险率、比例费用率和最低费用。

**$r_i$**：产品 $i$ 的收益率。

**$q_i$**：产品 $i$ 的风险率/风险系数。

**$p_i$**：产品 $i$ 的比例交易费用率。

**$m_i$**：使用产品 $i$ 时收取的最低交易费用。

P5 是源码中的无风险存款：$q_5=p_5=m_5=0$，$r_5=0.05$。

### 2. 资金（Fund）

**$F$**：可用总资金，$F=1{,}000{,}000$ 元。

**$R^{Max}$**：归一化风险上限，$R^{Max}=0.02$。

---

## 3. 变量

### 1. 决策变量

**$x_i$**：投入产品 $i$ 的整数元数，单位为元，定义域为 $mathbb{Z}_{\ge0}$，$\forall i\in P$。Kotlin 使用 `UIntVariable1`，Rust 使用 `VariableCombination1D<UInteger>`。

### 2. 辅助变量

**$a_i$**：由 `BinaryzationFunction` 生成的二元投资标记，无量纲，意图是在 $x_i>0$ 时恰为 1。

**$Premium_i$**：由 `MaxFunction` 生成的交易费用函数输出，受到比例费用和最低费用表达式的下界约束。

---

## 4. 谓词

### 1. 投资状态

**`Invested(i)`**：当 $x_i>0$ 时为真。

**`RiskFree(i)`**：第五个源码产品满足此谓词，其风险和费用参数均为零。

### 2. 费用分支

**`RateFeeDominates(i)`**：当 $p_ix_i\ge m_ia_i$ 时为真。

**`MinimumFeeDominates(i)`**：当 $m_ia_i\ge p_ix_i$ 时为真。

---

## 5. 集合

### 1. 产品类别

**$P$**：五种投资产品全集。

**$P^{+}$**：已投资产品子集，$P^{+}=\{i\in P\mid Invested(i)\}$。

**$P^{RF}$**：无风险产品子集；样例中 $P^{RF}=\{5\}$。

### 2. 实体对 / 关系

不需要实体对关系；费用、风险和收益表达式均按单个产品索引。

---

## 6. 中间值

### 1. 投资标记

**说明**：二元函数标记产品 $i$ 是否获得正投资，由源码的 `BinaryzationFunction` 及其生成的机制约束实现。

$$
a_i=\begin{cases}
1,&x_i>0,\\
0,&x_i\le0，
\end{cases}
\qquad \forall i\in P。
$$

### 2. 交易费用

**说明**：费用取比例费用和由正投资激活的最低费用中的较大值。当前 `MaxFunction` 生成下界机制约束；预算和收益使最优解中的费用取紧。

$$
Premium_i=\max(p_ix_i,m_ia_i)，\qquad \forall i\in P。
$$

### 3. 归一化风险

**说明**：总风险暴露除以总资金，风险率使用源码提供的小数值。

$$
Risk=\frac{\sum_{i\in P}q_ix_i}{F}。
$$

### 4. 净收益

**说明**：产品总收益减去交易费用。

$$
Yield=\sum_{i\in P}(r_ix_i-Premium_i)。
$$

### 5. 预算占用

**说明**：投资本金与交易费用之和。Kotlin 在预算约束中直接写出该和；Rust 为同一表达式注册了 `funds` 符号组合。

$$
BudgetUsage=\sum_{i\in P}(x_i+Premium_i)。
$$

---

## 7. 断言

### 1. 投资标记为二元值

**说明**：每个产品的标记只能为 0 或 1，并遵循正投资谓词。

$$
\forall i\in P\;(a_i\in\{0,1\}\wedge(a_i=1\Longleftrightarrow x_i>0))。
$$

### 2. 费用下界

**说明**：每笔费用至少达到比例费用和激活的最低费用；最优解中取两者的最大值。

$$
\forall i\in P\;(Premium_i\ge p_ix_i\wedge Premium_i\ge m_ia_i\wedge Premium_i=\max(p_ix_i,m_ia_i))。
$$

等式是最优解语义；生成的 `MaxFunction` 机制负责提供下界。

### 3. 含费用预算

**说明**：包含交易费用在内的全部资金必须恰好分配完。

$$
BudgetUsage=F。
$$

### 4. 当前结果范围

**说明**：当前结构测试没有固定数值投资分配。旧页面只使用 S2/S4 的结果属于此前四产品表述，不能作为当前五产品模型的最优解发布。

---

## 8. 约束

### 1. Investment Indicator（投资标记约束）

**说明**：二值化机制把每个标记与整数投资是否为正关联起来。Kotlin 使用默认 Big-M 行为；Rust 显式传入 `funds` 作为 Big-M，并在风险系数中保护资金为零的情形。

$$
s.t. \quad a_i\in\{0,1\}，\quad a_i=1\Longleftrightarrow x_i>0，\qquad \forall i\in P。
$$

### 2. Maximum Transaction Fee（交易费用最大值约束）

**说明**：`MaxFunction` 机制施加两条候选费用下界。由于费用消耗预算且降低目标，最优解会选择可行的最小值。

$$
s.t. \quad Premium_i\ge p_ix_i，\quad Premium_i\ge m_ia_i，\qquad \forall i\in P。
$$

### 3. Fee-Inclusive Budget（含费用预算约束）

**说明**：投资本金加全部交易费用必须等于可用资金。

$$
s.t. \quad \sum_{i\in P}(x_i+Premium_i)=F。
$$

### 4. Risk Limit（风险上限约束）

**说明**：投资组合的归一化风险不能超过配置上限。

$$
s.t. \quad \frac{\sum_{i\in P}q_ix_i}{F}\le R^{Max}。
$$

### 5. Investment Domain（投资金额定义域约束）

**说明**：每个投资金额是非负整数元数。

$$
s.t. \quad x_i\in\mathbb{Z}_{\ge0}，\qquad \forall i\in P。
$$

---

## 9. 目标函数（如适用）

**说明**：最大化扣除交易费用后的净收益。

$$
\max Yield。
$$

---

## 10. 算法引用

函数机制在正文中直接引用；当前示例不需要独立算法文档。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| `BinaryzationFunction` | `ospf-kotlin-example/src/main/.../Demo12.kt` 与 `ospf-rust-example/src/core/demo12.rs` | 第 6.1、8.1 节 | 生成正投资活动的二元标记。 |
| `MaxFunction` | 同上 | 第 6.2、8.2 节 | 为两个线性费用表达式的最大值生成下界。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 投资金额 | $x_i$ | 分配给产品 $i$ 的整数本金。 |
| 投资标记 | $a_i$ | 正投资的二元标记。 |
| 交易费用 | $Premium_i$ | 产品 $i$ 产生的交易费用。 |
| 风险 | $Risk$ | 按资金归一化的风险暴露。 |
| 净收益 | $Yield$ | 总收益减费用。 |
| 无风险存款 | $P^{RF}$ | 源码中的 P5，风险和费用均为零。 |

---

## 12. 设计决策

| 决策 | 替代方案 | 理由 | 日期 |
|------|----------|------|------|
| 保留 P5 产品 | 只使用四种有风险产品 | P5 是两端当前源码明确提供的无风险产品 | 2026-09-08 |
| 预算包含交易费用 | 使用 $\sum_i(1+p_i)x_i=F$ | 旧公式忽略最低费用，也不能表示当前模型 | 2026-09-08 |
| 将机制语义与 API 细节分开记录 | 声称两端使用相同 Big-M | Kotlin 依赖默认 Big-M；Rust 调用 `with_big_m(..., funds)` 并保护零资金 | 2026-09-08 |

### 当前最小模型构建片段

产品数据、`funds` 和 `maxRisk` 来自 `Demo12.kt`/`demo12.rs`。两个标签页都包含投资变量、标记与费用函数、风险/收益表达式、目标、预算和风险约束；机制生成的不等式由源码负责。

::: code-group
```kotlin [Kotlin]
// `products`、`funds`、`maxRisk` 和 `flt64Converter` 来自 Demo12.kt。
val metaModel = LinearMetaModel<Flt64>("demo12", converter = flt64Converter)
val x = UIntVariable1("x", Shape1(products.size))
val assignment = LinearIntermediateSymbols1<Flt64>("assignment", Shape1(products.size)) { i, _ ->
    LinearFunctionSymbolAdapter(
        delegate = BinaryzationFunction(
            polynomial = LinearPolynomial(x[i]),
            converter = flt64Converter,
            name = "assignment_$i"
        ),
        converter = flt64Converter
    )
}
val premium = LinearIntermediateSymbols1<Flt64>("premium", Shape1(products.size)) { i, _ ->
    val product = products[i]
    LinearFunctionSymbolAdapter(
        delegate = MaxFunction(
            listOf(
                LinearPolynomial(product.premium * x[i]),
                LinearPolynomial(product.minPremium * assignment[i])
            ),
            converter = flt64Converter,
            name = "premium_$i"
        ),
        converter = flt64Converter
    )
}
val risk = LinearExpressionSymbol(
    sum(products.map { p -> p.risk * x[p] / funds }), name = "risk"
)
val yield = LinearExpressionSymbol(
    sum(products.map { p -> p.yield * x[p] - premium[p] }), name = "yield"
)
metaModel.add(x); metaModel.add(assignment); metaModel.add(premium)
metaModel.add(risk); metaModel.add(yield)
metaModel.maximize(yield, "yield")
metaModel.addConstraint(sum(products.map { p -> x[p] + premium[p] }) eq funds)
metaModel.addConstraint(risk leq maxRisk)
```

```rust [Rust]
// `products`、`funds` 和 `max_risk` 来自 demo12.rs。
let n = products.len();
let mut model = MetaModel::<f64>::new("demo12");
let x: VariableCombination1D<UInteger> =
    VariableCombination1D::new(Shape::new([n]), "x");
let x_idx = model.register_combination(&x)?;
let assignment_fn = SymbolCombination::new(Shape::new([n]), "assignment", |i, _| {
    BinaryzationFunction::with_big_m(
        i as u64 + 100, &products[i].name,
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[i])],
            0.0,
        ),
        funds,
    )
});
model.add_symbol_combination(&assignment_fn)?;
let premium_fn = SymbolCombination::new(Shape::new([n]), "premium", |i, _| {
    let assign_idx = assignment_fn.symbol_polynomial(i).monomials()[0].var_index();
    MaxFunction::new(
        i as u64 + 200, &products[i].name,
        vec![
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    products[i].premium_rate, x_idx[i],
                )], 0.0,
            ),
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    products[i].min_premium, assign_idx,
                )], 0.0,
            ),
        ], false,
    )
});
model.add_symbol_combination(&premium_fn)?;
let premium_idx: Vec<_> = (0..n).map(|i|
    premium_fn.symbol_polynomial(i).monomials()[0].var_index()
).collect();
let yield_expr = flat_map1_indexed("yield", products, |i, product| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(product.yield_rate, x_idx[i]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, premium_idx[i]),
    ], 0.0)
}, |_, product| product.name.clone());
let funds_expr = flat_map1_indexed("funds", products, |i, product| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[i]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, premium_idx[i]),
    ], 0.0)
}, |_, product| product.name.clone());
let risk_expr = flat_map1_indexed("risk", products, |i, product| {
    let coefficient = if funds.abs() < 1e-10 { 0.0 } else { product.risk_rate / funds };
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(coefficient, x_idx[i]),
    ], 0.0)
}, |_, product| product.name.clone());
model.add_symbol_combination(&yield_expr)?;
model.add_symbol_combination(&funds_expr)?;
model.add_symbol_combination(&risk_expr)?;
let mut yield_terms = Vec::new();
for i in 0..n {
    for monomial in yield_expr.symbol_polynomial(i).monomials() {
        yield_terms.push((monomial.var_index(), *monomial.coefficient()));
    }
}
let mut funds_terms = Vec::new();
for i in 0..n {
    for monomial in funds_expr.symbol_polynomial(i).monomials() {
        funds_terms.push((monomial.var_index(), *monomial.coefficient()));
    }
}
let mut risk_terms = Vec::new();
for i in 0..n {
    for monomial in risk_expr.symbol_polynomial(i).monomials() {
        risk_terms.push((monomial.var_index(), *monomial.coefficient()));
    }
}
model.add_linear_objective(&yield_terms, "yield");
model.set_objective_category(ObjectiveCategory::Maximum);
model.add_linear_constraint(&funds_terms, ConstraintRelation::Equal, funds, "funds")?;
model.add_linear_constraint(
    &risk_terms, ConstraintRelation::LessEqual, max_risk, "risk",
)?;
```
:::

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 2026-09-08 | 按领域模型模板重组页面；记录所有费用/风险中间值、机制约束、Big-M 差异及 Kotlin/Rust 标签页 | 对齐当前五产品 Demo12 实现，同时不发布未经验证的投资分配 |

## 源码与验证

- [Kotlin 实现：`Demo12.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo12.kt)
- [Rust 对照实现：`demo12.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo12.rs)
- [Kotlin 结构测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
