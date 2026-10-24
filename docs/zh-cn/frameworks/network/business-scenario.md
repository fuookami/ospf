# 共享资源、缺货与联合模型

完成[第一次网络流优化](./first-flow)后，先加入一个网络外的容量，再决定是否允许缺货。弧容量充分、图上连通，不代表整个业务有可行解。

## 1. 一个共享装卸资源让问题不可行

以同一种货物的归一化单位计量：供给点 source 有 2 单位，需求点 sink 需要 2 单位，只有弧 source→sink，弧容量为 3。增加共享资源 dock，容量为 1，每运输 1 单位占用 dock 的 1 单位容量。

弧本身能送 2 单位，但 dock 只能支持 1 单位，所以硬需求不可行。增加另一条弧也不一定有用：若它仍消耗同一个 dock，资源总上界没有变化。恢复可行性需要增加资源容量、改变实际路径资源消耗，或经过授权放松需求。

从第一课的流问题开始，保持供需不变，只增加共享资源行；重新求解后，逐项核对实际流量与共享资源消耗。

## 2. 多商品必须先转换资源占用单位

### 构造供需和弧，再加入 dock

本课代码把同一种货物的一个业务单位归一化为 `NoneUnit`，源点为 `source`、需求点为 `sink`。先用 `makeProblem(2.0)` / `make_problem(2.0)` 跑通，再把 dock 上界改为 1.0；弧容量一直保持 3，供需一直保持 ±2。

::: code-group

<<< ../../../frameworks/_examples/network-business.kt#input [Kotlin]

<<< ../../../frameworks/_examples/network-business.rs#input [Rust]

:::

| 对象 | Kotlin | Rust |
| --- | --- | --- |
| 共享资源 | `FlowCapacity` | `FlowResource` |
| 商品-弧占用 | `FlowKey` 对应的 `usage` | `consumptions` 中的商品/弧键 |
| 商品可用弧 | `allowedArcs` | `allowed_arcs` |
| 弹性供需 | `FlowSlack` | `FlowBalanceSlack` |

::: tip 单位约束
商品以 kg、m、件计量时，需通过“资源单位/商品单位”的系数换算后求和。不能把未经过单位换算的整数步数直接当物理占用；也不能将不同 ID 的平行服务合并。
:::

## 3. 允许缺货必须同时有上界和费用

单独构造一个允许未使用供应和缺货的场景，数值使用同一计量单位。未使用供应表示供给未被运走，缺货表示需求未被满足；两者都保留原始供需，并在模型中增加带上限和罚费的偏差变量：

| 条件 | 数值 |
| --- | --- |
| source 供应 / sink 需求 | 3 / 4 |
| 弧最大流量 / 单位运输费 | 2 / 1 |
| source 允许未使用供应 / 单位罚费 | 最多 1 / 5 |
| sink 允许未满足需求 / 单位罚费 | 最多 2 / 7 |

流量为 2，未使用供应为 1，缺货为 2，总费用 `2×1 + 1×5 + 2×7 = 21`。缺货不等于把订单从 4 改成 2，原需求仍需报告；放松的上界也不是无限。

无论使用哪种语言，验收时都应分别展示实际运输量、未使用供应、缺货与各项费用。总费用相同，不代表这些业务结果也相同。


### 设置未使用供应和缺货的上限

`makeSlackProblem()` / `make_slack_problem()` 构造表中独立算例，不保留 dock 限制。用第一课相同服务求解这个新问题，再读取未使用供应、缺货和费用分解。这些允许供需偏差的变量也称为“松弛变量”。

::: code-group

<<< ../../../frameworks/_examples/network-business.kt#slack [Kotlin]

<<< ../../../frameworks/_examples/network-business.rs#slack [Rust]

:::

### 调用独立服务 {#entry-budget}

相同入口即可求解新增共享容量后的问题；纯流问题没有列数或定价轮数参数。求解预算由实际线性/整数后端配置，报告与业务方案一起保存。

::: code-group

<<< ../../../frameworks/_examples/network-business.kt#entry [Kotlin]

<<< ../../../frameworks/_examples/network-business.rs#entry [Rust]

:::

## 4. 如何把网络决策加入联合模型？

### 最小业务规则：增加一个共享 dock 容量行

本类规则已有领域对象，无需自造策略接口。Kotlin 将 `FlowCapacity` 放入 `FlowProblem.resources`；Rust 用 `FlowResource` 经 `with_resources` 注入。系数为“每一归一化商品单位占用 1 资源单位”，多个商品/服务共用时继续向同一行加入键。

::: code-group

<<< ../../../frameworks/_examples/network-business.kt#rule [Kotlin]

<<< ../../../frameworks/_examples/network-business.rs#rule [Rust]

:::

独立入口为 Kotlin `NetworkFlowApplicationService`、Rust `FlowApplicationService`。如果流量还要与建线、排程或共享预算一起决策，应把 `FlowContext` 注册到共同模型，由上层统一求解。

共享 ID 还不够：相互影响的决策必须通过实际变量、约束或表达式连接。先单独求最便宜运输、再把结果交给排程器，通常只是分解式业务流程，不能声称联合最优。若采用分解算法，需要明确反馈、可行性修复和终止证据。

复用模型时还需维护上下文对象与变量的对应关系，不能把上次运行的结果向量按旧下标填入新模型。每个请求应独立保存建模和求解状态；模型注册失败后应重新构造，不继续使用未完成的模型。

## 5. 验收与练习

1. 将 dock 上界从 1 增到 2，确认硬需求恢复可行；再增到 3，检查是否仍只运输需求 2。
2. 保持原始硬需求，显式加入有界缺货，再把缺货上界收紧到不足以弥补容量缺口，确认重新不可行。
3. 逐商品核对守恒、弧上下界、共享资源和松弛；分别重算运输费、未使用供应罚费及缺货罚费。
4. 在联合模型中修改共享预算，检查网络与另一个领域的决定共同变化，而不是两个互不相关的输出。

缺货方案能否交付由业务决定，不能只看框架成功返回。按[接入指南](../integration)同时保留缺口和真实求解状态；跨地点的时间与人员安排继续到 [Gantt](../gantt/business-scenario)。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。


## 完整应用源码

片段来自同一个源码文件；保留导入声明和辅助函数后，从应用调用 `runBusiness` / `run_business`。构造函数也可独立用于输入校验。后端依赖沿用第一课；这些入口不创建隐式求解后端。

:::: details 展开完整 Kotlin / Rust 源码

::: code-group

<<< ../../../frameworks/_examples/network-business.kt [Kotlin]

<<< ../../../frameworks/_examples/network-business.rs [Rust]

:::

::::
