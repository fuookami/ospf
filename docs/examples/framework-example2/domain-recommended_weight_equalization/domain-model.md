# Recommended weight equalization context model


## 1. Overview

Manages recommended weight equalization — ensuring cargo weight is distributed evenly across positions according to priority appointments.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Priority Appointment

Priority-based appointment of items to positions with weight equalization.

**$appointment$** : Item-to-position appointment mapping.

**$priority$** : Item priority.

---

## 3. Variables

### 1. Decision Variables

This context reuses decision variables from the stowage context and does not define independent decision variables.

### 2. Auxiliary Variables

This context does not define independent auxiliary variables.

---

## 4. Predicates

This context does not define independent predicates.

---

## 5. Sets

This context does not define independent sets; its limits range over items and positions from the stowage context.

---

## 6. Intermediate Values

This context does not define independent intermediate values.

---

## 7. Assertions

This context does not define independent assertions.

---

## 8. Constraints

### 1. Item Order Limit

**[CN]**: 货物顺序限制

**Description**: Items must be loaded in priority order.

$$
s.t. \quad priority_i < priority_j \rightarrow order_i \leq order_j, \; \forall i, j \in Items
$$

### 2. Priority Appointment Limit

**[CN]**: 优先级预约限制

**Description**: Priority appointments must be respected.

$$
s.t. \quad x_{ij} = 1, \; \forall (i, j) \in PriorityAppointment
$$

### 3. Recommended Weight Equalization Limit

**[CN]**: 推荐重量均衡限制

**Description**: Load weight should equalize across positions.

$$
s.t. \quad |loadWeight_j - avgWeight| \leq tolerance, \; \forall j \in J
$$

---

## 9. Objective Function (if applicable)

Minimize weight deviation from recommended values.

$$
\min \sum_{j \in J} |loadWeight_j - recommendedWeight_j|
$$

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Priority Appointment | `PriorityAppointment` | Priority-based item-to-position appointment |
| Weight Equalization | `WeightEqualization` | Even distribution of cargo weight |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Equalization strategy | Absolute vs Relative | Relative equalization is more flexible |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
