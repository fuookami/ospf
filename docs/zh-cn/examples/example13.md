# 示例 13：带卡车数量的两阶段运输

## 一、概述

本限界上下文描述从三个配送中心向五个经销商发货的运输模型，同时满足中心供应量、经销商需求量和卡车容量约束。源码数据如下：

| 配送中心 | 经销商 1 | 经销商 2 | 经销商 3 | 经销商 4 | 经销商 5 | 供应量 |
| :---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DC1 | 100 | 150 | 200 | 140 | 35 | 400 |
| DC2 | 50 | 70 | 60 | 65 | 80 | 200 |
| DC3 | 40 | 90 | 100 | 150 | 130 | 150 |
| 经销商需求 | 100 | 200 | 150 | 160 | 140 | -- |

表中数值是各配送中心 `distance` 映射中的距离。每辆卡车容量为 $Q=18$ 个单位。当前数据的五个距离映射均完整，总供应量和总需求量均为 $750$。

### 1. 依赖上下文

无。本示例是源码内的运输模型，经销商、配送中心、距离和卡车容量数据均由 `Demo13` 直接提供。

---

## 二、概念/实体

### 1. 经销商

经销商是必须接收规定数量货物的需求点。

**$Demand_d$**：经销商 $d$ 的整数需求量，单位为发运单位。

### 2. 配送中心

配送中心提供货物，并保存到每个经销商的距离。

**$Supply_c$**：配送中心 $c$ 可提供的整数供应量，单位为发运单位。

**$distance_{cd}$**：配送中心 $c$ 到经销商 $d$ 的整数距离，用作该中心—经销商对的卡车成本系数。

### 3. 卡车分配

卡车分配记录某个配送中心—经销商对分配的卡车数，由决策变量 $y_{dc}$ 表示，不另建路线实体。

---

## 三、变量

### 1. 决策变量

**$x_{dc}$**：从配送中心 $c$ 发往经销商 $d$ 的数量，是发运量，取值范围为 $\mathbb{Z}_{\ge 0}$，对每个 $(d,c)\in D\times C$ 取一个整数值。

**$y_{dc}$**：从配送中心 $c$ 分配给经销商 $d$ 的卡车数，是无量纲整数计数，取值范围为 $\mathbb{Z}_{\ge 0}$，$\forall(d,c)\in D\times C$。源码使用 `UIntVariable2` 创建两个数组，索引顺序为 `[dealer, distributionCenter]`。

### 2. 辅助变量

没有辅助变量。`Trans`、`Receive` 和 `Cost` 是中间表达式，不是由求解器决定的辅助决策变量。

---

## 四、谓词

### 1. 运输参与方谓词

**Dealer($d$)**：$d$ 是具有需求量并接收货物的实体。

**Center($c$)**：$c$ 是具有供应量并发出货物的实体。

### 2. 路由数据谓词

**DistanceDefined($c,d$)**：源码距离映射包含中心—经销商对 $(c,d)$，因而该对具有成本系数。当前数据中该谓词对 $C\times D$ 的每个组合都成立。

---

## 五、集合

### 1. 经销商与配送中心

**$D$**：五个经销商组成的全集。

**$C$**：三个配送中心组成的全集。

### 2. 运输对

**$A$**：距离已定义的中心—经销商对集合：

$$
A=\{(d,c)\in D\times C\mid DistanceDefined(c,d)\}。
$$

在本实例中 $A=D\times C$；源码仍创建完整的二维变量数组。

### 3. 实体对/关系

**$A\subseteq D\times C$**：从配送中心指向经销商的有向发运关系。本示例不建模反向关系或路线顺序。

---

## 六、中间值

### 1. 中心发运总量

**描述**：`Trans` 是配送中心 $c$ 发往所有经销商的货物总量。

$$
Trans_c=\sum_{d:(d,c)\in A}x_{dc},\qquad \forall c\in C。
$$

### 2. 经销商接收总量

**描述**：`Receive` 是经销商 $d$ 从所有配送中心接收的货物总量。

$$
Receive_d=\sum_{c:(d,c)\in A}x_{dc},\qquad \forall d\in D。
$$

### 3. 卡车—距离成本

**描述**：`Cost` 对每辆已分配卡车收取一次对应距离，因此是按卡车计的距离目标，而不是按发运量计的距离目标。

$$
Cost=\sum_{(d,c)\in A}distance_{cd}\,y_{dc}。
$$

