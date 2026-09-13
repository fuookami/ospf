# MAC context model


## 1. Overview

Computes Mean Aerodynamic Chord (MAC) percentage, longitudinal/lateral torque, CLIM, and index for each flight phase from aircraft and stowage data.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Torque

Computes longitudinal torque, lateral torque, CLIM, and index per flight phase from load, fuel, fuselage, and formula data.

**$longitudinalTorque_{phase}$** : Longitudinal torque per flight phase.

**$lateralTorque$** : Lateral torque (wide-body only).

**$clim$** : CLIM (Center of Gravity Index Moment).

**$index_{phase}$** : Index per flight phase.

### 2. MAC

Computes MAC percentage as a linear intermediate symbol from torque index and total weight.

**$mac$** : MAC percentage, linear intermediate symbol.

### 3. Horizontal Stabilizer

Horizontal stabilizer position and limits for balance computation.

**$key$** : Horizontal stabilizer identifier.

**$points$** : Horizontal stabilizer data points.

**$limit$** : Horizontal stabilizer limit.

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

This context does not define independent sets; the intermediate values range over the aircraft's positions and flight phases.

---

## 6. Intermediate Values

### 1. Longitudinal Torque

**Description**: Longitudinal torque per flight phase, composed of load torque, fuel torque, fuselage moment, and liferaft moment.

$$
longitudinalTorque_{phase} = \sum_{j \in J} loadLongitudinalTorque_j + fuelWeight_{phase} \cdot fuelArm_{phase} + dow \cdot balancedArm + liferaftWeight \cdot liferaftArm
$$

### 2. MAC Percentage

**Description**: Mean Aerodynamic Chord percentage, computed from torque index and total weight.

$$
mac = \frac{index_{TakeOff}}{tow} \cdot 100\%
$$

For each phase $p$, the phase index is derived from the corresponding torque
and aircraft formula. The exact coefficient and unit conversion are supplied
by `Formula` and the aircraft model; the displayed equations are the
contract-level expansion, not a replacement for those typed coefficients. MAC
is an intermediate consumed by the balance and airworthiness contexts, not an
independent decision variable.

## 7. Assertions

Every referenced phase has a valid fuel/fuselage formula and every arm, weight,
and index uses the aircraft's configured compatible units. A phase with no
required formula is not silently treated as a zero torque phase.

## 8. Constraints

This context exposes derived symbols only. Balance and envelope constraints are
registered by `mac_optimization` and `airworthiness_security`; defining MAC
does not add a solver row by itself.

---

## 9. Objective Function (if applicable)

This context does not define an independent objective function.

---

## 10. Algorithm References

No independent algorithm document; phase formulas are supplied by the aircraft `Formula` model and MAC aggregation.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Torque | `Torque` | Force times arm distance |
| Mean Aerodynamic Chord | `MAC` | Wing mean aerodynamic chord length |
| Horizontal Stabilizer | `HorizontalStabilizer` | Tail horizontal stabilizer |
| CLIM | `CLIM` | Center of gravity index moment |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| MAC computation formula | Linearized vs Non-linear | Linearization facilitates optimization model solving |

## 13. Change Log

| Version | Change | Reason |
| --- | --- | --- |
| 1.1 | Clarified phase scope, units, and downstream constraint ownership | Distinguish intermediates from active balance limits |
