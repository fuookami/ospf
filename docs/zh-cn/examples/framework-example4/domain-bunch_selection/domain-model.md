# Bunch Selection 上下文领域模型

[English](../../../examples/framework-example4/domain-bunch_selection/domain-model)

## 1. 概述

bunch selection 上下文拥有分支定价策略，并协调 bunch generation 与 bunch compilation。当前 Kotlin 上下文是算法编排器，
不声明独立变量族。

### 1. 依赖上下文

bunch generation、bunch compilation、task、rule、passenger 和 aircraft。

## 2. 概念/实体

### 1. 选择策略

**$P$** 包含约简成本容差、列上限、执行器最少列数和时间上限；**$gen$** 与 **$compile$** 是生成/编译端口。

### 2. 影子价格映射

**$\Pi$** 将主模型符号映射为定价使用的对偶值。

### 3. bunch 解

**$S$** 包含选中的 bunch 和最终编译结果。

## 3. 变量

`BunchSelectionContext` 不注册求解器变量。分支决策和节点界属于通用分支定价算法状态。

## 4. 谓词

**improving(b)**：生成 bunch 的约简成本可接受；**fractional(n)**：节点解有分数选择；**integral(n)**：选中列通过整数验收；
**compatible(b,n)**：bunch 遵守节点决策。

## 5. 集合

**$N$**：分支节点；**$B_n$**：与节点 `n` 兼容的列；**$D_n$**：分支 require/forbid 决策；**$\Pi_n$**：节点对偶映射。

## 6. 中间值

节点 `n` 的队列界和定价结果为：

$$
lowerBound_n=RMP(n),
\qquad
pricing_n=\{b\in B_n\mid rc_n(b)<-\epsilon\}。
$$

## 7. 断言

子节点继承不可变分支决策并只保留兼容列：

$$
\forall b\in B_n:\quad compatible(b,n)=true。
$$

整数候选只有在编译和领域可行性检查成功后才可接受。

## 8. 约束

本上下文不注册独立求解器约束。策略对时间、节点、列数和约简成本容差施加算法限制；这些是终止条件，不是数学行。

## 9. 目标函数

上下文把主模型目标委托给 bunch compilation，不引入第二个目标。

## 10. 算法引用

| 算法 | 源码角色 | 用途 |
| --- | --- | --- |
| BranchAndPriceAlgorithm | selection service | 处理节点、对分数解分支并协调定价 |

## 11. 通用语言

| 术语 | 符号 | 定义 |
| --- | --- | --- |
| 分支节点 | `n` | 带继承决策的 RMP |
| 分支决策 | `D_n` | require/forbid 分配或链接选择 |
| incumbent | `S` | 最优已接受整数解 |
| 下界 | `lowerBound_n` | 节点 RMP 界 |

## 12. 设计决策

| 决策 | 备选 | 原因 |
| --- | --- | --- |
| 在编排层维护分支 | 把分支状态编码成领域变量 | 通用算法可复用生成/编译端口 |

## 13. 演进记录

| 版本 | 变更 | 原因 |
| --- | --- | --- |
| 1.0 | 将选择记录为编排上下文 | 避免虚构第二套优化模型 |

