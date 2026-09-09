# Bandwidth 上下文领域模型

> [English](../../../examples/framework-example1/domain-bandwidth/domain-model) | 中文

[toc]

## 1. 概述

Bandwidth 上下文消费 Route 上下文的图、服务和分配聚合；它注册每条边/每个服务的带宽变量、带宽中间值、需求与容量管线以及带宽成本目标。权威实现是 [`BandwidthContext.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/BandwidthContext.kt)、[`EdgeBandwidth.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/model/EdgeBandwidth.kt) 和 [`PipelineListGenerator.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/service/PipelineListGenerator.kt)。

### 1. 依赖上下文

1. Route 上下文 — 提供 `Graph`、`Service`、分配变量 $x_{n,s}$ 以及服务/节点分配中间值 $A_s$ 和 $A_n$。

---

## 2. 概念 / 实体

### 1. 边 (Edge)

来自 Route 上下文图的有向边。只有源节点为普通节点的边可以在本上下文中获得非零带宽变量。

**$from(e)$**：边 $e$ 的源节点。
**$to(e)$**：边 $e$ 的目标节点。
**$maxBandwidth_{e}$**：边 $e$ 的最大带宽上界，无符号整数。
**$costPerBandwidth_{e}$**：边 $e$ 的单位带宽成本，无符号整数。

### 2. 服务 (Service)

来自 Route 上下文、需要在图边上分配流量的服务。

**$capacity_{s}$**：服务 $s$ 的带宽容量，无符号整数。
**$cost_{s}$**：Route 上下文目标使用的服务成本，无符号整数。

### 3. 节点 (Node)

Route 上下文图节点，可以是普通节点或客户端节点。

**$demand_{n}$**：入带宽需求，仅对客户端节点定义。
**$x_{n,s}$**：导入的 Route 上下文分配变量，本上下文不重新声明它。

### 4. 分配计数

Route 上下文暴露本上下文使用的已注册中间值。

**$A_s$**：分配给服务 $s$ 的普通节点数量。
**$A_n$**：分配给普通节点 $n$ 的服务数量（对客户端节点则为零多项式）。

---

## 3. 变量

### 1. 决策变量

**$y_{e,s}$**：边 $e$ 上分配给服务 $s$ 的带宽，无量纲非负无符号整数，其定义域和注册边界为：

$$
0 \le y_{e,s} \le maxBandwidth_e,\quad \forall e \in E^{normal},\ \forall s \in S;
\qquad y_{e,s}=0,\quad \forall e \in E^{client},\ \forall s \in S.
$$

Kotlin 声明是 `UIntVariable2("y", Shape2(edges.size, services.size))`；`EdgeBandwidth.register` 按边设置范围。Route 上下文变量 $x_{n,s}$ 是导入的决策变量，不是 Bandwidth 上下文的声明。

### 2. 辅助变量

没有另行声明的辅助决策变量。第 6 节的带宽合计以及节点/服务流量都是已注册的线性中间符号。

---

## 4. 谓词

### 1. 节点类型

> 谓词用于对实体集合分类；每个谓词定义一个子集。

**$normal(n)$**：节点 $n$ 是 `NormalNode`。
**$client(n)$**：节点 $n$ 是 `ClientNode`。

### 2. 边关联

**$from(e)=n$**：边 $e$ 从节点 $n$ 离开。
**$to(e)=n$**：边 $e$ 进入节点 $n$。
**$from\_normal(e)$**：$e$ 的源节点满足 $normal$；这是所有生效边带宽与成本管线使用的派生过滤条件。

---

## 5. 集合

### 1. 节点

**$N$**：Route 上下文图中的节点全集。

**$N^{normal}$**：满足 $normal$ 的普通/中转节点。
**$N^{client}$**：满足 $client$ 的客户端/终端节点。

### 2. 边

**$E$**：有向图边全集。

**$E^{normal}$**：子集 $\{e \in E \mid from(e) \in N^{normal}\}$，其 $y_{e,s}$ 变量的注册上界是边最大带宽。
**$E^{client}$**：子集 $E \setminus E^{normal}$，其 $y_{e,s}$ 变量固定为零。源码在该范围设置中使用 `!from(normal)`。

### 3. 服务

**$S$**：Route 上下文生成的服务集合。

### 4. 实体对 / 关系

**$R_{inc}$**：由源/目标谓词 $from(e)$ 与 $to(e)$ 表示的边到节点关联关系；没有注册独立的关系变量。

---

## 6. 中间值

### 1. 边总带宽

**描述**：`EdgeBandwidth.bandwidth` 对一条边上的所有服务带宽求和。对源节点非普通节点的边，实现使用零多项式。

$$
B_e = \begin{cases}
\displaystyle\sum_{s \in S} y_{e,s},& e \in E^{normal}\\
0,& e \in E^{client}
\end{cases}
\qquad \forall e \in E
$$

### 2. 服务入度带宽

**描述**：`ServiceBandwidth.inDegree` 是服务 $s$ 通过所有目标为 $n$ 的边进入节点 $n$ 的带宽。

$$
I_{n,s} = \sum_{e \in E:\,to(e)=n} y_{e,s},\qquad \forall n \in N,\ \forall s \in S
$$

### 3. 服务出度带宽

**描述**：`ServiceBandwidth.outDegree` 是服务 $s$ 离开普通节点的带宽；对客户端节点使用零多项式。

$$
O_{n,s} = \begin{cases}
\displaystyle\sum_{e \in E:\,from(e)=n} y_{e,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N,\ \forall s \in S
$$

### 4. 服务净流出量

**描述**：`ServiceBandwidth.outFlow` 在普通节点上用服务出度减去服务入度。它不是流守恒约束。

$$
F_{n,s} = \begin{cases}
O_{n,s}-I_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N,\ \forall s \in S
$$

### 5. 节点聚合入度带宽

**描述**：`NodeBandwidth.inDegree` 汇总每个图节点上所有服务的入带宽。

$$
I_n = \sum_{s \in S} I_{n,s},\qquad \forall n \in N
$$

### 6. 节点聚合出度带宽

**描述**：`NodeBandwidth.outDegree` 汇总普通节点上的服务出度；对客户端节点使用零多项式。

$$
O_n = \begin{cases}
\displaystyle\sum_{s \in S} O_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N
$$

### 7. 节点聚合净流出量

**描述**：`NodeBandwidth.outFlow` 汇总普通节点上的服务净流出量；对客户端节点使用零多项式。

$$
F_n = \begin{cases}
\displaystyle\sum_{s \in S} F_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N
$$

### 8. 候选最大出带宽容量（未注册符号）

**描述**：`Node.maxOutDegree()` 为未注册的 `TransferNodeBandwidthConstraint` 计算节点所连接出边的最大带宽之和。它是辅助计算值，不是 `Aggregation.register` 添加的中间符号。

$$
M_n = \sum_{e \in E:\,from(e)=n} maxBandwidth_e,\qquad \forall n \in N^{normal}
$$

---

## 7. 断言

> 断言描述由变量范围和符号构造保证的性质，不表示额外的管线约束。

### 1. 源节点非普通的边带宽为零

**描述**：`EdgeBandwidth.register` 的范围设置将源节点非普通的每个服务带宽变量固定为零。

$$
\forall e \in E^{client}\;\forall s \in S\;\bigl(y_{e,s}=0\bigr)
$$

### 2. 带宽非负且受边界限制

**描述**：每条源节点为普通节点的边，其变量都是不超过边最大带宽的无符号整数。

$$
\forall e \in E^{normal}\;\forall s \in S\;\bigl(y_{e,s}\in\mathbb{Z}_{\ge 0}\ \wedge\ y_{e,s}\le maxBandwidth_e\bigr)
$$

### 3. 净流出量是定义，不是守恒

**描述**：实现定义 $F_{n,s}=O_{n,s}-I_{n,s}$ 和 $F_n=\sum_sF_{n,s}$，但没有管线注册 $O_{n,s}=I_{n,s}$ 或其他流守恒等式。

$$
\forall n \in N^{normal}\;\forall s \in S\;\bigl(F_{n,s}=O_{n,s}-I_{n,s}\bigr)
$$

---

## 8. 约束

> 生效约束恰好是 `bandwidth_context/service/PipelineListGenerator.kt` 返回的管线。`service/limits` 中存在某个类，并不表示生成器返回它后该约束才生效。

### 1. 边带宽约束

**Edge Bandwidth Constraint [边带宽约束]**
**描述**：服务级分配计数控制每条源节点为普通节点的边上的带宽。服务未分配给任何普通节点时，其在每条此类边上的带宽为零；服务分配一次时，边变量范围仍将其限制在边最大值内。

$$
s.t.\quad (1-A_s)\,maxBandwidth_e+y_{e,s}\le maxBandwidth_e,
\qquad \forall e \in E^{normal},\ \forall s \in S
$$

**推论**：由于 Route 上下文约束给出 $A_s \le 1$，在生效定义域上该不等式等价于 $y_{e,s}\le maxBandwidth_e A_s$。

$$
(1-A_s)\,maxBandwidth_e+y_{e,s}\le maxBandwidth_e
\ \Longleftrightarrow\ y_{e,s}\le maxBandwidth_e A_s
$$

### 2. 需求约束

**Demand Constraint [需求约束]**
**描述**：每个客户端节点通过服务入带宽至少接收其声明的需求量。

$$
s.t.\quad I_n\ge demand_n,\qquad \forall n \in N^{client}
$$

### 3. 服务容量约束

**Service Capacity Constraint [服务容量约束]**
**描述**：普通节点上的服务净流出量受导入的节点-服务分配变量门控。代码允许净流出量为负；它只注册该上界不等式。

$$
s.t.\quad (1-x_{n,s})\,capacity_s+F_{n,s}\le capacity_s,
\qquad \forall n \in N^{normal},\ \forall s \in S
$$

**推论**：该生效不等式在代数上等价于 $F_{n,s}\le capacity_s x_{n,s}$。

$$
(1-x_{n,s})\,capacity_s+F_{n,s}\le capacity_s
\ \Longleftrightarrow\ F_{n,s}\le capacity_s x_{n,s}
$$

### 4. 传输节点带宽约束（未注册）

**Transfer Node Bandwidth Constraint [传输节点带宽约束]**
**描述**：`TransferNodeBandwidthConstraint` 类确实存在，并且会用节点出边最大值之和门控普通节点的聚合净流出量。但是 `PipelineListGenerator` 只返回 `EdgeBandwidthConstraint`、`DemandConstraint`、`ServiceCapacityConstraint` 和 `BandwidthCostObjective`，没有返回该类。因此下面的不等式在当前 Demo1 模型中不生效。

$$
s.t.\quad (1-A_n)\,M_n+F_n\le M_n,
\qquad \forall n \in N^{normal}
$$

**推论**：该候选约束只是单向的净流出门控，并不蕴含 $O_n=I_n$；当前带宽管线列表没有注册任何流守恒等式。

---

## 9. 目标函数（如适用）

**描述**：`BandwidthCostObjective` 最小化所有源节点为普通节点的边上的带宽总成本，包括生成的普通节点到客户端边。Route 上下文另外注册服务成本目标。

$$
\min\quad Z_{bandwidth}=\sum_{e \in E^{normal}} costPerBandwidth_e\,B_e
$$

---

## 10. 算法引用

求解后，本上下文唯一类似独立算法的操作是 `SolutionAnalyzer.kt` 中的 DFS 路径提取；它不是模型约束或目标。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| DFS 路径提取 | [`service/SolutionAnalyzer.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/service/SolutionAnalyzer.kt) | `BandwidthContext.analyze` | 读取正的分配和带宽 token，然后从已分配节点追踪不重复边，直到客户端节点。 |

