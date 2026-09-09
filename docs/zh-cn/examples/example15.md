# 示例 15：带替代关系的多工厂分销

## 一、概述

本限界上下文描述三家制造商向两个配送中心分发四种车型，并允许按方向使用车型替代比例。当前 Kotlin 源码数据如下：

| 配送中心 | M1 需求 | M2 需求 | M3 需求 | M4 需求 | 有向替代记录 |
| :---: | ---: | ---: | ---: | ---: | :--- |
| 丹佛 | 700 | 500 | 500 | 600 | M1→M2：0.10；M2→M1：0.10；M3→M4：0.20；M4→M3：0.20 |
| 迈阿密 | 600 | 500 | 200 | 100 | M1→M2：0.10；M2→M1：0.10；M2→M4：0.05；M4→M2：0.05 |

制造商的车型产能映射和物流成本如下：

| 制造商 | 可生产车型及产能 | 丹佛成本 | 迈阿密成本 |
| :---: | :--- | ---: | ---: |
| 洛杉矶 | M3：700；M4：300 | 80 | 215 |
| 底特律 | M1：500；M2：600；M4：400 | 100 | 108 |
| 新奥尔良 | M1：800；M2：400 | 102 | 68 |

替代率使用小数。Kotlin 源码计算了调整后需求，但当前约束没有使用该表达式；这一差异是本页模型契约的一部分。

### 1. 依赖上下文

无。车型、替代记录、制造商产能、中心需求和物流成本均直接定义在 `Demo15` 中。

---

## 二、概念/实体

### 1. 车型

车型是配送中心可提出需求、制造商可生产的产品类型。

**$Name_k$**：车型 $k$ 的名称（M1–M4）。

### 2. 配送中心

配送中心保存按车型的需求量和有向替代规则列表。

**$Demand_{dk}$**：中心 $d$ 对车型 $k$ 的需求量；Kotlin 调整后需求表达式将缺失的需求映射项视为零。

**$R_d$**：属于中心 $d$ 的有向替代记录集合。

### 3. 替代规则

替代规则允许在给定比例上限内用目标车型满足部分源车型需求。

**$From_r$** 与 **$To_r$**：替代记录 $r$ 的源车型和目标车型。

**$Rate^{Max}_{dr}$**：中心 $d$ 的记录 $r$ 的最大替代比例。

### 4. 制造商

制造商生产选定车型，并保存到各配送中心的物流成本。

**$Productivity_{mk}$**：制造商 $m$ 的车型 $k$ 产能映射值。

**$c_{md}$**：制造商 $m$ 向中心 $d$ 发运每单位货物的物流成本。

---

## 三、变量

### 1. 决策变量

**$x_{mdk}$**：制造商 $m$ 向中心 $d$ 发运车型 $k$ 的数量，是发运量，取值范围为 $\mathbb{Z}_{\ge 0}$，对每个 $(m,d,k)\in P\times D\times M$ 定义。当源码没有制造商—车型 $(m,k)$ 的 `productivity` 映射时，Kotlin 将该变量固定为零。

**$y_{dr}$**：中心 $d$ 按替代规则 $r$ 将源车型需求交由目标车型满足的比例，是无量纲连续量，取值范围为 $[0,Rate^{Max}_{dr}]$，$\forall d\in D,r\in R_d$。Kotlin 使用 `PctVariable1`，并按 `Replacement.maximum` 为每个分量设置上界。

### 2. 辅助变量

没有由求解器决定的辅助变量。`Receive`、`Trans`、`ExDemand` 和 `Cost` 都是中间表达式。

---

## 四、谓词

### 1. 产品与生产谓词

**HasDemand($d,k$)**：中心 $d$ 有车型 $k$ 的正数/已定义需求项。

**CanProduce($m,k$)**：制造商 $m$ 有车型 $k$ 的 `productivity` 映射项。

### 2. 替代谓词

