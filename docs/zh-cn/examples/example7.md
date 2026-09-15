# 示例 7：仓库到商店的运输

## 1. 概览

本有界上下文在仓库与商店之间分配整数发货量，在满足仓库容量和商店需求的同时最小化总运输成本。本文描述当前的 `Demo7` 模型；Kotlin 和 Rust 代码是模型构建片段，输入数据来自所链接的源码文件。

样例商店需求为 $(200,400,600,300)$，仓库容量为 $(510,470,520)$，单位运输成本如下：

| 仓库 | S1 | S2 | S3 | S4 |
| :---: | ---: | ---: | ---: | ---: |
| W1 | 12 | 13 | 21 | 7 |
| W2 | 14 | 17 | 8 | 18 |
| W3 | 10 | 11 | 9 | 15 |

### 1. 依赖上下文

1. 无。该示例是自包含的线性优化模型。

---

## 2. 概念 / 实体

### 1. 仓库（Warehouse）

仓库提供货物，具有总仓储容量以及到每家已建模商店的单位运输成本。

**$Stowage_w$**：仓库 $w$ 的容量，单位为发货单位。

**$Cost_{ws}$**：仓库 $w$ 到商店 $s$ 的单位运输成本；当源码成本映射中包含该对时定义。

### 2. 商店（Store）

商店接收货物并给出最低需求量。

**$Demand_s$**：商店 $s$ 的需求量。

---

## 3. 变量

### 1. 决策变量

**$x_{ws}$**：仓库 $w$ 运往商店 $s$ 的发货量，单位为发货单位，取非负整数，定义域为 $mathbb{Z}_{\ge 0}$，$\forall (w,s)\in A$。`Demo7` 使用 `UIntVariable2` 实现，没有添加逐弧上界。

### 2. 辅助变量

没有单独声明的辅助决策变量。第 6 节的 `Shipment_w`、`Purchase_s` 和 `Cost` 是由 $x$ 派生并注册的表达式/中间符号。

---

## 4. 谓词

### 1. 建模路线谓词

> 谓词用于分类实体集合；此谓词记录源码成本映射是否包含一条路线。

**`HasRoute(w,s)`**：当且仅当 `warehouse.cost` 为商店 $s$ 定义了系数时为真；只有这些有序对参与源码表达式。

---

## 5. 集合

### 1. 仓库与商店类别

**$W$**：仓库全集。

**$S$**：商店全集。

**$A$**：已建模的仓库-商店弧，$A=\{(w,s)\in W\times S\mid \mathrm{HasRoute}(w,s)\}$。

给定 Kotlin 数据中每个仓库都为每家商店定义了成本，因此 $A=W\times S$；Rust 源码也为表格中的每个单元保存一个成本。

### 2. 实体对 / 关系

**$ShipRoute$**：关系 $A$，连接可以服务某商店的仓库与商店。

---

## 6. 中间值

### 1. 总运输成本

**说明**：所有已选择发货的总成本；每个发货量乘以对应仓库-商店单位成本。

$$
Cost=\sum_{(w,s)\in A}Cost_{ws}x_{ws}。
$$

### 2. 仓库发货量

**说明**：仓库 $w$ 在所有已建模路线上的发货总量，并与该仓库容量比较。

$$
Shipment_w=\sum_{s:(w,s)\in A}x_{ws},\qquad \forall w\in W。
$$

### 3. 商店收货量

**说明**：商店 $s$ 从所有仓库收到的总量，并与该商店需求比较。

$$
Purchase_s=\sum_{w:(w,s)\in A}x_{ws},\qquad \forall s\in S。
$$

---

## 7. 断言

### 1. 样例数据的总需求可满足

**说明**：样例总容量足以覆盖总需求；这只是数据一致性检查，不能替代逐仓库约束。

$$
\sum_{s\in S}Demand_s=1500\le 1500=\sum_{w\in W}Stowage_w。
$$

### 2. 发货量记账非负

**说明**：每个已建模发货量及两个派生总量均为非负值。

$$
\forall (w,s)\in A\;(x_{ws}\ge0)\;\wedge\;\forall w\in W\;(Shipment_w\ge0)\;\wedge\;\forall s\in S\;(Purchase_s\ge0)。
$$

---

## 8. 约束

### 1. Warehouse Capacity（仓库容量约束）

**说明**：仓库发出的货物不能超过其仓储容量。

$$
s.t. \quad Shipment_w\le Stowage_w，\qquad \forall w\in W。
$$

### 2. Store Demand（商店需求约束）

**说明**：每家商店至少收到其需求量。当前源码不要求恰好等于需求量。

$$
s.t. \quad Purchase_s\ge Demand_s，\qquad \forall s\in S。
$$

### 3. Shipment Domain（发货量定义域约束）

**说明**：发货量是整数且不能为负。

$$
s.t. \quad x_{ws}\in\mathbb{Z}_{\ge0}，\qquad \forall (w,s)\in A。
$$

---

## 9. 目标函数（如适用）

**说明**：最小化总运输成本。给定数据中的成本均为非负值，因此无必要的超额配送不会带来收益，但“恰好满足需求”仍不是硬约束。

$$
\min Cost。
$$

---

## 10. 算法引用