---

## 七、断言

### 1. 发运流量恒等式

**描述**：每个配送中心计入的发运单位都恰好被一个经销商计入接收单位。

$$
\sum_{c\in C}Trans_c=\sum_{d\in D}Receive_d=\sum_{(d,c)\in A}x_{dc}。
$$

### 2. 输入数据总量平衡

**描述**：本实例的总供应量恰好覆盖全部经销商需求量。

$$
\sum_{c\in C}Supply_c=400+200+150=750=100+200+150+160+140=\sum_{d\in D}Demand_d。
$$

### 3. 当前距离关系完整

**描述**：当前每个经销商—配送中心对都有距离系数，因此当前成本表达式没有遗漏数据对。

$$
A=D\times C,\qquad |D|=5,\qquad |C|=3。
$$

---

## 八、约束

### 1. 供应能力上限 [Supply Capacity]

**描述**：配送中心的发运量不能超过其可用供应量。

$$
s.t.\quad Trans_c\le Supply_c,\qquad \forall c\in C。
$$

### 2. 经销商需求满足 [Dealer Demand Coverage]

**描述**：每个经销商至少接收其声明的需求量。

$$
s.t.\quad Receive_d\ge Demand_d,\qquad \forall d\in D。
$$

### 3. 卡车容量联结 [Truck Capacity Linking]

**描述**：某中心—经销商对上的发运量不能超过已分配卡车的总容量。源码不建模卡车路线或拆分装载顺序。

$$
s.t.\quad x_{dc}\le Qy_{dc},\qquad \forall(d,c)\in A,\qquad Q=18。
$$

**推论**：由于总供应量和总需求量都为 $750$，上述不等式族在总量上必为等式；再结合非负流量，本实例中每个中心的供应量和每个经销商的需求量都被紧密满足。

$$
\sum_{c\in C}Trans_c=\sum_{c\in C}Supply_c=750，
\qquad
\sum_{d\in D}Receive_d=\sum_{d\in D}Demand_d=750。
$$

---

## 九、目标函数

**描述**：最小化各中心—经销商距离乘以对应卡车数后的总和。

$$
\min Cost=\min\sum_{(d,c)\in A}distance_{cd}\,y_{dc}。
$$

当前源码没有在目标中用 $x$ 替代 $y$；这样替换会得到不同的模型。

---

## 十、算法引用

没有引用独立算法文档。模型直接在所链接的 core 示例中构建。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 源码内的线性运输表达式与约束 |

---

## 十一、通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 经销商 | $D$ / $d$ | Dealer | 接收货物的需求点。 |
| 配送中心 | $C$ / $c$ | Distribution center | 发出货物的供应点。 |
| 发运量 | $x_{dc}$ | Shipment | 从中心 $c$ 发往经销商 $d$ 的单位数。 |
| 卡车数 | $y_{dc}$ | Truck count | 分配给中心—经销商对的整数卡车数。 |
| 卡车容量 | $Q$ | Truck capacity | 单辆卡车最大载量，本例 $Q=18$。 |
| 距离成本 | $Cost$ | Distance cost | 由距离加权卡车数并被最小化的成本。 |

---

## 十二、设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| 保留源码数组索引 `[dealer, distributionCenter]` | 改为 `[center, dealer]` | 记录实际 `UIntVariable2` 索引，同时让数学符号 $x_{dc}$ 保持中心到经销商的语义。 | 2026-09-08 |
| 通过 $y$ 按卡车收取距离成本 | 通过 $x$ 按发运量收取距离成本 | 当前 `Demo13` 的目标由 `y` 定义；修改会改变模型。 | 2026-09-08 |
| 保留供应量和需求量不等式 | 将两者改为等式 | 不等式是源码实际注册的约束；总量相等作为推论记录。 | 2026-09-08 |

---

## 十三、演进记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型章节重组页面，并记录当前 Kotlin/Rust core API。 | 对齐领域模型模板，同时保留实现特有语义。 |

---

## 当前模型构建最小示例

下面是模型构建片段。`dealers`、`distributionCenters`、`carCapacity`、`flt64Converter` 以及 Rust 数据/辅助构建函数均来自所链接的源码文件。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