**ReplacementFrom($r,k$)**：$From_r=k$，因此记录 $r$ 会减少车型 $k$ 的所需发运量。

**ReplacementTo($r,k$)**：$To_r=k$，因此记录 $r$ 会增加车型 $k$ 的所需发运量。

---

## 五、集合

### 1. 参与方

**$P$**：三家制造商组成的全集。

**$D$**：两个配送中心组成的全集。

**$M$**：四种车型组成的全集。

### 2. 替代与可生产集合

**$R_d$**：中心 $d$ 的有向替代规则集合；当两个方向都存在时，它们是两条独立记录。

**$H$**：可生产的制造商—车型关系：

$$
H=\{(m,k)\in P\times M\mid CanProduce(m,k)\}。
$$

它表示源码允许制造商向外发运非零数量的车型类型集合。

### 3. 实体对/关系

**$Y$**：替代变量索引关系：

$$
Y=\{(d,r)\mid d\in D,\ r\in R_d\}。
$$

**$H\times D$**：$x_{mdk}$ 的允许发运索引关系；源码仍分配完整三维数组，并将关系之外的条目固定为零。

---

## 六、中间值

### 1. 接收量

**描述**：`Receive` 是配送中心从所有制造商收到的某车型数量。

$$
Receive_{dk}=\sum_{m\in P}x_{mdk},\qquad \forall d\in D,\ k\in M。
$$

### 2. 发运量

**描述**：`Trans` 是制造商向所有配送中心发出的某车型数量。

$$
Trans_{mk}=\sum_{d\in D}x_{mdk},\qquad \forall m\in P,\ k\in M。
$$

### 3. 调整后需求

**描述**：`ExDemand` 是应用有向车型替代后的需求表达式。对于规则 $r$，$y_{dr}$ 是选定的替代比例。Kotlin 表达式把缺失的需求映射项按零处理。

$$
ExDemand_{dk}=Demand_{dk}
-\sum_{\substack{r\in R_d\\From_r=k}}Demand_{dk}\,y_{dr}
+\sum_{\substack{r\in R_d\\To_r=k}}Demand_{d,From_r}\,y_{dr},
\qquad \forall d\in D,\ k\in M。
$$

第一项求和扣除被其他车型替代的车型 $k$ 需求，第二项求和加入车型 $k$ 吸收的其他源车型需求。

### 4. 物流成本

**描述**：`Cost` 是所有制造商到配送中心发运量产生的物流成本总和。替代比例不出现在成本表达式中。

$$
Cost=\sum_{m\in P}\sum_{d\in D}\sum_{k\in M}c_{md}x_{mdk}。
$$

其中 $(m,k)\notin H$ 的项因对应 $x_{mdk}$ 已固定为零而不产生贡献。

---

## 七、断言

### 1. 替代保持总需求单位

**描述**：替代只在车型之间转移需求，不改变某配送中心所需的总单位数。

$$
\sum_{k\in M}ExDemand_{dk}=\sum_{k\in M}Demand_{dk},\qquad \forall d\in D。
$$

### 2. 零替代还原基础需求

**描述**：当某中心的所有替代比例为零时，每个车型的调整后需求等于原始需求。

$$
\left(\forall r\in R_d,\ y_{dr}=0\right)\Rightarrow
\left(\forall k\in M,\ ExDemand_{dk}=Demand_{dk}\right),\qquad \forall d\in D。
$$

### 3. 不可生产车型发运量为零

**描述**：制造商不能发运其源码生产映射中不存在的车型。

$$
\forall(m,k)\in(P\times M)\setminus H,\ \forall d\in D:\quad x_{mdk}=0。
$$

---

## 八、约束

### 1. 生产可用性固定 [Production Availability Fixing]

**描述**：没有 `productivity` 映射的制造商—车型组合，在每个目的配送中心上都固定为零。

$$
s.t.\quad x_{mdk}=0,\qquad \forall(m,k)\in(P\times M)\setminus H,\ \forall d\in D。
$$

