# 示例 14：多阶段分销

## 一、概述

本限界上下文描述包含生产节点、销售节点和中间转运节点的有向转运网络。生产能力、销售需求和源码 `unitCost` 弧映射如下：

| 节点类型 | 节点及能力/需求 |
| :---: | :--- |
| 生产节点（$P$） | 广州 $600$；大连 $400$ |
| 销售节点（$S$） | 南京 $200$；济南 $150$；南昌 $350$；青岛 $300$ |
| 转运节点（$T$） | 上海；天津 |

有向弧及单位成本严格取自源码映射；`unitCost` 中不存在的条目不是弧。

| 弧 | 成本 | 弧 | 成本 |
| :--- | ---: | :--- | ---: |
| 广州 → 上海 | 2 | 广州 → 天津 | 3 |
| 大连 → 青岛 | 4 | 大连 → 上海 | 3 |
| 大连 → 天津 | 1 | 上海 → 南京 | 2 |
| 上海 → 济南 | 6 | 上海 → 南昌 | 3 |
| 上海 → 青岛 | 6 | 天津 → 南京 | 4 |
| 天津 → 济南 | 4 | 天津 → 南昌 | 6 |
| 天津 → 青岛 | 5 |  |  |

### 1. 依赖上下文

无。节点类、弧数据以及全部能力/成本均由 `Demo14` 在本地提供。

---

## 二、概念/实体

### 1. 网络节点

网络节点是分销网络中的一个地点。Kotlin 源码通过密封接口 `Node` 以及 `Product`、`Sale`、`Distribution` 三个类表示三种角色。

**$Name_i$**：节点 $i$ 的显示名称。

**$Storage_i$**：生产节点 $i\in P$ 的生产/存储能力；转运节点和销售节点没有该能力字段。

**$Demand_i$**：销售节点 $i\in S$ 的需求量；生产节点和转运节点没有该需求字段。

### 2. 有向弧

有向弧把源节点连接到目标节点，并承载整数流量。

**$c_{ij}$**：弧 $(i,j)$ 上的单位运输成本，读取自 `unitCost` 或 Rust 的 `ArcData.unit_cost`。

**$x_{ij}$**：分配到弧 $(i,j)$ 的流量；其决策变量定义见第三节。

---

## 三、变量

### 1. 决策变量

**$x_{ij}$**：节点 $i$ 到节点 $j$ 的整数货物流量，是发运量，取值范围为 $\mathbb{Z}_{\ge 0}$，对每个 $(i,j)\in N\times N$ 定义。非弧条目固定为零；Kotlin 模型只注册弧条目。Kotlin 还对每条生产节点出弧施加 $x_{ij}\le Storage_i$ 的变量范围，而 Rust 通过节点约束表达相同的生产上限。

### 2. 辅助变量

没有由求解器决定的辅助变量。`Cost`、`Out` 和 `In` 都是中间表达式。

---

## 四、谓词

### 1. 节点角色谓词

**ProductNode($i$)**：节点 $i$ 是生产节点，具有存储/生产能力。

**SaleNode($i$)**：节点 $i$ 是销售节点，具有需求量。

**DistributionNode($i$)**：节点 $i$ 是中间转运节点。

### 2. 弧谓词

**ArcExists($i,j$)**：源码 `unitCost` 映射（或 Rust `arcs` 列表）包含有向弧 $(i,j)$。

---

## 五、集合

### 1. 网络节点

**$N$**：八个节点组成的全集。

**$P=\{i\in N\mid ProductNode(i)\}$**：生产节点集合，包括广州和大连，负责供应货物。

**$S=\{i\in N\mid SaleNode(i)\}$**：销售节点集合，包括南京、济南、南昌和青岛，负责接收需求量。

**$T=\{i\in N\mid DistributionNode(i)\}$**：转运节点集合，包括上海和天津，负责保持流量守恒。

### 2. 有向弧

**$E$**：有向弧集合：

$$
E=\{(i,j)\in N\times N\mid ArcExists(i,j)\}。
$$

如果 $(i,j)\in E$，模型不会自动推断反向弧；每个方向都必须显式列出。

### 3. 实体对/关系

**$E\subseteq N\times N$**：有向运输关系。非弧在模型注册前固定为零，因此不能承载流量。

---

## 六、中间值

### 1. 运输成本

**描述**：`Cost` 是所有定义弧上流量产生的单位成本总和。

$$
Cost=\sum_{(i,j)\in E}c_{ij}x_{ij}。
$$

### 2. 流出量

**描述**：`Out` 是从生产节点或转运节点流出的总量。由于非弧变量已固定为零，源码对所有目标求和与按弧求和等价。

$$
Out_i=\sum_{j:(i,j)\in E}x_{ij},\qquad \forall i\in P\cup T。
$$

### 3. 流入量

