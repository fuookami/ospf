# Route 上下文领域模型

> [English](../../../examples/framework-example1/domain-route/domain-model) | 中文

[toc]

## 1. 概述

Route 上下文根据 Demo 输入构造 `Graph`、生成 `Service` 列表和服务到节点的分配聚合，并提供由 Bandwidth 上下文消费的路由变量与路由管线。权威实现是 Kotlin `demo1` 源码，重点包括 [`RouteContext.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/route_context/RouteContext.kt)、[`Assignment.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/route_context/model/Assignment.kt) 和 [`PipelineListGenerator.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/route_context/service/PipelineListGenerator.kt)。

### 1. 依赖上下文

无上游依赖。Bandwidth 上下文依赖本上下文提供的图、服务、分配变量以及分配中间值。

---

## 2. 概念 / 实体

### 1. 节点 (Node)

图中的顶点。节点要么是可承载服务的 `NormalNode`，要么是具有带宽需求的 `ClientNode`。Kotlin 对象身份用于区分图节点；不能假设输入客户端标识符与普通节点标识符在全局范围内唯一。

**$id_{n}$**：节点 $n$ 上保存的输入标识符，物理量为无符号整数，用作节点数据。
**$edges_{n}$**：`RouteContext.init` 连接到节点 $n$ 的有向出边可变列表。

#### 1.1 普通节点 (NormalNode)

按照从 $0$ 到 `normalNodeAmount - 1` 的索引创建的中转/承载节点。只有普通节点拥有自由的路由分配行，以及注册上界可为非零的出带宽变量。

#### 1.2 客户端节点 (ClientNode)

从 `ClientNodeDTO` 创建的终端节点；它消耗带宽，没有自由的服务分配行。

**$demand_{n}$**：客户端节点 $n$ 的所需入带宽，无符号整数，仅对 $n \in N^{client}$ 定义。

### 2. 边 (Edge)

有向图边。对每条输入 `EdgeDTO`，初始化会以相同的最大带宽和单位成本建立两个方向的边。对每个客户端，初始化还会从其关联的普通节点添加一条到客户端的边，该边最大带宽等于客户端需求且单位成本为零。

**$from(e)$**：有向边 $e$ 的源节点。
**$to(e)$**：有向边 $e$ 的目标节点。
**$maxBandwidth_{e}$**：边 $e$ 的最大带宽，无符号整数。
**$costPerBandwidth_{e}$**：边 $e$ 的单位带宽成本，无符号整数。

### 3. 服务 (Service)

可以分配给一个普通节点、并由依赖的 Bandwidth 上下文分配带宽的候选服务。

**$id_{s}$**：生成的服务标识符，无符号整数。
**$capacity_{s}$**：服务 $s$ 的带宽容量，无符号整数。在 `RouteContext.init` 中，每个生成服务都取客户端需求总和。
**$cost_{s}$**：服务 $s$ 的使用成本，无符号整数。在 Demo 输入中，每个生成服务都取 `input.serviceCost`。

### 4. 图 (Graph)

路由聚合的网络容器，保存节点列表和有向边列表。

**$nodes$**：所有普通节点和客户端节点对象的有序列表。
**$edges$**：所有生成的有向边对象的有序列表。

---

## 3. 变量

### 1. 决策变量

**$x_{n,s}$**：服务分配二元变量，无量纲，定义域为 $\{0,1\}$；$1$ 表示服务 $s$ 承载在普通节点 $n$ 上，$0$ 表示未承载，$\forall n \in N,\ \forall s \in S$。`Assignment.register` 将 $n \in N^{client}$ 的行固定为 $0$；因此真正未知的决策行是 $\forall n \in N^{normal},\ \forall s \in S$。

### 2. 辅助变量

没有另行声明的辅助决策变量。已注册的分配计数是第 6 节定义的中间符号。

---

## 4. 谓词

### 1. 节点类型

> 谓词用于对实体集合分类；每个谓词定义一个子集。

**$normal(n)$**：节点 $n$ 是 `NormalNode` 实例。
**$client(n)$**：节点 $n$ 是 `ClientNode` 实例。

### 2. 边关联

**$from(e)=n$**：边 $e$ 的源节点是 $n$；实现同时提供匹配指定节点或节点谓词的谓词函数。
**$to(e)=n$**：边 $e$ 的目标节点是 $n$。

---

## 5. 集合

### 1. 节点

**$N$**：`Graph.nodes` 中所有图节点对象的全集。

**$N^{normal}$**：满足 $normal$ 的子集，即可以承载服务的普通节点。
**$N^{client}$**：满足 $client$ 的子集，即带有需求、需要由带宽模型接收带宽的客户端节点。

### 2. 边

**$E$**：`Graph.edges` 中所有有向图边的全集，包括每条输入边生成的两个方向以及生成的普通节点到客户端边。

**$E^{normal}$**：满足 $from(e) \in N^{normal}$ 的边子集，即 Bandwidth 上下文允许其带宽变量非零的边。
**$E^{client}$**：子集 $E \setminus E^{normal}$，其带宽变量由 Bandwidth 上下文固定为零。

### 3. 服务

**$S$**：生成的候选服务全集。若输入 `normalNodeAmount` 为 $r$，则 `RouteContext.init` 创建 $\lfloor r/2\rfloor$ 个服务。

### 4. 实体对 / 关系

实现使用上述 $from(e)$ 与 $to(e)$ 边关联关系；没有注册单独的路由关系或路径集合。

---

## 6. 中间值

### 1. 节点分配计数

**描述**：节点上分配的服务数量。`Assignment.register` 对普通节点的二元变量行求和，并为客户端行安装零多项式。

$$
A_n = \begin{cases}
\displaystyle\sum_{s \in S} x_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N
$$

### 2. 服务分配计数

**描述**：服务 $s$ 被分配到的普通节点数量。该符号被服务分配约束使用，并被带宽边门控约束导入。

$$
A_s = \sum_{n \in N^{normal}} x_{n,s},\qquad \forall s \in S
$$

### 3. 生成的服务容量和数量

**描述**：以下是由输入推导出的实体属性，而不是模型符号。令 $D$ 为客户端需求总和，令 $r$ 为 `normalNodeAmount`。

$$
D = \sum_{n \in N^{client}} demand_n,\qquad |S| = \left\lfloor\frac{r}{2}\right\rfloor,\qquad capacity_s = D\quad \forall s \in S
$$

每个 $s \in S$ 的生成服务成本为 $cost_s = input.serviceCost$。

---

## 7. 断言

> 断言描述已注册路由聚合及其数据形状始终满足的性质。

### 1. 客户端分配行固定

**描述**：`Assignment.register` 将二元变量数组中每个客户端节点行固定为 false。

$$
\forall n \in N^{client}\;\forall s \in S\;\bigl(x_{n,s}=0\bigr)
$$

### 2. 客户端分配计数为零

**描述**：由于客户端行使用零多项式，所有客户端的节点分配中间值均为零。

$$
\forall n \in N^{client}\;\bigl(A_n=0\bigr)
$$

### 3. 分配计数受二元行界定

**描述**：应用路由管线前，这些求和是非负整数；第 8 节的生效约束施加上界 $A_n \le 1$ 和 $A_s \le 1$。代码没有注册要求每个服务都必须分配的等式。

$$
\forall n \in N^{normal}\;\forall s \in S\;\bigl(x_{n,s}\in\{0,1\}\bigr)
$$

---

## 8. 约束

> 约束是 `route_context/service/PipelineListGenerator.kt` 返回的路由管线；分配变量的定义域由 `Assignment.register` 在管线执行前注册。

### 1. 节点分配约束

**Node Assignment Constraint [节点分配约束]**
**描述**：每个普通节点至多承载一个服务。

$$
s.t.\quad A_n \le 1,\qquad \forall n \in N^{normal}
$$

### 2. 服务分配约束

**Service Assignment Constraint [服务分配约束]**
**描述**：每个服务至多分配给一个普通节点。在本上下文中服务可以保持未分配；客户端需求和边门控由依赖的 Bandwidth 管线处理。

$$
s.t.\quad A_s \le 1,\qquad \forall s \in S
$$

---

## 9. 目标函数（如适用）

**描述**：`ServiceCostObjective` 最小化服务被分配时产生的服务成本总和。本上下文还会与依赖上下文注册的带宽目标一起注册目标；源码没有说明二者采用加权或词典序组合。

$$
\min\quad Z_{route}=\sum_{s \in S} cost_s A_s
$$

---

## 10. 算法引用

Route 上下文领域模型没有引用独立算法文档。路由初始化和管线构造是 `RouteContext.kt` 与 `PipelineListGenerator.kt` 中的普通上下文操作。

| 算法名称 | 文件路径 | 引用位置 | 简要说明 |
|----------|----------|----------|----------|
| 无 | — | — | 没有注册独立的路由算法。 |

---

## 11. 通用语言

| 术语 | 符号 | 定义 |
|------|------|------|
| 节点 | $n$ | 图节点对象。 |
| 普通节点 | $N^{normal}$ | 可承载服务的中转节点。 |
| 客户端节点 | $N^{client}$ | 具有入带宽需求的终端节点。 |
| 边 | $e$ | 有向图边。 |
| 服务 | $s$ | 生成的候选服务。 |
| 分配 | $x_{n,s}$ | 将服务放置到普通节点上的二元决策。 |
| 节点分配计数 | $A_n$ | 节点 $n$ 上分配的服务数量。 |
| 服务分配计数 | $A_s$ | 分配服务 $s$ 的普通节点数量。 |
| 需求 | $demand_n$ | 客户端节点 $n$ 所需的入带宽。 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| 每条输入边物化为两个有向边 | 保留输入无向边 | 源码显式构造并索引两个方向。 | 当前源码 |
| 添加一条容量为需求、成本为零的普通节点到客户端边 | 只用节点等式表达需求 | 当前图通过显式边让入带宽满足每个客户端。 | 当前源码 |
| 生成 $\lfloor r/2\rfloor$ 个容量等于客户端需求总和的服务 | 从独立输入字段读取服务数量和容量 | 这是 `RouteContext.init` 的精确 Demo 规则。 | 当前源码 |
| 将 $x$ 的客户端行固定为零 | 让所有节点行自由 | 实现中的客户端是消费者而不是服务承载节点。 | 当前源码 |

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| Current | 重写为 13 节、与源码一致的领域模型 | 用实际 Kotlin 注册语义替换简略内容和未被支持的路由公式。 |
