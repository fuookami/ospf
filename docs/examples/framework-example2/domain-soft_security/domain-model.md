# Soft security context model


## 1. Overview

Manages soft safety constraints including empty loading division, main deck door empty preference, and ballast weight advice — constraints that improve safety but can be relaxed if needed.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Divide Empty Loading

Ensures empty positions are divided rather than clustered for structural safety.

**$positions$** : Position list.

**$load$** : Load data.

---

## 3. Variables

### 1. Decision Variables

This context reuses decision variables from the stowage context and does not define independent decision variables.

### 2. Auxiliary Variables

This context does not define independent auxiliary variables.

---

## 4. Predicates

The business-level relation `distributedEmptyPositions(J)` names the divide-empty preference. In the Demo2 pipeline, that preference is realized by three source intermediate expressions and three separate soft objective inputs over adjacent position pairs; the relation itself is not a registered solver constraint.

---

## 5. Sets

This context does not own independent sets. It uses $I$ for cargo items, $J$ for stowage positions, $A\subseteq J\times J$ for adjacent position pairs, $J^{\mathrm{emptyHated}}\subseteq J$ for positions marked `EmptyHated`, and $J^{\mathrm{beside}}\subseteq J$ for positions beside a main-deck door.

---

## 6. Intermediate Values

The source `DivideEmptyLoading` model registers three index-aligned intermediate expressions for each $(j,k)\in A$:

1. `emptyBetweenCargo[p]`, written $e^{\mathrm{bc}}_{jk}$, describes non-empty cargo at the first position against total cargo at the second.
2. `emptyCargoBetweenCargo[p]`, written $e^{\mathrm{cb}}_{jk}$, describes empty cargo at the first position against non-empty cargo at the second.
3. `emptyBetweenEmptyCargo[p]`, written $e^{\mathrm{bb}}_{jk}$, describes empty cargo at the first position against total cargo at the second.

For positions whose load is decided dynamically, the source `IfFunction` branches are represented by:

$$
\begin{aligned}
e^{\mathrm{bc}}_{jk} &= \operatorname{If}\!\left(L^{\mathrm{nonempty}}_j-\left(L^{\mathrm{all}}_k+1\right)+\tau\right),\\
e^{\mathrm{cb}}_{jk} &= \operatorname{If}\!\left(L^{\mathrm{empty}}_j+L^{\mathrm{nonempty}}_k-2+\tau\right),\\
e^{\mathrm{bb}}_{jk} &= \operatorname{If}\!\left(L^{\mathrm{empty}}_j-\left(L^{\mathrm{all}}_k+1\right)+\tau\right).
\end{aligned}
$$

Here $\tau$ denotes the nonzero comparison offset used by the source `IfFunction`. Fixed-range branches in the source can instead collapse an expression to zero, a load amount, or another constant; these intermediates are model expressions, not additional solver constraints.

---

## 7. Assertions

This context does not define independent assertions.

---

## 8. Constraints

This section names the context's soft limits. In the Demo2 pipeline, the four entries below contribute objective terms; they are not `s.t.` rows. The displayed expressions use source-layer symbols and do not assert a new aggregate objective.

### 1. Empty Hated Limit

**[CN]**: 空载厌恶限制

**Description**: The pipeline registers a soft objective term for positions marked `EmptyHated`; its exact source-aligned expression, including the position coefficient and full-load expression, is given in Section 9.

### 2. Main Deck Door Empty Limit

**[CN]**: 主甲板舱门空载限制

**Description**: The pipeline minimizes stowage assignments beside main-deck doors (B757/B767), rather than imposing an `empty_j = 1` row; the exact source-aligned expression is given in Section 9.

### 3. Divide Empty Loading Limit

**[CN]**: 空载分离限制

**Description**: The pipeline registers three separate soft objective inputs over adjacent pairs. `distributedEmptyPositions(J)` is only the business-level name for this preference, not an `s.t.` constraint; the three source-aligned expressions are given in Section 9.

### 4. Advice Ballast Weight Limit

**[CN]**: 建议压舱物重量限制

**Description**: The pipeline adds a one-sided threshold-slack objective only when an advice value exists; the negative-deviation semantics and exact source-aligned expression are given in Section 9, with no hard lower-bound `s.t.` row asserted here.

---

## 9. Objective Function (if applicable)

The context contributes the following separate soft objective inputs. Their activation and any higher-level aggregation belong to the selected mode pipeline; this page does not invent a single generic penalty-sum expression, and none of these entries is a hard `s.t.` row.

### 1. Empty Hated

$$
\min \sum_{j \in J^{\mathrm{emptyHated}}} c_j \left(1 - load^{\mathrm{full}}_j\right)
$$

Here $c_j$ is the position coefficient and $load^{\mathrm{full}}_j$ is the source full-load expression.

### 2. Main Deck Door Empty

$$
\min \sum_{i \in I}\sum_{j \in J^{\mathrm{beside}}} c_i\,stowage_{ij}
$$

Here $J^{\mathrm{beside}}$ is the set of positions whose door ubiety is `Beside`.

### 3. Divide Empty Loading

$$
\begin{aligned}
\min\;&\sum_{(j,k)\in A} c^{\mathrm{bc}}_{jk}e^{\mathrm{bc}}_{jk},\\
\min\;&\sum_{(j,k)\in A} c^{\mathrm{cb}}_{jk}e^{\mathrm{cb}}_{jk},\\
\min\;&\sum_{(j,k)\in A} c^{\mathrm{bb}}_{jk}e^{\mathrm{bb}}_{jk}.
\end{aligned}
$$

These are three separate source objective inputs, with the coefficient and intermediate families defined in Section 6.

### 4. Advice Ballast Weight

$$
\min c_{\mathrm{ballast}}\,s^{-}_{\mathrm{ballast}},
\qquad s^{-}_{\mathrm{ballast}}\geq 0
$$

The term is added only when an advice value exists; $s^{-}_{\mathrm{ballast}}$ is the one-sided slack for falling below the advice threshold.

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Divide Empty Loading | `DivideEmptyLoading` | Distributed empty positions |
| Empty Hated | `EmptyHated` | Soft penalty for empty positions |
| Main Deck Door Empty | `MainDeckDoorEmpty` | Soft penalty for stowage beside main-deck doors |
| Advice Ballast Weight | `AdviceBallastWeight` | Advisory ballast weight value |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Constraint type | Hard vs Soft | Soft constraints allow relaxation when needed |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
