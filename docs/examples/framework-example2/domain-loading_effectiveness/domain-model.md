# Loading effectiveness context model


## 1. Overview

Manages loading effectiveness constraints for operational efficiency — including trailer loading, sequential loading, transfer adjacency, and source/destination grouping.

### 1. Dependent Contexts

1. Aircraft
2. Stowage

---

## 2. Concepts / Entities

### 1. Advice Loading

Suggested loading amounts and weights per position for predistribution mode.

**$adviceAmount_{j}$** : Advice load amount at position $j$.

**$adviceWeight_{j}$** : Advice load weight at position $j$.

### 2. Transfer Adjacent Loading

Same-source/same-destination adjacency constraints for transfer efficiency.

**$adjacentPositions$** : List of adjacent position pairs.

**$sources$** : List of source stations.

**$destinations$** : List of destination stations.

### 3. Sequential Loading

Sequential loading constraints based on position ordering.

**$orderedPositions$** : List of ordered position pairs.

### 4. Trailer Loading

Trailer change and circling constraints for full-load mode.

**$trailers$** : List of trailers.

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

This context does not own independent sets. It uses $I$ for cargo items, $J$ for stowage positions, $A\subseteq J\times J$ for adjacent position pairs, and $R$ for the ordered trailer pairs supplied to the trailer-loading model.

---

## 6. Intermediate Values

The source trailer-loading model provides the intermediate symbols `trailerChange[p_1,p_2]` and `trailerCircling[p_1,p_2]`. For an ordered trailer pair $(r,s)\in R$ and an adjacent position pair $(j,k)\in A$, write `trailerChange[p_1,p_2]` as $\Delta_{rs,jk}$. Its dynamic `IfFunction` expression is:

$$
\Delta_{rs,jk}=\operatorname{If}\!\left(L^{s}_j+L^{r}_k-2+\tau\right)
$$

Here $L^{s}_j$ is the load amount at $j$ from trailer $s$, $L^{r}_k$ is the load amount at $k$ from trailer $r$, and $\tau$ is the nonzero comparison offset used by the source `IfFunction`. The page records `trailerCircling[p_1,p_2]` as a related model-layer intermediate but does not assign it to the trailer-change objective below.

---

## 7. Assertions

This context does not define independent assertions.

---

## 8. Constraints

The two adjacency entries below are business-level loading rules. Their concrete decision-variable registration belongs to the source pipeline; this page does not restate either rule as a generic `s.t.` formula.

### 1. Same Source Adjacent Limit

**[CN]**: 同源邻接限制

**Description**: Same-source items should be loaded in adjacent positions to reduce transfer handling. `SameSourceAdjacentLimit` is the source-pipeline boundary for the concrete expression; this page records the business rule without asserting a generic `s.t.` row.

### 2. Same Destination Adjacent Limit

**[CN]**: 同目的地邻接限制

**Description**: Same-destination items should be loaded in adjacent positions to reduce transfer handling. `SameDestinationAdjacentLimit` is the source-pipeline boundary for the concrete expression; this page records the business rule without asserting a generic `s.t.` row.

### 3. Trailer Change Limit

**[CN]**: 拖车变更限制

**Description**: The source limit minimizes a weighted sum of the registered `trailerChange[p_1,p_2]` intermediates over ordered trailer pairs and adjacent position pairs. It is an objective input, not an `s.t.` constraint; the exact expression is given in Section 9.

---

## 9. Objective Function (if applicable)

The source-level trailer-change objective is:

$$
\min \sum_{(r,s)\in R}\sum_{(j,k)\in A} c_{rs,jk}\,\Delta_{rs,jk}
$$

Here $c_{rs,jk}$ abbreviates the source coefficient call `coefficient((k,r),(j,s))`, preserving its position/trailer argument order. `loadingOperationalCost` is retained only as a business-level aggregate label for any additional operational terms selected by a pipeline. The Demo2 Kotlin source exposes no single linear expression with this name, so this page does not assert a solver formula or registered objective for it.

---

## 10. Algorithm References

This context does not define an independent algorithm reference.

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Advice Loading | `AdviceLoading` | Advice load amount per position |
| Transfer Adjacent Loading | `TransferAdjacent` | Adjacency requirement for transfer cargo |
| Sequential Loading | `SequentialLoading` | Order-based loading constraints |
| Trailer Loading | `TrailerLoading` | Trailer-related loading constraints |
| Trailer Change | `trailerChange[p_1,p_2]` | Registered intermediate for cross-trailer changes |
| Loading Operational Cost | `loadingOperationalCost` | Business-level aggregate label; no single Demo2 expression is asserted |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Adjacency definition | Loading order vs Physical position | Loading order matches operational reality |

---

## 13. Change Log

No context-specific change entries are recorded on this page.
