# Rule Context Domain Model

[中文](../../../zh-cn/examples/framework-example4/domain-rule/domain-model)

## 1. Overview

The rule context defines link, restriction, flow-control, and feasibility
services used to judge whether a recovered task sequence is legal. It supplies
predicates and costs to bunch generation; it is not an independent master
model.

### 1. Dependent Contexts

Task, aircraft, and infrastructure time/cost types.

## 2. Concepts / Entities

### 1. Link

**$prev_l$**, **$succ_l$**: predecessor and successor tasks;
**$splitCost_l$**: link split cost; **$type_l$**: connecting, stopover, or
connection-time-ignoring link.

### 2. Restriction

**$category_r$**: restriction category; **$weight_r$**: violation weight;
**$cost_r$**: optional violation cost; **$condition_r$**: task/airport/aircraft
condition.

### 3. Flow control

**$airport_f$**, **$scene_f$**, **$time_f$**, and **$capacity_f$** describe an
arrival/departure flow window and its capacity.

## 3. Variables

No decision or auxiliary variables are registered by `RuleContext`.

## 4. Predicates

**linkType(l)** classifies link semantics. **violates(t,r)** means task `t`
violates restriction `r`. **flowClosed(f)** means the flow capacity is zero.
**feasible(t,t')** means the connection time, rule, and aircraft tests pass.

## 5. Sets

**$L$**: links; **$L^{conn}$**, **$L^{stop}$**, and **$L^{ignore}$** are link
subsets. **$R$**: restrictions; **$F$**: flow-control windows;
**$T^{feas}$**: task transitions accepted by all enabled rules.

## 6. Intermediate Values

The connection service derives a transition time and total violation cost:

$$
\Delta(t,t')=start_{t'}-end_t,
\qquad
cost(t,t')=\sum_{r\in R}weight_r\,\mathbf1[violates(t,t',r)].
$$

## 7. Assertions

Every link has two task endpoints, and a connecting link only connects tasks
whose airport/time data are defined. A flow window has nonnegative capacity and
nonempty time range.

## 8. Constraints

No independent solver rows are registered. Feasibility is a service predicate:

$$
s.t.\quad feasible(t,t')\Longleftrightarrow
\Delta(t,t')\ge requiredConnectionTime(t,t')\wedge
\neg violates(t,t',r)\ \forall r\in R^{hard}.
$$

## 9. Objective Function

None. Transition and restriction costs are inputs to a bunch cost calculator.

## 10. Algorithm References

No standalone algorithm document; `FeasibilityJudger`, connection-time, and
cost calculators implement the predicates.

## 11. Ubiquitous Language

| Term | Symbol | Definition |
| --- | --- | --- |
| link | `l` | relation between consecutive tasks |
| restriction | `r` | rule limiting a task transition |
| flow control | `f` | airport/time capacity window |
| feasibility | `feasible` | all enabled rules pass |

## 12. Design Decisions

| Decision | Alternative | Rationale |
| --- | --- | --- |
| evaluate rules before column insertion | add invalid bunches then filter in master | keeps generated columns feasible |

## 13. Change Log

| Version | Change | Reason |
| --- | --- | --- |
| 1.0 | Documented rule predicates as a service boundary | Separate feasibility from master variables |

