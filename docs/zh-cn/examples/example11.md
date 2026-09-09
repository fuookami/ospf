# 示例 11：最大流

## 1. 概览

本有界上下文在有向容量网络中最大化从根节点 0 到汇节点 8 的整数流量。Kotlin 与 Rust 的注册细节不同，但编码了相同的边集合、流量守恒和目标函数。

当前网络的边容量如下：

| 边 | 容量 | 边 | 容量 |
| :---: | ---: | :---: | ---: |
| $0\to1$ | 15 | $0\to2$ | 10 |
| $0\to3$ | 40 | $1\to4$ | 15 |
| $2\to5$ | 10 | $2\to6$ | 35 |
| $3\to6$ | 30 | $3\to7$ | 20 |
| $4\to6$ | 10 | $5\to8$ | 10 |
| $6\to7$ | 10 | $7\to8$ | 45 |

### 1. 依赖上下文

1. 无。网络流模型是自包含的。

---

## 2. 概念 / 实体

### 1. 节点（Node）

节点是网络顶点。样例包含一个根节点、七个中间节点和一个汇节点。

**$r$**：根/源节点，节点 0。

**$t$**：终点/汇节点，节点 8。

**$V^N$**：中间节点，除 $r$、$t$ 外的所有节点。

### 2. 有向边（Directed Edge）

有向边是具有有限容量的有向连接。

**$c_{ij}$**：边 $(i,j)$ 的容量，仅对已建模的边定义。

---

## 3. 变量

### 1. 决策变量

**$x_{ij}$**：边 $(i,j)$ 上的整数流量，单位为流量单位，定义域为 $mathbb{Z}_{\ge0}$，$\forall(i,j)\in E$。Kotlin 只注册 `capacities` 中出现的项；Rust 注册完整矩阵并将非边设为固定零范围。

**$f$**：源到汇的总流量，单位为流量单位，定义域为 $mathbb{Z}_{\ge0}$。Kotlin 使用标量 `UIntVar("flow")`；Rust 使用一个元素的 `VariableCombination1D<UInteger>`。

### 2. 辅助变量

**$In_i$** 和 **$Out_i$** 是由边流量派生的线性中间符号，不是独立决策变量。

---

## 4. 谓词

### 1. 网络成员谓词

**`IsEdge(i,j)`**：当 $(i,j)\in E$ 且容量映射存在该项时为真。

**`IsNormal(i)`**：当 $i\in V\setminus\{r,t\}$ 时为真。

---

## 5. 集合

### 1. 节点与边类别

**$V$**：节点全集，$V=\{0,1,\ldots,8\}$。

**$E$**：数据表列出的恰好十二条有向边。

**$V^N$**：中间节点子集，$V^N=V\setminus\{r,t\}$。

### 2. 实体对 / 关系

**$NetworkEdge$**：关系 $E$，连接边的尾节点与头节点。

非边 $(i,j)\notin E$ 在 Kotlin 中没有业务流量变量，在 Rust 中对应完整矩阵中的固定零项。

---

## 6. 中间值

### 1. 节点流入量

**说明**：沿已建模入边进入节点 $i$ 的总流量。

$$
In_i=\sum_{j:(j,i)\in E}x_{ji},\qquad \forall i\in V。
$$

### 2. 节点流出量

**说明**：沿已建模出边离开节点 $i$ 的总流量。

$$
Out_i=\sum_{j:(i,j)\in E}x_{ij},\qquad \forall i\in V。
$$

### 3. 流量平衡残差

**说明**：流出量减流入量；根节点为 $f$，汇节点为 $-f$，中间节点为零。

$$
Balance_i=Out_i-In_i=
\begin{cases}
f,&i=r,\\
-f,&i=t,\\
0,&i\in V^N。
\end{cases}
$$

---

## 7. 断言

### 1. 边容量支配流量

**说明**：任何已建模边的流量都不超过自身容量。

$$
\forall(i,j)\in E\;(0\le x_{ij}\le c_{ij})。
$$

### 2. 中间节点守恒

**说明**：中间节点不产生也不消耗净流量。

$$
\forall i\in V^N\;(Out_i=In_i)。
$$

### 3. 源汇流量一致

**说明**：同一个非负标量 $f$ 离开根节点并进入汇节点。

$$
Out_r-In_r=f\wedge In_t-Out_t=f\wedge f\ge0。
$$

### 4. 样例最大流

**说明**：给定网络的当前模型最大流为 $f=40$。一个可行见证是在 $0\to1\to4\to6\to7\to8$ 上发送 10，在 $0\to2\to5\to8$ 上发送 10，在 $0\to3\to7\to8$ 上发送 20，其余边为零。进入节点 7 的网络使 $7\to8$ 可用流量最多为 30，因此该见证达到最优。

$$
f^{*}=40。
$$

---

## 8. 约束

### 1. Edge Capacity（边容量约束）

**说明**：每条已建模边的流量非负且不能超过容量。Kotlin 在注册边项时设置上界，Rust 在范围生成器中编码上界。

$$
s.t. \quad 0\le x_{ij}\le c_{ij}，\qquad \forall(i,j)\in E。
$$

### 2. Root Balance（源点流量平衡约束）

**说明**：根节点的净流出量等于总流量变量。

$$
s.t. \quad Out_r-In_r=f。
$$

### 3. Sink Balance（汇点流量平衡约束）

**说明**：汇节点的净流入量等于同一个总流量变量。

$$
s.t. \quad In_t-Out_t=f。
$$

### 4. Normal-Node Conservation（中间节点守恒约束）

