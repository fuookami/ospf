# 示例 8：设备工时约束下的生产

## 1. 概览

本有界上下文选择非负整数产量，在设备工时容量约束下最大化产品利润。本文遵循当前 `Demo8` 的数据与模型；代码标签页是模型构建片段，数据由源码文件提供。

五种产品利润为 $(123,94,105,132,118)$。设备数量与每件产品工时如下：

| 设备 | 数量 | P1 | P2 | P3 | P4 | P5 |
| :---: | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 12 | .23 | .44 | .17 | .08 | .36 |
| B | 14 | .13 | – | .20 | .37 | .19 |
| C | 8 | – | .25 | .34 | – | .18 |
| D | 6 | .55 | .72 | – | .61 | – |

每台设备最多提供 $2000$ 工时。短横线表示源码映射中缺少该系数。

### 1. 依赖上下文

1. 无。不需要外部领域上下文。

---

## 2. 概念 / 实体

### 1. 产品（Product）

产品可以按整数数量生产，并为每个生产单位贡献固定利润。

**$Profit_p$**：产品 $p$ 每生产一个单位带来的利润。

### 2. 设备类型（Equipment Type）

设备类型具有若干可用设备，并且生产每个产品会消耗该设备类型特定的工时。

**$Amount_e$**：设备类型 $e$ 的可用设备数量。

**$ManHours_{ep}$**：产品 $p$ 每生产一个单位所需的设备类型 $e$ 工时，仅在源码映射中存在该项时定义。

**$ManHours^{Max}$**：每台设备可提供的最大工时；当前值为 $2000$。

---

## 3. 变量

### 1. 决策变量

**$x_p$**：产品 $p$ 的产量，单位为产品件数，取非负整数，定义域为 $mathbb{Z}_{\ge0}$，$\forall p\in P$。Kotlin 源码使用 `UIntVariable1`，Rust 源码使用 `VariableCombination1D<UInteger>`。

### 2. 辅助变量

没有单独声明的辅助决策变量。`Profit` 和 `ManHours_e` 是注册的派生符号。

---

## 4. 谓词

### 1. 设备使用谓词

> 该谓词识别参与某个设备工时表达式的系数。

**`Uses(e,p)`**：当源码存在非缺失的 `e.manHours[p]` 系数时为真。缺失映射项不进入求和；Rust 数据用 `0.00` 表示并过滤零系数。

---

## 5. 集合

### 1. 产品与设备类别

**$P$**：产品全集。

**$E$**：设备类型全集。

**$P_e$**：对设备 $e$ 定义了工时系数的产品集合，$P_e=\{p\in P\mid \mathrm{Uses}(e,p)\}$。

### 2. 实体对 / 关系

**$EquipmentUse$**：关系 ${(e,p)\in E\times P\mid \mathrm{Uses}(e,p)\}$。

---

## 6. 中间值

### 1. 总利润

**说明**：所有产品产量产生的总利润。

$$
Profit=\sum_{p\in P}Profit_px_p。
$$

### 2. 设备工时

**说明**：设备类型 $e$ 被所有具有源码系数的产品消耗的总工时。该系数是时间量，不是货币成本。

$$
ManHours_e=\sum_{p\in P_e}ManHours_{ep}x_p，\qquad \forall e\in E。
$$

### 3. 设备容量

**说明**：设备类型 $e$ 的总可用工时，由设备数量乘以每台设备最大工时得到。

$$
Capacity_e=Amount_e\,ManHours^{Max}，\qquad \forall e\in E。
$$

`Capacity_e` 是文档层面的派生值；当前源码在每条约束中直接写出该乘积。

---

## 7. 断言

### 1. 缺失系数不消耗建模工时

**说明**：设备映射中不存在的产品不会出现在该设备的工时表达式中。

$$
\forall e\in E,\ p\notin P_e\;\Longrightarrow\;[x_p]ManHours_e=0。
$$

### 2. 设备容量非负

**说明**：设备数量和每台设备最大工时均非负，因此每个设备容量非负。

$$
\forall e\in E\;(Amount_e\ge0\wedge ManHours^{Max}\ge0\Longrightarrow Capacity_e\ge0)。
$$

### 3. 历史求解观察

**说明**：此前一次运行报告 $(x_1,x_2,x_3,x_4,x_5)=(0,0,18771,19672,53431)$。当前结构测试只检查模型构建，因此该数值是参考结果而不是固定不变量。

---

## 8. 约束

### 1. Equipment Man-Hour Capacity（设备工时容量约束）

**说明**：某设备类型承担的生产工时不能超过所有可用设备提供的工时。

$$
s.t. \quad ManHours_e\le Amount_e\,ManHours^{Max}，\qquad \forall e\in E。
$$

### 2. Production Domain（产量定义域约束）

