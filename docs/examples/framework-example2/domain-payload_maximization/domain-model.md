# Payload maximization context model


## 1. Overview

Maximizes the total payload (cargo weight) loaded onto the aircraft within all safety and structural constraints.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Payload

Total cargo weight to be maximized.

**$payload$** : Total payload amount.

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

This context does not define independent sets; the objective ranges over the item and position sets from the stowage context.

---

## 6. Intermediate Values

This context does not define independent intermediate values.

---

## 7. Assertions

This context does not define independent assertions.

---

## 8. Constraints

### 1. Max Payload Limit

**[CN]**: 最大载荷限制

**Description**: Payload must not exceed aircraft maximum payload capacity.

$$
s.t. \quad payload \leq maxPayload
$$

---

## 9. Objective Function (if applicable)

Maximize total payload.

$$
\max \sum_{i \in I} \sum_{j \in J} weight_i \cdot x_{ij}
$$

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Payload | `Payload` | Total cargo weight on aircraft |
| Max Payload | `MaxPayload` | Maximum aircraft payload capacity |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Objective function | Payload maximization vs Cost minimization | Payload maximization is the primary business goal |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