**说明**：每个中间节点的流入量和流出量相等。Kotlin 添加两个方向的不等式；Rust 合并两类表达式后添加一个 `Equal` 关系。

$$
s.t. \quad Out_i=In_i，\qquad \forall i\in V^N。
$$

实现形式为

$$
s.t. \quad Out_i\ge In_i\ \wedge\ Out_i\le In_i，\qquad \forall i\in V^N\quad\text{（Kotlin）}，
$$

或 Rust 中的一条等式。

### 5. Flow Domain（总流量定义域约束）

**说明**：总流量是非负整数。

$$
s.t. \quad f\in\mathbb{Z}_{\ge0}。
$$

---

## 9. 目标函数（如适用）

**说明**：最大化从根节点输送到汇节点的总流量。

$$
\max f。
$$

---

## 10. 算法引用

没有引用独立算法文档。这是标准的带容量最大流线性模型。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 页面直接记录优化公式。 |

---

## 11. 统一语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 根/源 | $r$ | 流量产生的节点 0。 |
| 汇/终点 | $t$ | 流量终止的节点 8。 |
| 边流量 | $x_{ij}$ | 已建模有向边上的整数流量。 |
| 容量 | $c_{ij}$ | 边流量的上界。 |
| 流入量 | $In_i$ | 进入节点的边流量之和。 |
| 流出量 | $Out_i$ | 离开节点的边流量之和。 |

---

## 12. 设计决策

| 决策 | 替代方案 | 理由 | 日期 |
|------|----------|------|------|
| 只把表中十二条边作为业务边 | 为每一对节点添加变量 | 源码容量映射定义了网络，缺失项不是边 | 2026-09-08 |
| 将中间节点守恒表达为等式 | 只保留一个方向的不等式 | Kotlin 的两个不等式与 Rust 的合并等式语义相同 | 2026-09-08 |
| 使用显式总流量变量 | 直接最大化根节点流出量 | 两端当前源码都公开 `flow` 模型变量 | 2026-09-08 |

### 当前最小模型构建片段

节点与容量数据来自 `Demo11.kt`/`demo11.rs`。代码标签页同时包含变量、流入/流出表达式、目标和所有用户层平衡约束；边容量由变量范围注册携带。

::: code-group
```kotlin [Kotlin]
// `nodes`、`capacities`、`rootNode`、`endNode` 和 `flt64Converter` 来自 Demo11.kt。
val metaModel = LinearMetaModel<Flt64>("demo11", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
for (from in nodes) for (to in nodes) {
    capacities[from]?.get(to)?.let { capacity ->
        x[from, to].range.leq(capacity)
        metaModel.add(x[from, to])
    }
}
val flow = UIntVar("flow")
metaModel.add(flow)
val flowIn = LinearIntermediateSymbols1<Flt64>("flow_in", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, i]), name = "flow_in_$i")
}
val flowOut = LinearIntermediateSymbols1<Flt64>("flow_out", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[i, _a]), name = "flow_out_$i")
}
metaModel.add(flowIn)
metaModel.add(flowOut)
metaModel.maximize(flow, "flow")
metaModel.addConstraint(flowOut[rootNode] - flowIn[rootNode] eq flow)
metaModel.addConstraint(flowIn[endNode] - flowOut[endNode] eq flow)
for (node in nodes.filterIsInstance<NormalNode>()) {
    metaModel.addConstraint(flowOut[node] geq flowIn[node])
    metaModel.addConstraint(flowOut[node] leq flowIn[node])
}
```

```rust [Rust]
// `data` 来自 demo11.rs 中的 MaxFlowData::sample()。
let node_count = data.nodes.len();
let mut model = MetaModel::<f64>::new("demo11");
let arc_vars = VariableCombination2D::<UInteger>::with_name_and_range_generator(
    Shape::new([node_count, node_count]), "x",
    |_index, vector| format!("{}_{}", vector[0], vector[1]),
    |_index, vector| data.capacities.iter()
        .find(|arc| arc.from == vector[0] && arc.to == vector[1])
        .map(|arc| VariableRange::bounded(0.0, arc.capacity))
        .unwrap_or_else(|| VariableRange::fixed(0.0)),
);
let arc_idx = model.register_combination(&arc_vars)?;
let flow_vars = VariableCombination1D::<UInteger>::new(Shape::new([1]), "flow");
let flow_idx = model.register_combination(&flow_vars)?;
model.add_linear_objective(&[(flow_idx[0], 1.0)], "flow");
model.set_objective_category(ObjectiveCategory::Maximum);
let flow_out = flat_map1_indexed("flow_out", &data.nodes, |node, _| {
    let monomials = (0..node_count).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, arc_idx[&[node, j]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |i, _| format!("{}", i));
let flow_in = flat_map1_indexed("flow_in", &data.nodes, |node, _| {
    let monomials = (0..node_count).map(|i|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, arc_idx[&[i, node]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |i, _| format!("{}", i));
model.add_symbol_combination(&flow_out)?;
model.add_symbol_combination(&flow_in)?;
// add_constraints() 组合 `flow_out - flow_in`：根节点加入 `-flow`，汇节点加入 `+flow`，
// 每个中间节点加入 Equal 0 守恒约束。
```
:::

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| 2026-09-08 | 按领域模型模板重组页面；补充带量词的中间值、双语约束名及等范围 Kotlin/Rust 标签页 | 对齐当前 Demo11 实现并保留两端守恒表达形式差异 |

## 源码与验证

- [Kotlin 实现：`Demo11.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo11.kt)
- [Rust 对照实现：`demo11.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo11.rs)
- [Kotlin 结构测试：`CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
