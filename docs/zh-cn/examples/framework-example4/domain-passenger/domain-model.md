# 旅客上下文模型

## 1. 概述

管理航班恢复调度系统中旅客的取消、舱位变更、航班变更和数量跟踪，将旅客相关约束和目标注册到列生成模型中。

### 1. 依赖上下文

1. **任务（task）**
2. **bunch_compilation**（航班串编译）

---

## 2. 概念 / 实体

### 1. 旅客（Passenger）

具有数量和多航段航班列表的旅客，每个航段分配一个舱位。

**$\text{id}_{p}$** ：旅客 $p$ 的唯一标识。

**$\text{amount}_{p}$** ：旅客 $p$ 的数量（团体旅客可大于 1）。

**$\text{flights}_{p}$** ：旅客 $p$ 的航段列表，每项为（航班任务，舱位）对。

**$\text{route}_{p}$** ：旅客 $p$ 的机场路线。

### 2. 航班旅客（FlightPassenger）

将旅客链接到特定航班的关联实体，具有可选前一航段。

**$\text{flight}_{fp}$** ：关联的航班任务。

**$\text{passenger}_{fp}$** ：关联的旅客。

**$\text{prev}_{fp}$** ：前一航段的航班旅客关联（可选）。

**$\text{cls}_{fp}$** ：旅客在该航班上的舱位。

**$\text{amount}_{fp}$** ：旅客数量。

### 3. 旅客取消（PassengerCancel）

跟踪列生成公式中的旅客取消决策变量。

**$\text{passengerCancel}_{fp}$** ：航班旅客 $fp$ 的取消变量。

### 4. 旅客变更（PassengerChange）

跟踪列生成公式中的旅客舱位变更和航班变更决策变量。

**$\text{passengerClassChange}_{fp,cls}$** ：航班旅客 $fp$ 变更为舱位 $cls$ 的变量。

