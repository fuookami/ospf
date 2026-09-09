# Framework 示例 5：VRPTW 领域模型

[English](../../examples/framework-example5/domain-model.md)

本页是可运行 Demo5 应用的上下文级模型契约。完整实现审计见[示例总览](../framework-example5)，
本页按有界上下文组织较长的分支定价模型，便于导航和审阅。

## 1. 概述与上下文依赖

| 上下文 | 职责 | 依赖 |
| --- | --- | --- |
| 输入基础设施 | Demo17/Solomon 读取、单位转换、校验和 `VrptwInstance` 组装 | 外部样例/文本 |
| VRPTW 领域 | 客户、仓库、车辆类型、路线/停靠点、单位和容差 | 输入归一化后无依赖 |
| 策略/计算 | 距离、行驶时间、弧成本、路线成本和弧可行性 | VRPTW 领域 |
| route compilation | 路线列 RMP、人工覆盖、客户/机队表达式和阶段管线 | VRPTW 领域与路线列 |
| route generation | 初始路线、分支感知图、ESPPRC 标签和定价结果 | VRPTW 领域、策略和 RMP 影子价格 |
| application | 节点求解、阶段切换、分支、界和输出 | 编译与生成 |
| 求解器基础设施 | SCIP/Gurobi 列生成后端选择 | 应用配置 |

生产路径是基于路线的 RMP 加 ESPPRC 定价。`Demo5DirectMipOracle` 中的双下标 MIP 仅用于测试，
不是路线列模型的领域替代方案。

## 2. 概念、谓词与集合

令 `D` 为客户集合，`K` 为车辆类型集合，`R_k` 为类型 `k` 的路线集合，`N` 为路线节点（仓库和客户），
`A` 为可行有向弧集合。客户 `i` 具有需求 `q_i`、就绪时间 `ready_i`、截止时间 `due_i` 和服务时长
`service_i`；车辆类型 `k` 具有容量 `Q_k`、固定成本 `f_k` 和可用数量 `amount_k`。

谓词包括 `customer(n)`、`depot(n)`、`feasibleArc(p,q)`、`contains(i,r)`、`elementary(r)` 和
`branchCompatible(r,b)`。因此：

$$
R_k^{feas}=\{r\in R_k\mid elementary(r)\wedge capacity(r)\le Q_k
\wedge timeWindows(r)\}，
$$

分支节点的列池是在 require/forbid 掩码过滤后的 `R_{k,b}^{compatible}`。

## 3. 变量与中间值

对当前分支节点和列池：

$$
x_r\ge0\quad(r\in R_k^{compatible}),\qquad
a_i\ge0\quad(i\in D)。
$$

`x_r` 是 RMP 中的路线使用量；分支定价框架在整数 incumbent 上检查路线选择的整数性。
`a_i` 是第一阶段的人工覆盖变量。route compilation 暴露：

$$
cover_i=\sum_{r\ni i}x_r,
\qquad
fleet_k=\sum_{r\in R_k}x_r,
\qquad
routeCost_r=\sum_{(p,q)\in r}arcCost_{pq}+f_k。
$$

对于停靠点 `q` 及其前驱 `p`，定价中间值为：

$$
arrival_q=departure_p+travelTime_{pq},
\qquad
start_q=\max(arrival_q,ready_q),
\qquad
departure_q=start_q+service_q。
$$

载荷按客户需求累加，并受 `Q_k` 限制。

## 4. 断言与约束

第一阶段/第二阶段均使用的主问题约束为：

$$
s.t.\quad a_i+\sum_{r\ni i}x_r=1\qquad(i\in D)，
$$

$$
s.t.\quad \sum_{r\in R_k}x_r\le amount_k\qquad(k\in K)。
$$

每条定价路线必须满足：

$$
s.t.\quad start_i\le due_i，
\qquad
s.t.\quad load_r\le Q_k，
\qquad
s.t.\quad r\text{ 至多访问每个客户一次并遵守分支掩码}。
$$

这些是路线生成可行性断言，而不是额外的 RMP 行。只有 ESPPRC 报告不存在改进路线时定价才完成；
达到单次列上限表示定价不完整，不能报告为收敛。

## 5. 目标函数

第一阶段最小化人工覆盖：

$$
\min\sum_{i\in D}a_i。
$$

所有 `a_i` 固定为零后，第二阶段最小化路线成本：

$$
\min\sum_{k\in K}\sum_{r\in R_k}routeCost_r x_r。
$$

对于类型 `k` 的定价路线 `r`，框架使用的约简成本为：

$$
rc(r)=c^{phase}(r)-\sum_{i\in r\cap D}\pi_i-\mu_k，
$$

其中第一阶段真实路线的 `c^{phase}` 为零，第二阶段为归一化路线成本；`π_i` 与 `μ_k` 是客户和机队影子价格。

## 6. 分支定价生命周期

1. 校验并归一化输入实例。
2. 使用初始单客户路线和人工覆盖构造分支节点 RMP，求解第一阶段。
3. 提取影子价格，构造分支感知 ESPPRC 图，加入去重的负约简成本列。
4. 人工覆盖收敛后切换第二阶段，固定 `a_i=0` 并继续列生成。
5. 对车辆类型/客户分配和车辆类型/弧决策进行分支；按掩码过滤继承/新路线，在组装解前校验整数路线。

应用使用 best-bound 队列，并受时间、节点、间隙、定价列数和每节点迭代上限控制。这些是终止策略，不是数学约束。

## 7. 通用语言与设计决策

| 术语 | 符号 | 含义 |
| --- | --- | --- |
| customer | `i` | 具有需求的服务点 |
| vehicle type | `k` | 容量、固定成本和机队数量 |
| route/column | `r` | 可行的仓库到仓库客户序列 |
| artificial coverage | `a_i` | 用于启动覆盖的第一阶段变量 |
| RMP | `x_r` | 当前路线主问题 |
| pricing | `rc(r)` | ESPPRC 搜索改进路线 |
| branch mask | `b` | 节点继承的 require/forbid 决策 |

| 决策 | 备选 | 原因 |
| --- | --- | --- |
| 生产模型使用路线列 | 直接弧下标 MIP | 符合框架分解和可扩展定价 |
| 人工第一阶段 | 假定初始路线覆盖所有客户 | 在生成列期间可控地处理不可覆盖 |
| 精确定价完成标记 | 把达到列上限当作收敛 | 防止定价被截断时错误报告最优 |
| 类型化单位与校验器 | 全程使用裸 double | 保持坐标、时间和载荷兼容 |

## 8. 实现边界

源码依据是 `RouteCompilationContext`、`BranchNodeSolver`、`EspprcPricer`、`BranchAndPriceAlgorithm`
和 Demo5 `Application`。直接 MIP oracle 与 Rust 对照实现是比较工件，不改变本文记录的 Kotlin 生产模型。

