# Framework 示例 3：csp1d 领域模型

[English](../../examples/framework-example3/domain-model.md)

本页明确 csp1d 的上下文边界，是[示例总览](../framework-example3)的补充，并遵循领域模型模板。
当前应用使用生成的切割计划列，不会手工构造一个固定的“所有模式”RMP。

## 1. 概述与依赖上下文

| 上下文 | 职责 | 依赖 |
| --- | --- | --- |
| material | 产品宽度、原料宽度范围和需求 | 无 |
| produce | 当前计划列、使用变量、需求贡献、原料/机器表达式 | material 与生成计划 |
| cutting-plan-generation | 初始计划和约简成本计划生成 | material 与 produce 影子价格 |
| length-assignment | 动态产品的可选长度限制 | material 与 produce |
| wasting-minimization | 废料/余料/原料成本和超产项 | produce 与配置 |
| application | 列生成编排、求解器、轨迹和输出 | 以上所有上下文 |

## 2. 概念、谓词与集合

令 `D` 为产品集合，`M` 为原料集合，`J_t` 为第 `t` 次迭代可用的列集合，`K` 为机器集合。
产品 `d` 的宽度 `width_d` 单位为 Meter，需求 `demand_d` 的单位为卷数。生成的切割计划 `j`
包含整数系数 `a_{dj}`（计划中产品 `d` 的件数）以及原料使用量。

常用谓词为 `dynamic(d)`（产品有可选长度规则）、`active(j)`（计划属于当前池）和
`feasible(j,m)`（计划满足原料、刀数和长度规则）。因此：

$$
D^{dynamic}=\{d\in D\mid dynamic(d)\},\qquad
J_t^{active}=\{j\mid j\text{ 已在第 }t\text{ 次迭代前插入}\}。
$$

示例输入使用宽度为 1000 Meter 的单一原料和四种产品 `(450,97)`、`(360,610)`、`(310,395)`、
`(140,211)`，每对分别表示 `(宽度, 需求)`。

## 3. 变量与中间值

受限主问题的计划使用变量为：

$$
x_j\in\mathbb Z_{\ge 0}\quad(j\in J_t^{active}),
$$

`x_j` 表示计划 `j` 的使用次数。产品产量及核心资源表达式为：

$$
q_d=\sum_{j\in J_t^{active}}a_{dj}x_j,
\qquad
u_m=\sum_{j\in J_t^{active}}usage_{mj}x_j,
\qquad
b_k=\sum_{j\in J_t^{active}}batch_{kj}x_j。
$$

启用产量松弛时，produce 上下文还使用非负的 `under_d` 和 `over_d`：

$$
q_d-over_d+under_d=demand_d\quad(d\in D)。
$$

长度分配和废料最小化上下文只在相应谓词/配置存在时贡献表达式。

## 4. 断言与约束

没有松弛的普通路径要求满足全部需求：

$$
s.t.\quad q_d\ge demand_d\qquad(d\in D)。
$$

原料、机器和已配置的长度限制由对应管线添加：

$$
s.t.\quad u_m\le capacity_m\quad(m\in M),
\qquad
s.t.\quad b_k\le capacity_k\quad(k\in K)，
$$

精确系数由生成计划及原料/机器数据提供。动态产品的长度断言仅对
`d∈D^{dynamic}` 生效。`addColumns` 后 `J_t` 会改变，因此量词是迭代局部的，不能替换成固定的全局模式集合。

## 5. 目标函数

基础生效目标最小化计划使用次数：

$$
\min\sum_{j\in J_t^{active}}x_j。
$$

已配置的 wasting-minimization 可以增加废料、余料/原料成本或超产惩罚。本示例调用没有传入这些可选配置覆盖，
因此不能声称这些项已生效。

## 6. 列生成算法边界

第 `t` 次迭代先在 `J_t^{active}` 上求解 RMP。需求/资源约束产生的影子价格传给定价生成器，后者搜索可行计划并返回
负约简成本列。应用插入去重后的列得到 `J_{t+1}^{active}`，直到定价完成或达到配置限制。`RMP` 与 `pricing`
是算法角色；`Main.kt` 中没有第二套固定变量族。

## 7. 通用语言与设计决策

| 术语 | 符号 | 含义 |
| --- | --- | --- |
| product | `d` | 请求的切割宽度和需求 |
| plan/column | `j` | 一个可行切割模式 |
| yield | `q_d` | 已选计划生产的产品件数 |
| usage | `x_j` | 计划使用次数（整数） |
| RMP | `J_t` | 当前受限列池 |
| pricing | — | 搜索改进型可行计划 |

| 决策 | 备选 | 原因 |
| --- | --- | --- |
| 增量生成列 | 固定全模式 RMP | 控制主问题规模并符合 `addColumns` |
| 需求按卷数表示 | 把需求当作长度 | 源码 `legacyRoll` 使用数量语义 |
| 可选惩罚由配置控制 | 始终添加惩罚 | 避免把未激活目标写成当前模型 |

## 8. 源码与演进边界

实现证据是父页面链接的 csp1d `Csp1dProblem`、produce 聚合、`Csp1dColumnGeneration` 和 `Main.kt`。
本页记录模型契约；具体配置下哪些可选约束生效，以源码管线和测试为准。
