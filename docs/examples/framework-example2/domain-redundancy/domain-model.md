# Redundancy context model


## 1. Overview

Manages redundancy and experimental longitudinal balance constraints for weight distribution analysis and safety margins.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Redundancy

Redundancy model for weight distribution safety margins.

**$redundancy$** : Redundancy value.

### 2. Experimental Longitudinal Balance

Experimental longitudinal balance model based on redundancy computations.

**$experimentalBalance$** : Experimental longitudinal balance value.

**$redundancy$** : Dependent redundancy value.

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

This context does not define independent sets; its limits range over the stowage and flight-phase sets.

---

## 6. Intermediate Values

This context does not define independent intermediate values.

---

## 7. Assertions

This context does not define independent assertions.

---

## 8. Constraints

### 1. Redundancy Limit

**[CN]**: 冗余限制

**Description**: Redundancy must be within acceptable bounds.

$$
s.t. \quad minRedundancy \leq redundancy \leq maxRedundancy
$$

### 2. Experimental Longitudinal Balance Limit

**[CN]**: 实验纵向平衡限制

**Description**: Experimental longitudinal balance must be within bounds.

$$
s.t. \quad minBalance \leq experimentalBalance \leq maxBalance
$$

---

## 9. Objective Function (if applicable)

This context does not define an objective function; it only provides constraints.

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Redundancy | `Redundancy` | Safety margin for weight distribution |
| Experimental Longitudinal Balance | `ExperimentalLongitudinalBalance` | Longitudinal balance based on redundancy |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Redundancy modeling | Independent vs Coupled | Coupling with longitudinal balance matches reality |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