// 数据和 flt64Converter 来自 Demo13.kt。
val model = LinearMetaModel<Flt64>("demo13", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(dealers.size, distributionCenters.size))
val y = UIntVariable2("y", Shape2(dealers.size, distributionCenters.size))
val trans = LinearIntermediateSymbols1<Flt64>("trans", Shape1(distributionCenters.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, distributionCenters[i]]), name = "trans_$i")
}
val receive = LinearIntermediateSymbols1<Flt64>("receive", Shape1(dealers.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[dealers[i], _a]), name = "receive_$i")
}
val cost = LinearExpressionSymbol(
    sum(dealers.flatMap { dealer -> distributionCenters.mapNotNull { center ->
        center.distance[dealer]?.let { it * y[dealer, center] }
    } }),
    name = "cost"
)
model.add(x); model.add(y); model.add(trans); model.add(receive); model.add(cost)
model.minimize(cost, "cost")
for (center in distributionCenters) model.addConstraint(trans[center] leq center.supply)
for (dealer in dealers) model.addConstraint(receive[dealer] geq dealer.demand)
for (dealer in dealers) for (center in distributionCenters) {
    model.addConstraint(x[dealer, center] - carCapacity.toFlt64() * y[dealer, center] leq Flt64.zero)
}

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

```rust [Rust]
use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::flat_map1_indexed;
use ospf_rust_core::variable::{UInteger, VariableCombination2D};
use ospf_rust_multiarray::Shape;

// build_dealers/build_centers, extract_coeffs, and solve_typed are from demo13.rs.
let dealers = build_dealers();
let centers = build_centers();
let capacity = 18.0;
let mut model = MetaModel::<f64>::new("demo13");
let shape = Shape::new([dealers.len(), centers.len()]);
let x_vars = VariableCombination2D::with_name_generator(shape.clone(), "x", |_i, v| {
    format!("{}_{}", v[0], v[1])
});
let y_vars = VariableCombination2D::with_name_generator(shape, "y", |_i, v| {
    format!("{}_{}", v[0], v[1])
});
let x_idx = model.register_combination(&x_vars)?;
let y_idx = model.register_combination(&y_vars)?;

let cost = flat_map1_indexed("cost", &dealers, |d, dealer| {
    let terms = (0..centers.len()).map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(
        dealer.distance_to(c), y_idx[&[d, c]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, dealer| dealer.name.clone());
let trans = flat_map1_indexed("trans", &centers, |c, _| {
    let terms = (0..dealers.len()).map(|d| ospf_rust_core::symbol::flatten::LinearMonomial::new(
        1.0, x_idx[&[d, c]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, center| center.name.clone());
let receive = flat_map1_indexed("receive", &dealers, |d, _| {
    let terms = (0..centers.len()).map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(
        1.0, x_idx[&[d, c]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, dealer| dealer.name.clone());
model.add_symbol_combination(&cost)?;
model.add_symbol_combination(&trans)?;
model.add_symbol_combination(&receive)?;
let cost_coeffs = (0..dealers.len()).flat_map(|i| cost.symbol_polynomial(i).monomials().iter()
    .map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>()).collect::<Vec<_>>();
model.add_linear_objective(&cost_coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for c in 0..centers.len() {
    model.add_linear_constraint(&extract_coeffs(&trans[c]), ConstraintRelation::LessEqual,
        centers[c].supply, &format!("supply_{}", c))?;
}
for d in 0..dealers.len() {
    model.add_linear_constraint(&extract_coeffs(&receive[d]), ConstraintRelation::GreaterEqual,
        dealers[d].demand, &format!("demand_{}", d))?;
}
for d in 0..dealers.len() { for c in 0..centers.len() {
    let terms = vec![(x_idx[&[d, c]], 1.0), (y_idx[&[d, c]], -capacity)];
    model.add_linear_constraint(&terms, ConstraintRelation::LessEqual, 0.0,
        &format!("truck_{}_{}", d, c))?;
}}
let _output = solve_typed(model)?;
```

:::

## 源码与验证

### Kotlin/Rust 对照

- [Rust 对照实现：`src/core/demo13.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo13.rs)

Rust 对照实现使用相同的五个经销商、三个配送中心数据、数组方向、卡车容量不等式和按卡车计距离目标；其 `MetaModel`、变量组合和符号组合 API 与 Kotlin 独立。

- [当前实现：`Demo13.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo13.kt)
- [Core 结构构建测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

当前 core 构建测试只检查模型结构，不断言数值分配或目标值。
