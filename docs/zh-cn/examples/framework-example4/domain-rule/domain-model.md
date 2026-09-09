# Rule 上下文领域模型

[English](../../../examples/framework-example4/domain-rule/domain-model)

## 1. 概述

rule 上下文定义链接、限制、流控制和可行性服务，用于判断恢复后的任务序列是否合法。
它向 bunch generation 提供谓词和成本，不是独立主问题模型。

### 1. 依赖上下文

task、飞机以及基础设施的时间/成本类型。

## 2. 概念/实体

### 1. 链接

**$prev_l$**、**$succ_l$**：前置和后继任务；**$splitCost_l$**：链接拆分成本；**$type_l$**：连接、经停或忽略连接时间的链接。

### 2. 限制

**$category_r$**：限制类别；**$weight_r$**：违反权重；**$cost_r$**：可选违反成本；**$condition_r$**：任务/机场/飞机条件。

### 3. 流控制

**$airport_f$**、**$scene_f$**、**$time_f$** 和 **$capacity_f$** 描述到达/出发流时间窗及容量。

## 3. 变量

`RuleContext` 不注册决策变量或辅助变量。

## 4. 谓词

**linkType(l)** 对链接语义分类；**violates(t,r)** 表示任务 `t` 违反限制 `r`；**flowClosed(f)** 表示流容量为零；
**feasible(t,t')** 表示连接时间、规则和飞机检查均通过。

## 5. 集合

**$L$**：链接；**$L^{conn}$**、**$L^{stop}$** 和 **$L^{ignore}$** 是链接子集；**$R$**：限制；
**$F$**：流控制时间窗；**$T^{feas}$**：通过全部启用规则的任务转移。

## 6. 中间值

连接服务导出转移时间和违反总成本：

$$
\Delta(t,t')=start_{t'}-end_t,
\qquad
cost(t,t')=\sum_{r\in R}weight_r\,\mathbf1[violates(t,t',r)]。
$$

## 7. 断言

每个链接都有两个任务端点，连接链接只连接机场/时间数据已定义的任务。每个流时间窗的容量非负且时间范围非空。

## 8. 约束

本上下文不注册独立求解器行。可行性是服务谓词：

$$
s.t.\quad feasible(t,t')\Longleftrightarrow
\Delta(t,t')\ge requiredConnectionTime(t,t')\wedge
\neg violates(t,t',r)\ \forall r\in R^{hard}。
$$

## 9. 目标函数

无。转移和限制成本作为 bunch 成本计算器的输入。

## 10. 算法引用

没有独立算法文档；`FeasibilityJudger`、连接时间和成本计算器实现这些谓词。

## 11. 通用语言

| 术语 | 符号 | 定义 |
| --- | --- | --- |
| 链接 | `l` | 连续任务之间的关系 |
| 限制 | `r` | 限制任务转移的规则 |
| 流控制 | `f` | 机场/时间容量窗口 |
| 可行性 | `feasible` | 全部启用规则通过 |

## 12. 设计决策

| 决策 | 备选 | 原因 |
| --- | --- | --- |
| 在插入列前评估规则 | 先把非法 bunch 加入主模型再过滤 | 保持生成列可行 |

## 13. 演进记录

| 版本 | 变更 | 原因 |
| --- | --- | --- |
| 1.0 | 将规则谓词记录为服务边界 | 将可行性与主模型变量分离 |