没有引用独立算法文档。模型使用常规 `LinearMetaModel`/`MetaModel` 注册路径以及 `ScipLinearSolver`/Rust 求解器适配器。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 不需要独立算法。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 仓库 | $w\in W$ | 提供发货量的起点。 |
| 商店 | $s\in S$ | 具有最低需求的目的地。 |
| 发货量 | $x_{ws}$ | 已建模路线上的整数发货数量。 |
| 仓储容量 | $Stowage_w$ | 仓库的最大总发货量。 |
| 收货量 | $Purchase_s$ | 商店收到的总数量。 |

---

## 12. 设计决策

| 决策 | 替代方案 | 理由 | 日期 |
|------|----------|------|------|
| 使用非负整数发货变量 | 连续流量或二元路线选择 | 当前 Kotlin 源码使用 `UIntVariable2`，Rust 源码使用 `UInteger` | 2026-09-08 |
| 使用“至少满足需求”约束 | 使用恰好满足需求的等式 | 保留当前 `geq` API 和源码语义 | 2026-09-08 |
| Kotlin 与 Rust 代码分别展示 | 使用一个伪 API | 两端使用不同的元模型、组合变量和符号 API | 2026-09-08 |

### 当前最小模型构建片段

数据（`stores`、`warehouses`）和转换器来自 `Demo7.kt`；Rust 数据由 `demo7.rs` 中的 `build_warehouses()` 与 `build_stores()` 构造。以下只展示模型构建，不保证每个代码块可单独作为完整文件运行。

::: code-group
```kotlin [Kotlin]
// `stores`、`warehouses` 和 `flt64Converter` 来自 Demo7.kt。
val metaModel = LinearMetaModel<Flt64>("demo7", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(warehouses.size, stores.size))
metaModel.add(x)

val cost = LinearExpressionSymbol(
    sum(warehouses.map { w ->
        sum(stores.filter { w.cost.contains(it) }.map { s -> w.cost[s]!! * x[w, s] })
    }),
    name = "cost"
)
val shipment = LinearIntermediateSymbols1<Flt64>("shipment", Shape1(warehouses.size)) { i, _ ->
    val w = warehouses[i]
    LinearExpressionSymbol(
        sum(stores.filter { w.cost.contains(it) }.map { s -> x[w, s] }),
        name = "shipment_${w.index}"
    )
}
val purchase = LinearIntermediateSymbols1<Flt64>("purchase", Shape1(stores.size)) { i, _ ->
    val s = stores[i]
    LinearExpressionSymbol(
        sum(warehouses.filter { w -> w.cost.contains(s) }.map { w -> x[w, s] }),
        name = "purchase_${s.index}"
    )
}
metaModel.add(cost)
metaModel.add(shipment)
metaModel.add(purchase)
metaModel.minimize(cost, "cost")
for (w in warehouses) {
    metaModel.addConstraint(shipment[w] leq w.stowage, name = "stowage_${w.index}")
}
for (s in stores) {
    metaModel.addConstraint(purchase[s] geq s.demand, name = "demand_${s.index}")
}
```

```rust [Rust]
// `warehouses` 和 `stores` 来自 demo7.rs；这里是注册/约束片段。
let mut model = MetaModel::<f64>::new("demo7");
let x_vars: VariableCombination2D<UInteger> = VariableCombination2D::with_name_generator(
    Shape::new([warehouses.len(), stores.len()]),
    "x",
    |_index, vector| format!("{}_{}", vector[0], vector[1]),
);
let x_idx = model.register_combination(&x_vars)?;
let cost = flat_map1_indexed(
    "cost",
    warehouses,
    |w, warehouse| {
        let monomials = stores.iter().enumerate().map(|(s, _)|
            ospf_rust_core::symbol::flatten::LinearMonomial::new(
                warehouse.cost_to(s), x_idx[&[w, s]],
            )
        ).collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, warehouse| warehouse.name.clone(),
);
model.add_symbol_combination(&cost)?;
let shipment = flat_map1_indexed("shipment", warehouses, |w, _| {
    let monomials = stores.iter().enumerate().map(|(s, _)|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, warehouse| warehouse.name.clone());
let purchase = flat_map1_indexed("purchase", stores, |s, _| {
    let monomials = warehouses.iter().enumerate().map(|(w, _)|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, store| store.name.clone());
model.add_symbol_combination(&shipment)?;
model.add_symbol_combination(&purchase)?;
let mut cost_coeffs = Vec::new();
for w in 0..warehouses.len() {
    for monomial in cost.symbol_polynomial(w).monomials() {
        cost_coeffs.push((monomial.var_index(), *monomial.coefficient()));
    }
}
model.set_linear_objective_input(
    LinearObjectiveInput::minimize("cost").terms(cost_coeffs.into_iter())
);
for w in 0..warehouses.len() {
    model.add_linear_constraint(
        &extract_coeffs(&shipment[w]), ConstraintRelation::LessEqual,
        warehouses[w].stowage, &format!("stowage_{}", w),
    )?;
}
for s in 0..stores.len() {
    model.add_linear_constraint(
        &extract_coeffs(&purchase[s]), ConstraintRelation::GreaterEqual,
        stores[s].demand, &format!("demand_{}", s),
    )?;
}
```
:::

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 2026-09-08 | 按领域模型模板重组页面；补充带量词的中间值定义、双语约束名及 Kotlin/Rust 标签页 | 与当前 Demo7 实现保持一致 |

## 源码与验证

- [Kotlin 实现：`Demo7.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo7.kt)
- [Rust 对照实现：`demo7.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo7.rs)
- [Kotlin 结构测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