**$\text{passengerFlightChange}_{fp,f',cls}$** ：航班旅客 $fp$ 变更为航班 $f'$ 和舱位 $cls$ 的变量。

### 5. 旅客数量（PassengerAmount）

计算每航班每舱位的旅客数量表达式，考虑取消和变更。

**$\text{passengerAmount}_{f,cls}$** ：航班 $f$ 在舱位 $cls$ 上的旅客数量表达式。

---

## 3. 变量

### 1. 决策变量

**$c_{fp}$** ：航班旅客 $fp$ 的取消数量，无量纲量，取值范围为 $[0, \text{amount}_{fp}]$ ，整数，表示被取消的旅客数量，$\forall fp \in FP$ 。

**$s_{fp,cls}$** ：航班旅客 $fp$ 变更为舱位 $cls$ 的数量，无量纲量，取值范围为 $[0, \text{amount}_{fp}]$ ，整数，$\forall fp \in FP, \forall cls \in CLS \setminus \{fp.cls\}$ 。

**$r_{fp,f',cls}$** ：航班旅客 $fp$ 变更为航班 $f'$ 和舱位 $cls$ 的数量，无量纲量，取值范围为 $[0, \text{amount}_{fp}]$ ，整数，$\forall fp \in FP, \forall f' \in \text{toFlights}_{fp.flight}, \forall cls \in CLS$ 。

### 2. 辅助变量

> 无额外辅助变量。

---

## 4. 谓词

### 1. 旅客状态

**isCancelled** ：航班旅客 $fp$ 被取消（$c_{fp} > 0$）。

**isClassChanged** ：航班旅客 $fp$ 发生舱位变更。

**isFlightChanged** ：航班旅客 $fp$ 发生航班变更。

**isTransfer** ：旅客 $p$ 是中转旅客（路线包含 2 个以上机场）。

---

## 5. 集合

### 1. 航班旅客

**$FP$** ：所有航班旅客关联全集。

**$FP_{f}$** ：航班 $f$ 上的航班旅客子集，$\forall f \in F$ 。

**$FP_{p}$** ：旅客 $p$ 的航班旅客子集，$\forall p \in P$ 。

### 2. 航班

**$F$** ：所有航班任务全集。

**$F_{fp}$** ：航班旅客 $fp$ 的替代航班子集（相同出发和到达机场）。

### 3. 舱位

**$CLS$** ：所有舱位全集（PassengerClass 枚举）。

---

## 6. 中间值

### 1. 旅客数量表达式

**描述**：航班 $f$ 在舱位 $cls$ 上的净旅客数量，考虑取消、舱位变更和航班变更。

$$
\text{passengerAmount}_{f,cls} = \sum_{fp \in FP_{f}} \left( \mathbb{1}[fp.cls = cls] \cdot \text{amount}_{fp} - c_{fp} - \mathbb{1}[fp.cls = cls] \cdot \sum_{cls' \in CLS} s_{fp,cls'} - \mathbb{1}[fp.cls = cls] \cdot \sum_{f', cls'} r_{fp,f',cls'} + \sum_{cls'} s_{fp,cls} + \sum_{fp' \in FP : f \in F_{fp'}} r_{fp',f,cls} \right)
$$

---

## 7. 断言

### 1. 旅客路线连续性

**描述**：旅客的航段列表中，连续航段的到达机场必须与下一航段的出发机场一致。

$$
\forall p \in P, \forall k \in [1, |\text{flights}_{p}|) \; (\text{flights}_{p}[k-1].arr = \text{flights}_{p}[k].dep)
$$

### 2. 航班类型一致性

**描述**：旅客的所有航段必须是航班类型。

$$
\forall p \in P, \forall (f, cls) \in \text{flights}_{p} \; (f.\text{isFlight})
$$

---

## 8. 约束

### 1. 旅客取消最小化

**[EN]**：Passenger Cancel Minimization

**描述**：最小化被取消的旅客总数（目标函数项）。

$$
\min \sum_{fp \in FP} w_{fp} \cdot c_{fp}
$$

### 2. 旅客舱位变更最小化

**[EN]**：Passenger Class Change Minimization

**描述**：最小化舱位变更的旅客总数（目标函数项）。

$$
\min \sum_{fp \in FP} \sum_{cls \in CLS \setminus \{fp.cls\}} w_{fp} \cdot s_{fp,cls}
$$

### 3. 旅客航班变更最小化

**[EN]**：Passenger Flight Change Minimization

**描述**：最小化航班变更的旅客总数（目标函数项）。

$$
\min \sum_{fp \in FP} \sum_{f' \in F_{fp}} \sum_{cls \in CLS} w_{fp} \cdot r_{fp,f',cls}
$$

### 4. 航班容量约束

**[EN]**：Passenger Flight Capacity Constraint

**描述**：每航班每舱位的旅客数量不得超过可用容量。

$$
s.t. \quad \text{passengerAmount}_{f,cls} \leq \text{capacity}_{f,cls}, \; \forall f \in F, \forall cls \in CLS
$$

### 5. 路线取消约束

**[EN]**：Passenger Route Cancel Constraint

**描述**：如果旅客的任一航段被取消，则整个路线的所有航段均被取消。

$$
s.t. \quad c_{fp} = c_{fp'}, \; \forall p \in P, \forall fp, fp' \in FP_{p}
$$

---

## 9. 目标函数（如适用）

**描述**：最小化旅客取消、舱位变更和航班变更的加权总和。

$$
\min \sum_{fp \in FP} \left( w^{cancel}_{fp} \cdot c_{fp} + \sum_{cls} w^{class}_{fp} \cdot s_{fp,cls} + \sum_{f', cls} w^{flight}_{fp} \cdot r_{fp,f',cls} \right)
$$

---

## 10. 算法引用

> 当前上下文无独立算法引用。

---

## 11. 通用语言

| 术语 | 符号 | 英文 | 定义 |
|------|------|------|------|
| 旅客 | $P$ | Passenger | 具有数量和多航段航班列表的旅客 |
| 航班旅客 | $FP$ | Flight Passenger | 旅客与特定航班的关联 |
| 取消 | $c$ | Cancel | 旅客被取消的数量 |
| 舱位变更 | $s$ | Class Change | 旅客舱位的变更数量 |
| 航班变更 | $r$ | Flight Change | 旅客航班的变更数量 |
| 路线 | route | Route | 旅客访问的机场序列 |

---

## 12. 设计决策

| 决策 | 备选方案 | 选择原因 | 日期 |
|------|----------|----------|------|
| 取消/变更分离建模 | 统一为恢复变量 | 不同业务语义和惩罚权重 | - |
| 路线取消联动 | 独立取消每航段 | 旅客体验：部分取消无意义 | - |

---

## 13. 变更记录

| 版本 | 变更 | 原因 |
|------|------|------|
| v1 | 初始实现 | 基础旅客域建模 |