### 2. 替代比例界限 [Replacement Rate Bounds]

**描述**：每条有向替代记录的比例非负，且不超过该中心的上限。

$$
s.t.\quad 0\le y_{dr}\le Rate^{Max}_{dr},\qquad \forall d\in D,\ r\in R_d。
$$

### 3. 当前需求覆盖 [Current Demand Coverage]

**描述**：当前 Kotlin 实现要求每个已定义需求项的接收量覆盖原始需求。这里没有使用 `ExDemand`。

$$
s.t.\quad Receive_{dk}\ge Demand_{dk},\qquad \forall(d,k)\text{ with }HasDemand(d,k)。
$$

### 4. 当前生产下界 [Current Production Lower Bound]

**描述**：当前 Kotlin 实现使用下界，要求发运量至少达到 `productivity` 映射值。这与通常的能力上限含义相反；Kotlin 没有注册生产能力上界约束。

$$
s.t.\quad Trans_{mk}\ge Productivity_{mk},\qquad \forall(m,k)\in H。
$$

**推论**：在当前 Kotlin 约束下，$y$ 既不影响可行性，也不影响目标；发运量没有被 `productivity` 从上方限制。采用 $Trans_{mk}\le Productivity_{mk}$ 并在需求约束中使用 $ExDemand$，会得到另一个实现。

---

## 九、目标函数

**描述**：最小化制造商到配送中心的物流成本；替代变量不计价。

$$
\min Cost=\min\sum_{m\in P}\sum_{d\in D}\sum_{k\in M}c_{md}x_{mdk}。
$$

当前 Kotlin core 结构构建测试只检查结构，不给出数值最优解。Rust 对照实现的成本方向相同，但其需求和产能约束见第十二节。

---

## 十、算法引用

没有引用独立算法文档；替代调整和线性表达式都直接在 core 示例中构建。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 源码内的替代算术和线性分销模型 |

---

## 十一、通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 制造商 | $P$ / $m$ | Manufacturer | 发运可生产车型的实体。 |
| 配送中心 | $D$ / $d$ | Distribution center | 具有需求和替代规则的目的地。 |
| 车型 | $M$ / $k$ | Car model | 被分销的产品类型。 |
| 发运量 | $x_{mdk}$ | Shipment | 制造商 $m$ 向中心 $d$ 发运的车型 $k$ 单位数。 |
| 替代比例 | $y_{dr}$ | Replacement rate | 按规则 $r$ 将源车型需求交由目标车型满足的比例。 |
| 调整后需求 | $ExDemand_{dk}$ | Adjusted demand | 应用有向替代算术后的需求。 |
| 产能 | $Productivity_{mk}$ | Productivity | 当前 Kotlin 下界约束使用的制造商—车型值。 |
| 物流成本 | $c_{md}$ | Logistics cost | 制造商 $m$ 到中心 $d$ 的单位成本。 |

---

## 十二、设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| 当前模型章节以 Kotlin 实现为准 | 推断业务上期望的能力语义 | 源码实际注册 `Receive ≥ Demand` 和 `Trans ≥ Productivity`；不能把 `ExDemand` 或 `Trans ≤ Productivity` 写成已启用。 | 2026-09-08 |
| 将有向替代记录分开处理 | 将相反方向合并成无向边 | Kotlin 独立存储每条 `Replacement(c1,c2,maximum)` 记录。 | 2026-09-08 |
| 将 Rust 作为不等价对照实现记录 | 把 Kotlin 和 Rust 写成同一个模型 | Rust 使用整数 `UInteger` 替代变量，所有小于 1 的上限都会使 $y=0$；同时使用 `Trans ≤ Productivity` 并约束 `Receive ≥ ExDemand`。 | 2026-09-08 |

---

## 十三、演进记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型章节重组页面，补充制造商数据，并分离 Kotlin 与 Rust 语义。 | 明确 Kotlin 未使用的 `ExDemand` 及跨语言模型差异。 |

