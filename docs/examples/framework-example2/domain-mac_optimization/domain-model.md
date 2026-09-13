# MAC optimization context model


## 1. Overview

Manages MAC optimization including longitudinal balance (MAC range constraints) and lateral balance constraints for aircraft weight distribution.

### 1. Dependent Contexts

1. Aircraft
2. Stowage
3. Mean Aerodynamic Chord (MAC)

---

## 2. Concepts / Entities

### 1. MAC Range

Defines allowable MAC percentage range based on total weight.

**$minMAC_{weight}$** : Minimum MAC percentage at given total weight.

**$maxMAC_{weight}$** : Maximum MAC percentage at given total weight.

### 2. Longitudinal Balance

Longitudinal balance constraints ensuring MAC is within allowable range per flight phase.

**$macRange$** : MAC range.

**$torque$** : Torque data.

### 3. Lateral Balance

Lateral balance constraints for wide-body aircraft ensuring symmetrical loading.

**$torque$** : Lateral torque data.

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

This context does not define independent sets; its balance limits range over positions and flight phases.

---

## 6. Intermediate Values

This context does not define independent intermediate values.

---

## 7. Assertions

This context does not define independent assertions.

---

## 8. Constraints

### 1. Longitudinal Balance Limit

**[CN]**: 纵向平衡限制

**Description**: MAC percentage must be within allowable range per flight phase.

$$
s.t. \quad minMAC_{weight} \leq mac \leq maxMAC_{weight}, \; \forall phase \in FlightPhases
$$

### 2. Lateral Balance Limit

**[CN]**: 横向平衡限制

**Description**: Lateral torque must be within allowable range (wide-body only).

$$
s.t. \quad |lateralTorque| \leq maxLateralTorque
$$

### 3. Horizontal Stabilizer Limit

**[CN]**: 水平安定面限制

**Description**: Horizontal stabilizer position must match MAC.

$$
s.t. \quad stabilizerPosition = f(mac), \; \forall phase \in FlightPhases
$$

---

## 9. Objective Function (if applicable)

Minimize MAC deviation from target range.

$$
\min |mac - macTarget|
$$

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| MAC Range | `MACRange` | Allowable MAC percentage range |
| Longitudinal Balance | `LongitudinalBalance` | Fore-aft weight balance |
| Lateral Balance | `LateralBalance` | Left-right weight balance |
| Horizontal Stabilizer | `HorizontalStabilizer` | Tail horizontal stabilizer |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| MAC range modeling | Linear vs Piecewise linear | Piecewise linear is more accurate |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
