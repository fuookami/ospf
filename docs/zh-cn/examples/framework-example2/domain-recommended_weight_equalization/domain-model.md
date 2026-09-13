# 建议载重量均衡上下文模型


## 1. 概述

管理建议载重量均衡——确保货物重量按优先级预约在各位置间均匀分布。

### 1. 依赖上下文

1. 飞机上下文（`aircraft`）
2. 装载分配上下文（`stowage`）

---

## 2. 概念 / 实体

### 1. 优先级预约

基于优先级的货物-位置预约及重量均衡。

**$appointment$** ：货物到位置的预约映射。

**$priority$** ：货物优先级。

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

本上下文不定义独立集合；其限制使用装载分配上下文的货物项和位置集合。

---

## 6. 中间值

本上下文不定义独立中间值。

---

## 7. 断言

本上下文不定义独立断言。

---

## 8. 约束

### 1. 货物顺序限制

**[英]**：Item Order Limit

**描述**：货物必须按优先级顺序装载。

$$
s.t. \quad priority_i < priority_j \rightarrow order_i \leq order_j, \; \forall i, j \in Items
$$

### 2. 优先级预约限制

**[英]**：Priority Appointment Limit

**描述**：必须遵守优先级预约。

$$
s.t. \quad x_{ij} = 1, \; \forall (i, j) \in PriorityAppointment
$$

### 3. 建议载重量均衡限制

**[英]**：Recommended Weight Equalization Limit

**描述**：装载重量应在各位置间均衡分布。

$$
s.t. \quad |loadWeight_j - avgWeight| \leq tolerance, \; \forall j \in J
$$

---

## 9. 目标函数（如适用）

最小化重量偏离推荐值的程度。

$$
\min \sum_{j \in J} |loadWeight_j - recommendedWeight_j|
$$

---

## 10. 算法引用

本上下文不定义独立算法引用。

---

## 11. 通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 优先级预约 | `PriorityAppointment` | Priority Appointment | 基于优先级的货物-位置预约 |
| 重量均衡 | `WeightEqualization` | Weight Equalization | 货物重量的均匀分布 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 |
|------|----------|----------|
| 均衡策略 | 绝对均衡与相对均衡 | 相对均衡更灵活 |

---

## 13. 变更记录

本页未记录上下文专属的变更条目。