---

## 当前模型构建最小示例

下面是模型构建片段。Kotlin 数据和 `flt64Converter` 来自 `Demo15.kt`；Rust 构建器、辅助函数和 `next_auto_intermediate_symbol_id` 来自 `demo15.rs`。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.math.symbol.polynomial.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

// manufacturers、distributionCenters、carModels 和 flt64Converter 来自 Demo15.kt。
val model = LinearMetaModel<Flt64>("demo15", converter = flt64Converter)
val x = UIntVariable3("x", Shape3(manufacturers.size, distributionCenters.size, carModels.size))
for (m in manufacturers) for (d in distributionCenters) for (k in carModels) {
    if (m.productivity.containsKey(k)) model.add(x[m, d, k])
    else x[m, d, k].range.eq(UInt64.zero)
}
val y = distributionCenters.associateWith { d ->
    PctVariable1("y_${d.name}", Shape1(d.replacements.size)).also { rates ->
        for ((r, replacement) in d.replacements.withIndex()) rates[r].range.leq(replacement.maximum)
    }
}
val receive = LinearIntermediateSymbols2<Flt64>("receive", Shape2(distributionCenters.size, carModels.size)) { _, v ->
    LinearExpressionSymbol(sum(x[_a, distributionCenters[v[0]], carModels[v[1]]]), name = "receive_${v.joinToString("_")}")
}
val demand = LinearIntermediateSymbols2<Flt64>("demand", Shape2(distributionCenters.size, carModels.size)) { _, v ->
    val d = distributionCenters[v[0]]; val k = carModels[v[1]]
    val removed = sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
        if (replacement.c1 == k && (d.demands[k] ?: UInt64.zero) gr UInt64.zero)
            d.demands[k]!!.toFlt64() * y[d]!![r] else null
    })
    val added = sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
        if (replacement.c2 == k) d.demands[replacement.c1]?.let { it.toFlt64() * y[d]!![r] } else null
    })
    LinearExpressionSymbol(LinearPolynomial((d.demands[k] ?: UInt64.zero).toFlt64()) - removed + added,
        name = "demand_${d.name}_${k.name}")
}
val trans = LinearIntermediateSymbols2<Flt64>("trans", Shape2(manufacturers.size, carModels.size)) { _, v ->
    LinearExpressionSymbol(sum(x[manufacturers[v[0]], _a, carModels[v[1]]]), name = "trans_${v.joinToString("_")}")
}
val cost = LinearExpressionSymbol(sum(manufacturers.flatMap { m -> distributionCenters.flatMap { d ->
    m.logisticsCost[d]?.let { c -> carModels.filter { m.productivity.containsKey(it) }.map { k -> c * x[m, d, k] } } ?: emptyList()
} }}), name = "cost")
model.add(x); model.add(y.values.flatten()); model.add(receive); model.add(demand); model.add(trans); model.add(cost)
model.minimize(cost, "cost")
for (d in distributionCenters) for (k in carModels) d.demands[k]?.let { model.addConstraint(receive[d, k] geq it) }
for (m in manufacturers) for (k in carModels) m.productivity[k]?.let { model.addConstraint(trans[m, k] geq it) }

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