---

## 11. 通用语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 带宽变量 | $y_{e,s}$ | 边 $e$ 上分配给服务 $s$ 的整数带宽。 |
| 边总带宽 | $B_e$ | 边 $e$ 上所有服务的带宽之和；源节点非普通时为零。 |
| 服务入度 | $I_{n,s}$ | 服务 $s$ 进入节点 $n$ 的带宽。 |
| 服务出度 | $O_{n,s}$ | 服务 $s$ 离开普通节点 $n$ 的带宽。 |
| 服务净流出量 | $F_{n,s}$ | 普通节点上的服务出度减服务入度。 |
| 节点入度 | $I_n$ | 按服务汇总的节点入带宽。 |
| 节点净流出量 | $F_n$ | 普通节点上按服务汇总的净流出量。 |
| 服务分配计数 | $A_s$ | 导入的服务 $s$ 被分配到的普通节点数量。 |
| 普通源边 | $E^{normal}$ | 源节点满足 `normal` 谓词的边。 |
| 候选节点容量 | $M_n$ | 出边最大带宽之和，仅由未注册的传输节点约束使用。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| 将 $y$ 声明为无符号整数数组并按边设置上界 | 使用实数或全局统一上界的数组 | 匹配 `UIntVariable2` 以及按边设置 `maxBandwidth` 范围的实现。 | 当前源码 |
| 将源节点非普通的变量固定为零 | 仅在后续约束中门控每条边 | 这是 `EdgeBandwidth.register` 中的显式定义域设置。 | 当前源码 |
| 对所有节点定义入度，而仅对普通节点定义出度/净流出量 | 三者都只对普通节点定义 | 匹配 `ServiceBandwidth` 和 `NodeBandwidth` 中 `flatMap` 的分支。 | 当前源码 |
| 保留传输节点门控类但不把它加入管线生成器 | 与其他带宽约束一起自动注册 | 当前管线生成器没有返回该类，因此其不等式不生效。 | 当前源码 |
| 不从净流出量定义隐式增加流守恒 | 注册诸如 $O_n=I_n$ 的等式 | 源码只定义差值，没有返回守恒管线。 | 当前源码 |

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| Current | 重写为 13 节、与源码一致的领域模型 | 用实际注册的变量、中间值、约束、目标和未注册类状态替换臆造的传输公式及错误分配门控。 |
