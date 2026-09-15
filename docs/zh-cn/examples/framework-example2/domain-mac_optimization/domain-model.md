# MAC 优化上下文模型


## 1. 概述

管理 MAC 优化，包括纵向平衡（MAC 范围约束）和横向平衡约束，用于飞机重量分布。

### 1. 依赖上下文

1. 飞机上下文（`aircraft`）
2. 装载分配上下文（`stowage`）
3. 平均气动弦上下文（`mac`）

---

## 2. 概念 / 实体

### 1. MAC 范围

基于总重量定义允许的 MAC 百分比范围。

**$minMAC_{weight}$** ：给定总重量下的最小 MAC 百分比。

**$maxMAC_{weight}$** ：给定总重量下的最大 MAC 百分比。

### 2. 纵向平衡

纵向平衡约束，确保各飞行阶段 MAC 在允许范围内。

**$macRange$** ：MAC 范围。

**$torque$** ：扭矩数据。

### 3. 横向平衡

宽体飞机的横向平衡约束，确保对称装载。

**$torque$** ：横向扭矩数据。

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

本上下文不定义独立集合；其平衡限制使用位置和飞行阶段集合。

---

## 6. 中间值

本上下文不定义独立中间值。

---

## 7. 断言

本上下文不定义独立断言。

---

## 8. 约束

### 1. 纵向平衡限制

**[英]**：Longitudinal Balance Limit

**描述**：MAC 百分比必须在各飞行阶段的允许范围内。

$$
s.t. \quad minMAC_{weight} \leq mac \leq maxMAC_{weight}, \; \forall phase \in FlightPhases
$$

### 2. 横向平衡限制

**[英]**：Lateral Balance Limit

**描述**：横向扭矩必须在允许范围内（仅宽体飞机）。

$$
s.t. \quad |lateralTorque| \leq maxLateralTorque
$$

### 3. 水平安定面限制

**[英]**：Horizontal Stabilizer Limit

**描述**：水平安定面位置必须与 MAC 匹配。

$$
s.t. \quad stabilizerPosition = f(mac), \; \forall phase \in FlightPhases
$$

---

## 9. 目标函数（如适用）

最小化 MAC 偏离目标范围的程度。

$$
\min |mac - macTarget|
$$

---

## 10. 算法引用

本上下文不定义独立算法引用。

---

## 11. 通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| MAC 范围 | `MACRange` | MAC Range | 允许的 MAC 百分比范围 |
| 纵向平衡 | `LongitudinalBalance` | Longitudinal Balance | 前后方向的重量平衡 |
| 横向平衡 | `LateralBalance` | Lateral Balance | 左右方向的重量平衡 |
| 水平安定面 | `HorizontalStabilizer` | Horizontal Stabilizer | 尾翼水平安定面 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 |
|------|----------|----------|
| MAC 范围建模 | 线性与分段线性 | 分段线性更精确 |

---

## 13. 变更记录

本页未记录上下文专属的变更条目。