```rust [Rust]
use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::{LinearExpressionSymbol, SymbolCombination};
use ospf_rust_core::variable::{UInteger, VariableCombination1D, VariableCombination3D, VariableRange};
use ospf_rust_multiarray::{MultiArray, Shape};

// build_car_models/build_centers/build_manufacturers and solve_typed are from demo15.rs.
let models = build_car_models();
let centers = build_centers();
let manufacturers = build_manufacturers();
let mut model = MetaModel::<f64>::new("demo15");
let x_vars = VariableCombination3D::with_name_and_range_generator(
    Shape::new([manufacturers.len(), centers.len(), models.len()]), "x",
    |_i, v| format!("{}_{}_{}", v[0], v[1], v[2]),
    |_i, v| if manufacturers[v[0]].productivity_by_model[v[2]].is_some() {
        VariableRange::with_lower(0.0)
    } else { VariableRange::fixed(0.0) });
let x_idx = model.register_combination(&x_vars)?;
let mut y_idx: Vec<MultiArray<usize, Shape<1>>> = Vec::new();
for d in 0..centers.len() {
    let y_vars = VariableCombination1D::with_name_and_range_generator(
        Shape::new([centers[d].replacements.len()]), &format!("y_{}", d),
        |_i, v| v[0].to_string(),
        |_i, v| VariableRange::bounded(0.0, centers[d].replacements[v[0]].max_ratio));
    y_idx.push(model.register_combination(&y_vars)?);
}
let cost = SymbolCombination::new(Shape::new([1]), "cost", |_i, _| {
    let terms = (0..manufacturers.len()).flat_map(|m| (0..centers.len()).flat_map(move |d| {
        (0..models.len()).map(move |k| ospf_rust_core::symbol::flatten::LinearMonomial::new(
            manufacturers[m].logistics_cost_to_centers[d], x_idx[&[m, d, k]]))
    })).collect();
    let p = ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0);
    LinearExpressionSymbol::new(ospf_rust_core::symbol::next_auto_intermediate_symbol_id(),
        "total_cost", p.monomials().to_vec(), *p.constant_term())
});
let trans = /* SymbolCombination<...> with sum_d x[m,d,k], as in demo15.rs */;
let receive = /* SymbolCombination<...> with sum_m x[m,d,k], as in demo15.rs */;
let demand = /* replacement delta: +Demand[from]*y for from, -Demand[from]*y for to */;
model.add_symbol_combination(&cost)?;
model.add_symbol_combination(&trans)?;
model.add_symbol_combination(&receive)?;
model.add_symbol_combination(&demand)?;
let coeffs = cost.symbol_polynomial(0).monomials().iter()
    .map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>();
model.add_linear_objective(&coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for d in 0..centers.len() { for k in 0..models.len() {
    let mut coeffs = extract_coeffs(&receive.symbol_polynomial_at(&[d, k]));
    coeffs.extend(extract_coeffs(&demand.symbol_polynomial_at(&[d, k])));
    model.add_linear_constraint(&coeffs, ConstraintRelation::GreaterEqual,
        centers[d].demands[k], &format!("demand_{}_{}", d, k))?;
}}
for m in 0..manufacturers.len() { for k in 0..models.len() {
    if let Some(cap) = manufacturers[m].productivity_by_model[k] {
        let coeffs = extract_coeffs(&trans.symbol_polynomial_at(&[m, k]));
        model.add_linear_constraint(&coeffs, ConstraintRelation::LessEqual, cap,
            &format!("capacity_{}_{}", m, k))?;
    }
}}
let _output = solve_typed(model)?;
```

:::

## 源码与验证

### Kotlin/Rust 对照

- [Rust 对照实现：`src/core/demo15.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo15.rs)

该对照实现与当前 Kotlin 模型不等价，页面对此明确区分。Rust 使用上限为 $0.05$、$0.10$ 或 $0.20$ 的整数 `UInteger` 替代变量；由于变量是整数，所有替代比例都被迫为 $0$。Rust 的需求符号只保存替代增量，并通过 `receive + demand ≥ base demand` 约束，代数上等价于 `Receive ≥ ExDemand`。Rust 还使用 `Trans ≤ Productivity`，而 Kotlin 当前使用 `Trans ≥ Productivity`。两者都最小化物流成本，并将不可生产的制造商—车型发运量固定为零。

- [当前实现：`Demo15.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo15.kt)
- [Core 结构构建测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

Kotlin core 结构构建测试只检查模型结构，不断言数值最优解。重新求解任一实现时，应使用该实现自己的约束解释结果。
