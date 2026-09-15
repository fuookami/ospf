# Express effectiveness context model


## 1. Overview

Manages express effectiveness constraints that optimize item priority ordering — ensuring high-priority cargo is loaded preferentially.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Absolute Order

Defines absolute priority ordering for predistribution mode.

**$order_{i}$** : Absolute priority order of item $i$.

### 2. Relative Order

Defines relative priority ordering for full-load mode.

**$relativeOrder_{ij}$** : Priority order of item $i$ relative to item $j$.

### 3. Must-Ship Items

Items that must be shipped regardless of priority.

**$mustShipIndices$** : Index list of must-ship items.

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

### 1. Must-Ship Limit

**[CN]**: 必须发运限制

**Description**: Must-ship items must be loaded.

$$
s.t. \quad \sum_{j \in J} x_{ij} = 1, \; \forall i \in MustShip
$$

### 2. Item Priority Limit

**[CN]**: 货物优先级限制

**Description**: Higher priority items should be loaded before lower priority items.

$$
s.t. \quad priority_i < priority_j \rightarrow loaded_i \geq loaded_j, \; \forall i, j \in Items
$$

---

## 9. Objective Function (if applicable)

Minimize priority violation cost.

$$
\min \sum_{i \in I} priorityViolationCost_i
$$

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Absolute Order | `AbsoluteOrder` | Priority ordering for predistribution mode |
| Relative Order | `RelativeOrder` | Priority ordering for full-load mode |
| Must-Ship Items | `MustShip` | Items that must be shipped |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Ordering mode | Absolute vs Relative | Different stowage modes use different ordering strategies |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
