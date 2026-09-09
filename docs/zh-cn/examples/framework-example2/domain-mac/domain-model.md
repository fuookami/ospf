# 平均气动弦 领域模型

[English](../../../examples/framework-example2/domain-mac/domain-model)

[toc]

## 一、概述

从飞机和装载数据计算平均气动弦（MAC）百分比、纵向/横向扭矩、CLIM 和各飞行阶段的指数。

### 1. 依赖上下文

1. 飞机（aircraft）
2. 装载分配（stowage）

---

## 二、概念/实体

### 1. 扭矩

从装载、燃油、机身和公式数据计算各飞行阶段的纵向扭矩、横向扭矩、CLIM 和指数。

**$longitudinalTorque_{phase}$** ：各飞行阶段的纵向扭矩。
**$lateralTorque$** ：横向扭矩（仅宽体飞机）。
**$clim$** ：CLIM（Center of Gravity Index Moment）。
**$index_{phase}$** ：各飞行阶段的指数。

### 2. 平均气动弦

从扭矩指数和总重量计算 MAC 百分比。

**$mac$** ：MAC 百分比，线性中间符号。

### 3. 水平安定面

用于平衡计算的水平安定面位置和限制。

**$key$** ：水平安定面标识。
**$points$** ：水平安定面数据点。
**$limit$** ：水平安定面限制。

---

## 三、变量

本上下文复用装载分配上下文的决策变量，不定义独立决策变量。

---

## 四、中间值

### 1. 纵向扭矩

**描述**：各飞行阶段的纵向扭矩，由装载扭矩、燃油扭矩、机身力矩和救生筏力矩组成。

$$
longitudinalTorque_{phase} = \sum_{j \in J} loadLongitudinalTorque_j + fuelWeight_{phase} \cdot fuelArm_{phase} + dow \cdot balancedArm + liferaftWeight \cdot liferaftArm
$$

### 2. MAC 百分比

**描述**：平均气动弦百分比，由扭矩指数和总重量计算得出。

$$
mac = \frac{index_{TakeOff}}{tow} \cdot 100\%
$$

对每个飞行阶段，源码根据对应扭矩和飞机公式导出阶段指数。精确的系数及单位换算由 `Formula` 和飞机模型提供；
上式是契约层展开，不替代类型化系数。MAC 是供平衡和适航上下文消费的中间值，不是独立决策变量。

## 4.1 断言

每个引用的飞行阶段都必须有有效的燃油/机身公式，所有力臂、重量和指数使用飞机配置中兼容的单位。
没有所需公式的阶段不会被静默当作零扭矩阶段。

## 4.2 约束

本上下文只暴露派生符号。平衡和包络线约束由 `mac_optimization` 与 `airworthiness_security` 注册；定义 MAC 本身不会增加求解器行。

---

## 五、通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 扭矩 | Torque | Torque | 力与力臂的乘积 |
| 平均气动弦 | MAC | Mean Aerodynamic Chord | 机翼平均气动弦长 |
| 水平安定面 | HorizontalStabilizer | Horizontal Stabilizer | 尾翼水平安定面 |
| CLIM | CLIM | Center of Gravity Index Moment | 重心指数力矩 |

---

## 六、设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| MAC 计算公式 | 线性化 vs 非线性 | 线性化便于优化模型求解 | 2024 |

## 七、算法引用

没有独立算法文档；阶段公式由飞机 `Formula` 模型和 MAC 聚合实现。

## 八、演进记录

| 版本 | 变更 | 原因 |
| --- | --- | --- |
| 1.1 | 明确阶段范围、单位和下游约束归属 | 区分中间值与生效平衡限制 |