**描述**：`In` 是进入销售节点或转运节点的总量。源码使用流入方向 $x_{ji}$；若写成 $x_{ij}$，得到的将是流出量。

$$
In_i=\sum_{j:(j,i)\in E}x_{ji},\qquad \forall i\in S\cup T。
$$

---

## 七、断言

### 1. 节点角色划分

**描述**：每个节点恰好属于源码三个节点类中的一个。

$$
N=P\mathbin{\dot\cup}S\mathbin{\dot\cup}T,
\qquad P\cap S=S\cap T=P\cap T=\emptyset。
$$

### 2. 能力与需求数据平衡

**描述**：本实例的总生产能力等于总销售需求。

$$
\sum_{i\in P}Storage_i=600+400=1000
=200+150+350+300=\sum_{i\in S}Demand_i。
$$

### 3. 流量记账恒等式

**描述**：每条弧流量在其源节点计一次流出、在其目标节点计一次流入；转运平衡因此只转移流量，不产生或消灭流量。

$$
\sum_{i\in N}Out_i-\sum_{i\in N}In_i=0
$$

其中节点角色不适用的 `Out`/`In` 项视为零，非弧流量为零。

---

## 八、约束

### 1. 生产能力上限 [Production Capacity]

**描述**：每个生产节点发出的流量不能超过其存储/生产能力。

$$
s.t.\quad Out_i\le Storage_i,\qquad \forall i\in P。
$$

### 2. 销售需求满足 [Sales Demand Coverage]

**描述**：每个销售节点接收的流量至少达到其需求量。

$$
s.t.\quad In_i\ge Demand_i,\qquad \forall i\in S。
$$

### 3. 转运流量平衡 [Transshipment Flow Balance]

**描述**：转运节点不能积存或产生货物，流入量必须等于流出量。Kotlin 以两个不等式注册等式，Rust 以一个 `Equal` 关系注册等式。

$$
s.t.\quad In_i=Out_i,\qquad \forall i\in T。
$$

**推论**：结合第七节的总量平衡，能力和需求不等式在总量上都会取紧。

$$
\sum_{i\in P}Out_i=\sum_{i\in P}Storage_i=1000,
\qquad
\sum_{i\in S}In_i=\sum_{i\in S}Demand_i=1000。
$$

---

## 九、目标函数

**描述**：最小化列出的有向弧上的总运输成本。

$$
\min Cost=\min\sum_{(i,j)\in E}c_{ij}x_{ij}。
$$

对于当前数据，一个成本为 $4600$ 的流量方案如下：

| 弧 | 流量 |
| :--- | ---: |
| 广州 → 上海 | 550 |
| 广州 → 天津 | 50 |
| 大连 → 青岛 | 300 |
| 大连 → 天津 | 100 |
| 上海 → 南京 | 200 |
| 上海 → 南昌 | 350 |
| 天津 → 济南 | 150 |

其他定义弧流量均为零。这是列出弧成本下的可行最优方案记录，不是 Kotlin core 结构构建测试断言的数值结果。

---

## 十、算法引用

没有引用独立算法文档；本页描述的是源码内的转运模型。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 直接弧流量表达式与节点平衡约束 |

---

## 十一、通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 生产节点 | $P$ | Production node | 提供货物并受 `Storage` 限制的节点。 |
| 销售节点 | $S$ | Sales node | 按 `Demand` 要求接收流量的节点。 |
| 转运节点 | $T$ | Transshipment node | 流入和流出量相等的节点。 |
| 有向弧 | $E$ | Directed arc | 源到目标的、在数据中列出的运输连接。 |
| 弧流量 | $x_{ij}$ | Arc flow | 沿弧 $(i,j)$ 发送的整数货物量。 |
| 单位成本 | $c_{ij}$ | Unit cost | 弧 $(i,j)$ 上每个单位的成本。 |
| 总成本 | $Cost$ | Total cost | 单位成本乘弧流量后的总和。 |

---

## 十二、设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| 使用显式有向弧集 $E$ | 将所有节点对视为弧 | 匹配 `unitCost`/`ArcData`，避免不存在的直发关系。 | 2026-09-08 |
| 将 `In` 保持为 $\sum_jx_{ji}$ | 对两个方向都使用 $\sum_jx_{ij}$ | 转运平衡必须使用流入方向。 | 2026-09-08 |
| 保留两端等式实现差异 | 统一为一种 API 写法 | Kotlin 用两个不等式表达平衡，Rust 用 `Equal`；页面记录两者真实语义。 | 2026-09-08 |

---

## 十三、演进记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 1.0 | 按领域模型章节重组页面，并记录 Kotlin/Rust 弧流量实现。 | 对齐模板，同时保留当前数据和成本 $4600$ 的流量方案。 |

---

## 当前模型构建最小示例

