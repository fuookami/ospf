# 装载效能上下文模型


## 1. 概述

管理装载效能约束以提升操作效率——包括拖车装载、顺序装载、转运邻接和始发/目的地分组。

### 1. 依赖上下文

1. 飞机上下文（`aircraft`）
2. 装载分配上下文（`stowage`）

---

## 2. 概念 / 实体

### 1. 建议装载

预分配模式下各位置的建议装载数量和重量。

**$adviceAmount_{j}$** ：位置 $j$ 的建议装载数量。

**$adviceWeight_{j}$** ：位置 $j$ 的建议装载重量。

### 2. 转运邻接装载

同源/同目的地邻接约束以提升转运效率。

**$adjacentPositions$** ：邻接位置对列表。

**$sources$** ：始发站列表。

**$destinations$** ：目的站列表。

### 3. 顺序装载

基于位置排序的顺序装载约束。

**$orderedPositions$** ：排序后的位置对列表。

### 4. 拖车装载

满载模式下的拖车变更和绕行约束。

**$trailers$** ：拖车列表。

---

## 3. 变量

### 1. 决策变量

本上下文复用装载分配上下文的决策变量，不定义独立决策变量。

### 2. 辅助变量

本上下文不定义独立辅助变量。

---

## 4. 谓词

本上下文不定义独立谓词。

---

## 5. 集合

本上下文不拥有独立集合。使用 $I$ 表示货物项，$J$ 表示装载位置，$A\subseteq J\times J$ 表示相邻位置对，$R$ 表示拖车装载模型提供的有序拖车对。

---

## 6. 中间值

源码拖车装载模型提供中间符号 `trailerChange[p_1,p_2]` 和 `trailerCircling[p_1,p_2]`。对于有序拖车对 $(r,s)\in R$ 和相邻位置对 $(j,k)\in A$，将 `trailerChange[p_1,p_2]` 记为 $\Delta_{rs,jk}$。其动态 `IfFunction` 表达式为：

$$
\Delta_{rs,jk}=\operatorname{If}\!\left(L^{s}_j+L^{r}_k-2+\tau\right)
$$

其中 $L^{s}_j$ 表示位置 $j$ 上来自拖车 $s$ 的装载量，$L^{r}_k$ 表示位置 $k$ 上来自拖车 $r$ 的装载量，$\tau$ 表示源码 `IfFunction` 使用的非零比较偏移量。本页将 `trailerCircling[p_1,p_2]` 记录为相关的模型层中间值，但不把它计入下面的拖车变更目标。

---

## 7. 断言

本上下文不定义独立断言。

---

## 8. 约束

下面两项邻接规则属于业务语义。其具体决策变量注册由源码管线负责；本页不把任一规则重述为通用的 `s.t.` 公式。

### 1. 同源邻接限制

**[英]**：Same Source Adjacent Limit

**描述**：同源货物应装载在相邻位置，以减少转运处理。`SameSourceAdjacentLimit` 是具体表达式的源码管线边界；本页只记录业务规则，不断言通用的 `s.t.` 行。

### 2. 同目的地邻接限制

**[英]**：Same Destination Adjacent Limit

**描述**：同目的地货物应装载在相邻位置，以减少转运处理。`SameDestinationAdjacentLimit` 是具体表达式的源码管线边界；本页只记录业务规则，不断言通用的 `s.t.` 行。

### 3. 拖车变更限制

**[英]**：Trailer Change Limit

**描述**：源码限制在有序拖车对和相邻位置对上，对已注册的 `trailerChange[p_1,p_2]` 中间值求加权和的最小值。它是目标输入而不是 `s.t.` 约束；精确表达式见第 9 节。

---

## 9. 目标函数（如适用）

源码层的拖车变更目标为：

$$
\min \sum_{(r,s)\in R}\sum_{(j,k)\in A} c_{rs,jk}\,\Delta_{rs,jk}
$$

其中 $c_{rs,jk}$ 是源码系数调用 `coefficient((k,r),(j,s))` 的缩写，保留其位置/拖车参数顺序。`loadingOperationalCost` 仅保留为业务层的聚合名称，用于表示管线可能选择的其他操作项。Demo2 Kotlin 源码没有以此名称暴露单一线性表达式，因此本页不为它断言求解器公式或已注册的目标。

---

## 10. 算法引用

本上下文不定义独立算法引用。

---

## 11. 通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 建议装载 | `AdviceLoading` | Advice Loading | 各位置的建议装载量 |
| 转运邻接 | `TransferAdjacent` | Transfer Adjacent | 转运货物的邻接要求 |
| 顺序装载 | `SequentialLoading` | Sequential Loading | 基于顺序的装载约束 |
| 拖车装载 | `TrailerLoading` | Trailer Loading | 拖车相关的装载约束 |
| 拖车变更 | `trailerChange[p_1,p_2]` | Trailer Change | 跨拖车变更的已注册中间值 |
| 装载操作成本 | `loadingOperationalCost` | Loading Operational Cost | 业务层聚合名称；不在此断言单一 Demo2 表达式 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 |
|------|----------|----------|
| 邻接定义 | 装载顺序与物理位置 | 装载顺序更符合操作实际 |

---

## 13. 变更记录

本页未记录上下文专属的变更条目。