**说明**：每种产品产量都是非负整数。

$$
s.t. \quad x_p\in\mathbb{Z}_{\ge0}，\qquad \forall p\in P。
$$

---

## 9. 目标函数（如适用）

**说明**：最大化产品总利润。

$$
\max Profit。
$$

---

## 10. 算法引用

没有引用独立算法文档。模型通过当前 Kotlin `LinearMetaModel`/SCIP 路径或 Rust `MetaModel`/求解器路径注册为线性表达式。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 不需要独立算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 产品 | $p\in P$ | 被选择整数产量的产品。 |
| 设备 | $e\in E$ | 具有有限数量和工时的资源类型。 |
| 利润 | $Profit$ | 生产带来的总价值。 |
| 工时 | $ManHours_e$ | 设备类型 $e$ 被消耗的工时。 |
| 容量 | $Capacity_e$ | 设备类型 $e$ 提供的总工时。 |

---

## 12. 设计决策

| 决策 | 替代方案 | 理由 | 日期 |
|------|----------|------|------|
| 保持产量为整数，且只通过工时约束间接限制上界 | 连续产量或手动添加产品上界 | 与当前 `UIntVariable1`/`UInteger` 一致 | 2026-09-08 |
| 忽略缺失的设备系数 | 将短横线视为显式业务惩罚 | Kotlin 映射省略键，Rust 过滤零系数 | 2026-09-08 |
| 保留 `ManHours` 命名 | 改名为 `Cost` | 系数的物理单位是工时，`Cost` 会错误表达含义 | 2026-09-08 |

### 当前最小模型构建片段

`products`、`equipments`/`equipment` 来自 `Demo8.kt` 和 `demo8.rs`；转换器及求解器设置也由源码提供。以下只展示注册符号和约束。

::: code-group
```kotlin [Kotlin]
// `products`、`equipments`、`maxManHours` 和 `flt64Converter` 来自 Demo8.kt。
val metaModel = LinearMetaModel<Flt64>("demo8", converter = flt64Converter)
val x = UIntVariable1("x", Shape1(products.size))
val profit = LinearExpressionSymbol(
    sum(products.map { p -> p.profit * x[p] }),
    name = "profit"
)
val manHours = LinearIntermediateSymbols1<Flt64>(
    "man_hours",
    Shape1(equipments.size)
) { i, _ ->
    val e = equipments[i]
    LinearExpressionSymbol(
        sum(products.mapNotNull { p -> e.manHours[p]?.let { it * x[p] } }),
        name = "man_hours_${e.index}"
    )
}
metaModel.add(x)
metaModel.add(profit)
metaModel.add(manHours)
metaModel.maximize(profit, "profit")
for (e in equipments) {
    metaModel.addConstraint(
        manHours[e] leq e.amount.toFlt64() * maxManHours,
        name = "eq_man_hours_${e.index}"
    )
}
```

```rust [Rust]
// `products` 和 `equipments` 来自 demo8.rs。
let mut model = MetaModel::<f64>::new("demo8");
let x: VariableCombination1D<UInteger> =
    VariableCombination1D::new(Shape::new([products.len()]), "x");
let x_idx = model.register_combination(&x)?;
let profit = flat_map1_indexed("profit", products, |i, product| {
    ospf_rust_core::symbol::flatten::Linear::new(
        vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
            product.profit, x_idx[i],
        )],
        0.0,
    )
}, |_, product| product.name.clone());
model.add_symbol_combination(&profit)?;
let man_hours = flat_map1("man_hours", equipments, |equipment| {
    let monomials = products.iter().enumerate().filter_map(|(p, _)| {
        let value = equipment.man_hours_by_product[p];
        (value != 0.0).then(||
            ospf_rust_core::symbol::flatten::LinearMonomial::new(value, x_idx[p])
        )
    }).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, equipment| equipment.name.clone());
model.add_symbol_combination(&man_hours)?;
let profit_coeffs = extract_coeffs(&profit[0]);
model.add_linear_objective(&profit_coeffs, "profit");
model.set_objective_category(ObjectiveCategory::Maximum);
for (e, equipment) in equipments.iter().enumerate() {
    model.add_linear_constraint(
        &extract_coeffs(&man_hours[e]), ConstraintRelation::LessEqual,
        equipment.amount * max_man_hours, &format!("equipment_{}_{}", e, equipment.name),
    )?;
}
```
:::

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 2026-09-08 | 按领域模型模板重组页面；补充带量词的中间值、双语约束名及 Kotlin/Rust 标签页 | 与当前 Demo8 实现保持一致 |

## 源码与验证

- [Kotlin 实现：`Demo8.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo8.kt)
- [Rust 对照实现：`demo8.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo8.rs)
- [Kotlin 结构测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
