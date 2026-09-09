# Task Context Domain Model

[中文](../../../zh-cn/examples/framework-example4/domain-task/domain-model)

## 1. Overview

The task context owns the flight-task vocabulary and recovery input used by
rule checking, bunch generation, and bunch compilation. The current Kotlin
`FlightTaskContext` is a data context; it does not register an optimization
variable family.

### 1. Dependent Contexts

None. Downstream contexts consume its task values.

## 2. Concepts / Entities

### 1. Flight task

**$id_t$**: stable task identifier; **$dep_t,arr_t$**: departure and arrival
airports; **$time_t$**: scheduled/actual time range; **$aircraft_t$**: assigned
or enabled aircraft; **$status_t$**: recovery permissions and hard-limit flags.

### 2. Flight leg and recovery assignment

**$leg_t$**: flight-leg plan and route; **$recovery_t$**: optional aircraft,
time, or route change; **$duration_t$**: task duration.

### 3. Flight-task bunch

An ordered sequence assigned to one aircraft. **$B_b$** is its task sequence,
**$aircraft_b$** its executor, and **$cost_b$** its calculated cost.

## 3. Variables

### 1. Decision Variables

None. Task and recovery values are inputs to generated columns.

### 2. Auxiliary Variables

None.

## 4. Predicates

**isFlight(t)**: task `t` is a flight-type task. **recoveryNeeded(t)**: the task
is in the recovery set. **aircraftChangeEnabled(t)**, **delayEnabled(t)**, and
**routeChangeEnabled(t)** classify allowed recovery operations.

## 5. Sets

**$T$**: all flight tasks; **$T^{F}$**: flight-type tasks; **$T^{R}$**: tasks
requiring recovery; **$A$**: aircraft; **$B$**: candidate task bunches.

## 6. Intermediate Values

For consecutive tasks in a bunch, the transition time is calculated from the
predecessor's end and successor's start:

$$
connectionTime(t,t')=start_{t'}-end_t.
$$

## 7. Assertions

Every flight task has a nonblank identifier, valid airports, and a consistent
time range. A consecutive flight-leg chain is airport-continuous:

$$
\forall(t,t')\in B_b:\quad arr_t=dep_{t'}.
$$

## 8. Constraints

This context contributes no solver constraints by itself. Its status and time
values are consumed by the rule and bunch-generation feasibility services.

## 9. Objective Function

None. Cost is calculated for a generated bunch and consumed by compilation.

## 10. Algorithm References

No independent algorithm document.

## 11. Ubiquitous Language

| Term | Symbol | Definition |
| --- | --- | --- |
| flight task | `t` | schedulable flight/recovery unit |
| flight leg | `leg_t` | task plan with airports and time |
| recovery assignment | `recovery_t` | optional change applied to a task |
| bunch | `b` | ordered tasks assigned to one aircraft |

## 12. Design Decisions

| Decision | Alternative | Rationale |
| --- | --- | --- |
| keep task data separate from compilation | register task variables globally | permits multiple generation/compilation policies |

## 13. Change Log

| Version | Change | Reason |
| --- | --- | --- |
| 1.0 | Documented the data-only task boundary | Avoid presenting task entities as a solved model |

