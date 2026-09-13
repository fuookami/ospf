# 软安全上下文模型


## 1. 概述

管理软安全约束，包括空载分离、主甲板舱门空载偏好和压舱物重量建议——这些约束可提升安全性，但必要时可放松。

### 1. 依赖上下文

1. 飞机上下文（`aircraft`）
2. 装载分配上下文（`stowage`）

---

## 2. 概念 / 实体

### 1. 空载分离

确保空载位置分散而非集中，以保障结构安全。

**$positions$** ：位置列表。

**$load$** ：装载数据。

---

## 3. 变量

### 1. 决策变量

本上下文复用装载分配上下文的决策变量，不定义独立决策变量。

### 2. 辅助变量

本上下文不定义独立辅助变量。

---

## 4. 谓词

业务语义关系 `distributedEmptyPositions(J)` 表示空载分离偏好。在 Demo2 管线中，该偏好由相邻位置对上的三个源码中间表达式和三个独立软目标输入实现；该关系本身不是已注册的求解器约束。

---

## 5. 集合

本上下文不拥有独立集合。使用 $I$ 表示货物项，$J$ 表示装载位置，$A\subseteq J\times J$ 表示相邻位置对，$J^{\mathrm{emptyHated}}\subseteq J$ 表示标记为 `EmptyHated` 的位置，$J^{\mathrm{beside}}\subseteq J$ 表示主甲板舱门旁的位置。

---

## 6. 中间值

源码 `DivideEmptyLoading` 模型为每个 $(j,k)\in A$ 注册三个按索引对齐的中间表达式：

1. `emptyBetweenCargo[p]`，记为 $e^{\mathrm{bc}}_{jk}$，表示第一位置的非空货物与第二位置的总货物之间的空载模式。
2. `emptyCargoBetweenCargo[p]`，记为 $e^{\mathrm{cb}}_{jk}$，表示第一位置的空货物与第二位置的非空货物之间的空载模式。
3. `emptyBetweenEmptyCargo[p]`，记为 $e^{\mathrm{bb}}_{jk}$，表示第一位置的空货物与第二位置的总货物之间的空载模式。

对于装载量由模型动态决定的位置，源码 `IfFunction` 分支可表示为：

$$
\begin{aligned}
e^{\mathrm{bc}}_{jk} &= \operatorname{If}\!\left(L^{\mathrm{nonempty}}_j-\left(L^{\mathrm{all}}_k+1\right)+\tau\right),\\
e^{\mathrm{cb}}_{jk} &= \operatorname{If}\!\left(L^{\mathrm{empty}}_j+L^{\mathrm{nonempty}}_k-2+\tau\right),\\
e^{\mathrm{bb}}_{jk} &= \operatorname{If}\!\left(L^{\mathrm{empty}}_j-\left(L^{\mathrm{all}}_k+1\right)+\tau\right).
\end{aligned}
$$

其中 $\tau$ 表示源码 `IfFunction` 使用的非零比较偏移量。源码对固定范围还提供将表达式化为零、装载量或其他常量的分支；这些是模型表达式，不是额外的求解器约束。

---

## 7. 断言

本上下文不定义独立断言。

---

## 8. 约束

本节列出本上下文的软性限制。在 Demo2 管线中，下面四项都会贡献目标项，而不是 `s.t.` 行。公式使用源码层符号，不在此页断言新的聚合目标。

### 1. 空载厌恶限制

**[英]**：Empty Hated Limit

**描述**：管线为标记为 `EmptyHated` 的位置注册软目标项；包含位置系数和满载表达式的源码对齐形式见第 9 节。

### 2. 主甲板舱门空载限制

**[英]**：Main Deck Door Empty Limit

**描述**：管线最小化主甲板舱门旁位置的装载分配（B757/B767），而不是施加 `empty_j = 1` 行；源码对齐的精确表达式见第 9 节。

### 3. 空载分离限制

**[英]**：Divide Empty Loading Limit

**描述**：管线在相邻位置对上注册三个独立的软目标输入。`distributedEmptyPositions(J)` 只是这一偏好的业务语义名称，并未注册为 `s.t.` 约束；三个源码对齐表达式见第 9 节。

### 4. 建议压舱物重量限制

**[英]**：Advice Ballast Weight Limit

**描述**：仅当存在建议值时，管线才加入单侧阈值松弛目标；负偏差语义及源码对齐的精确表达式见第 9 节，此处没有断言硬性的下界 `s.t.` 行。

---

## 9. 目标函数（如适用）

本上下文贡献以下独立软目标输入。目标项的启用方式及更高层聚合由所选模式管线决定；本页不虚构单一的通用惩罚总和表达式，且这些条目都不是硬性的 `s.t.` 行。

### 1. 空载厌恶

$$
\min \sum_{j \in J^{\mathrm{emptyHated}}} c_j \left(1 - load^{\mathrm{full}}_j\right)
$$

其中 $c_j$ 是位置系数，$load^{\mathrm{full}}_j$ 是源码中的满载表达式。

### 2. 主甲板舱门空载

$$
\min \sum_{i \in I}\sum_{j \in J^{\mathrm{beside}}} c_i\,stowage_{ij}
$$

其中 $J^{\mathrm{beside}}$ 表示门位置关系为 `Beside` 的位置集合。

### 3. 空载分离

$$
\begin{aligned}
\min\;&\sum_{(j,k)\in A} c^{\mathrm{bc}}_{jk}e^{\mathrm{bc}}_{jk},\\
\min\;&\sum_{(j,k)\in A} c^{\mathrm{cb}}_{jk}e^{\mathrm{cb}}_{jk},\\
\min\;&\sum_{(j,k)\in A} c^{\mathrm{bb}}_{jk}e^{\mathrm{bb}}_{jk}.
\end{aligned}
$$

这是三个独立的源码目标输入，系数族和中间值族见第 6 节。

### 4. 建议压舱物重量

$$
\min c_{\mathrm{ballast}}\,s^{-}_{\mathrm{ballast}},
\qquad s^{-}_{\mathrm{ballast}}\geq 0
$$

仅当存在建议值时才加入该项；$s^{-}_{\mathrm{ballast}}$ 是低于建议阈值时的单侧松弛。

---

## 10. 算法引用

本上下文不定义独立算法引用。

---

## 11. 通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 空载分离 | `DivideEmptyLoading` | Divide Empty Loading | 空载位置的分散分布 |
| 空载厌恶 | `EmptyHated` | Empty Hated | 对空载位置的软惩罚 |
| 主甲板舱门空载 | `MainDeckDoorEmpty` | Main Deck Door Empty | 主甲板舱门旁装载的软惩罚 |
| 压舱物建议 | `AdviceBallastWeight` | Advice Ballast Weight | 压舱物重量的建议值 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 |
|------|----------|----------|
| 约束类型 | 硬约束 vs 软约束 | 软约束允许在必要时放松 |

---

## 13. 变更记录

本页未记录上下文专属的变更条目。
