# Bunch Generation Context Domain Model

[中文](../../../zh-cn/examples/framework-example4/domain-bunch_generation/domain-model)

## 1. Overview

The bunch-generation context builds feasible flight-task columns per aircraft,
including an initial pool and reduced-cost pricing. It returns domain objects
to the selection algorithm; it does not register master variables.

### 1. Dependent Contexts

Task, rule, crew, cargo, aircraft, and the compilation shadow-price contract.

## 2. Concepts / Entities

### 1. Route graph

**$G_a=(V_a,E_a)$** is the aircraft-specific task transition graph;
**$feasible_a(e)$** is its rule/time/aircraft feasibility predicate.

### 2. Flight-task bunch

**$B_b$** is an ordered task sequence for aircraft `a`; **$cost_b$** is its
total cost; **$cover(t,b)$** indicates task coverage.

### 3. Shadow-price map

**$\pi_t$** and **$\mu_a$** are master dual values used to calculate reduced cost.

## 3. Variables

No solver decision variables are owned by this context. A generated bunch is an
output object, not a binary variable until compilation inserts it.

## 4. Predicates

**initial(b)**: bunch belongs to the seed pool. **feasible(b)**: all task/rule,
aircraft, time, lock, and cost checks pass. **improving(b)**: reduced cost is
below the configured tolerance.

## 5. Sets

**$A$**: aircraft; **$T$**: tasks; **$G_a$**: graph for aircraft `a`; **$B_0$**:
initial bunches; **$B_t^{price}$**: pricing output at iteration `t`;
**$\Pi$**: shadow-price maps.

## 6. Intermediate Values

For bunch `b` of aircraft `a`, the pricing reduced cost is:

$$
rc(b)=cost_b-\sum_{t\in B_b}\pi_t-\mu_a.
$$

The exact cost and dual normalization are supplied by the configured cost
calculator and compilation policy.

## 7. Assertions

Every generated bunch has a known aircraft, a nonempty ordered task sequence,
and passes the configured feasibility judger:

$$
\forall b\in B_t^{price}:\quad feasible(b)=true.
$$

## 8. Constraints

This context does not register solver rows. Generation enforces graph edges,
locks, connection times, rule restrictions, and aircraft usability before a
column is returned.

## 9. Objective Function

Pricing searches for `rc(b)<0`; this is a subproblem criterion, not an
independent global objective registered by the context.

## 10. Algorithm References

| Algorithm | Source role | Purpose |
| --- | --- | --- |
| InitialFlightTaskBunchGenerator | initial pool | seed compilation |
| FlightTaskBunchGenerator | pricing | enumerate improving feasible bunches |

## 11. Ubiquitous Language

| Term | Symbol | Definition |
| --- | --- | --- |
| route graph | `G_a` | aircraft-specific feasible transition graph |
| bunch/column | `b` | ordered task sequence returned to compilation |
| pricing | `rc(b)` | search for an improving column |
| shadow price | `π, μ` | dual values from the master |

## 12. Design Decisions

| Decision | Alternative | Rationale |
| --- | --- | --- |
| keep generated columns outside the solver until insertion | preallocate all possible bunches | supports incremental column generation |

## 13. Change Log

| Version | Change | Reason |
| --- | --- | --- |
| 1.0 | Documented initial/pricing output boundary | Distinguish pricing results from master variables |

