# Bunch Selection Context Domain Model

[中文](../../../zh-cn/examples/framework-example4/domain-bunch_selection/domain-model)

## 1. Overview

The selection context owns the branch-and-price policy and coordinates bunch
generation with bunch compilation. The current Kotlin context is an algorithm
orchestrator; it does not declare an independent variable family.

### 1. Dependent Contexts

Bunch generation, bunch compilation, task, rule, passenger, and aircraft.

## 2. Concepts / Entities

### 1. Selection policy

**$P$** contains reduced-cost tolerance, column limits, executor minimums, and
time limits; **$gen$** and **$compile$** are the generation/compilation ports.

### 2. Shadow-price map

**$\Pi$** maps master symbols to dual values consumed by pricing.

### 3. Bunch solution

**$S$** contains selected bunches and the final compilation result.

## 3. Variables

No solver variables are registered by `BunchSelectionContext`. Branch decisions
and node bounds belong to the generic branch-and-price algorithm state.

## 4. Predicates

**improving(b)**: generated bunch has acceptable reduced cost. **fractional(n)**:
node solution has a fractional selection. **integral(n)**: selected columns pass
the integer acceptance check. **compatible(b,n)**: bunch respects node decisions.

## 5. Sets

**$N$**: branch nodes; **$B_n$**: columns compatible with node `n`;
**$D_n$**: require/forbid branch decisions; **$\Pi_n$**: node dual map.

## 6. Intermediate Values

For node `n`, the queue bound and pricing result are:

$$
lowerBound_n=RMP(n),
\qquad
pricing_n=\{b\in B_n\mid rc_n(b)<-\epsilon\}.
$$

## 7. Assertions

Child nodes inherit immutable branch decisions and only compatible columns:

$$
\forall b\in B_n:\quad compatible(b,n)=true.
$$

An integer candidate is accepted only after compilation and domain feasibility
checks succeed.

## 8. Constraints

No independent solver constraints are registered here. The policy imposes
algorithmic limits on time, nodes, columns, and reduced-cost tolerance; these
are termination conditions rather than mathematical rows.

## 9. Objective Function

The context delegates the master objective to bunch compilation. It does not
introduce a second objective.

## 10. Algorithm References

| Algorithm | Source role | Purpose |
| --- | --- | --- |
| BranchAndPriceAlgorithm | selection service | process nodes, branch fractional solutions, and coordinate pricing |

## 11. Ubiquitous Language

| Term | Symbol | Definition |
| --- | --- | --- |
| branch node | `n` | RMP plus inherited decisions |
| branch decision | `D_n` | require/forbid assignment or link choice |
| incumbent | `S` | best accepted integer solution |
| lower bound | `lowerBound_n` | node RMP bound |

## 12. Design Decisions

| Decision | Alternative | Rationale |
| --- | --- | --- |
| keep branching in orchestration | encode branch state as domain variables | generic algorithm can reuse compilation/generation ports |

## 13. Change Log

| Version | Change | Reason |
| --- | --- | --- |
| 1.0 | Documented selection as orchestration context | Avoid inventing a second optimization model |