下面是模型构建片段。`nodes`、`unitCost`、`Node` 子类、`arcs`、`NodeType` 以及辅助函数均来自所链接的源码文件。

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

// nodes、unitCost 和 flt64Converter 来自 Demo14.kt。
val model = LinearMetaModel<Flt64>("demo14", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
for (from in nodes) for (to in nodes) {
    if (unitCost[from]?.get(to) != null) {
        if (from is Product) x[from, to].range.leq(from.storage)
        model.add(x[from, to])
    } else {
        x[from, to].range.eq(UInt64.zero)
    }
}
val cost = LinearExpressionSymbol(
    sum(nodes.flatMap { from -> nodes.mapNotNull { to ->
        unitCost[from]?.get(to)?.let { it * x[from, to] }
    } }), name = "cost"
)
val out = LinearIntermediateSymbols1<Flt64>("out", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[nodes[i], _a]), name = "out_${nodes[i].name}")
}
val input = LinearIntermediateSymbols1<Flt64>("in", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, nodes[i]]), name = "in_${nodes[i].name}")
}
model.add(cost); model.add(out); model.add(input); model.minimize(cost)
for (node in nodes.filterIsInstance<Product>()) model.addConstraint(out[node] leq node.storage)
for (node in nodes.filterIsInstance<Sale>()) model.addConstraint(input[node] geq node.demand)
for (node in nodes.filterIsInstance<Distribution>()) {
    model.addConstraint(out[node] geq input[node])
    model.addConstraint(out[node] leq input[node])
}

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

```rust [Rust]
use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::flat_map1;
use ospf_rust_core::variable::{UInteger, VariableCombination2D, VariableRange};
use ospf_rust_multiarray::Shape;

// build_nodes/build_arcs、extract_coeffs 和 solve_typed 来自 demo14.rs。
let nodes = build_nodes();
let arcs = build_arcs();
let mut model = MetaModel::<f64>::new("demo14");
let x_vars = VariableCombination2D::with_name_and_range_generator(
    Shape::new([nodes.len(), nodes.len()]), "x", |_i, v| format!("{}_{}", v[0], v[1]),
    |_i, v| if arcs.iter().any(|a| a.from == v[0] && a.to == v[1]) {
        VariableRange::with_lower(0.0)
    } else { VariableRange::fixed(0.0) }
);
let x_idx = model.register_combination(&x_vars)?;
let cost = flat_map1("cost", &arcs, |arc| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(
            arc.unit_cost, x_idx[&[arc.from, arc.to]])
    ], 0.0)
}, |_, arc| format!("{}_{}", arc.from, arc.to));
let ids: Vec<usize> = (0..nodes.len()).collect();
let out = flat_map1("trans_out", &ids, |&i| {
    let terms = (0..nodes.len()).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, i| i.to_string());
let input = flat_map1("trans_in", &ids, |&j| {
    let terms = (0..nodes.len()).map(|i|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])).collect();
    ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0)
}, |_, j| j.to_string());
model.add_symbol_combination(&cost)?;
model.add_symbol_combination(&out)?;
model.add_symbol_combination(&input)?;
let coeffs = (0..arcs.len()).flat_map(|i| cost.symbol_polynomial(i).monomials().iter()
    .map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>()).collect::<Vec<_>>();
model.add_linear_objective(&coeffs, "cost");
model.set_objective_category(ObjectiveCategory::Minimum);
for (i, node) in nodes.iter().enumerate() {
    let out_coeffs = extract_coeffs(&out[i]);
    let in_coeffs = extract_coeffs(&input[i]);
    match node.kind {
        NodeType::Product(cap) => model.add_linear_constraint(
            &out_coeffs, ConstraintRelation::LessEqual, cap, &format!("product_{}", i))?,
        NodeType::Sale(demand) => model.add_linear_constraint(
            &in_coeffs, ConstraintRelation::GreaterEqual, demand, &format!("sale_{}", i))?,
        NodeType::Distribution => {
            let mut balance = out_coeffs;
            balance.extend(in_coeffs.into_iter().map(|(idx, c)| (idx, -c)));
            model.add_linear_constraint(&balance, ConstraintRelation::Equal, 0.0,
                &format!("balance_{}", i))?;
        }
    }
}
let _output = solve_typed(model)?;
```

:::

## 源码与验证

### Kotlin/Rust 对照

- [Rust 对照实现：`src/core/demo14.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo14.rs)

两端实现使用相同的八个节点、十三条有向弧、成本、能力、需求以及最小成本流语义；模型、变量组合和符号组合 API 相互独立。Kotlin 注册每条定义弧并对生产弧增加冗余的变量范围上限；Rust 通过 `VariableRange` 固定非弧，并在节点约束中施加生产能力。

- [当前实现：`Demo14.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo14.kt)
- [Core 结构构建测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

Kotlin core 构建测试只检查结构，不断言数值流量或成本 $4600$ 的结果。
