# Stowage context model


## 1. Overview

Manages cargo stowage assignment decisions — which items go to which positions — including load weight computation, payload calculation, total weight, and max load weight.

### 1. Dependent Contexts

1. Aircraft

---

## 2. Concepts / Entities

### 1. Item

A cargo item with destination, weight, ULD, location tags, cargo type, priority, and status.

**$id_{i}$** : Item unique identifier.

**$dest_{i}$** : Destination (IATA code).

**$weight_{i}$** : Item weight, unit kg.

**$uld_{i}$** : Associated ULD (optional).

**$location_{i}$** : Location tags (Main/Low/Bulk/Head/Tail).

**$cargo_{i}$** : Cargo type and priority.

**$status_{i}$** : Status (Loaded/Preassigned/Optional/Reserved/AdjustmentNeeded).

**$order_{i}$** : Order information (hardstand time, reweigh time, car-board info).

### 2. Position

Stowage position with max load amount (MLA), predicate load weight (PLW), recommended load weight, and status.

**$spaceName_{j}$** : Space name.

**$mla_{j}$** : Max Load Amount.

**$plw_{j}$** : Predicate Load Weight, its definition is further specified in *Stowage*.

**$mlw_{j}$** : Max Load Weight, its definition is further specified in *Airworthiness Security*.

**$coordinate_{j}$** : Coordinate (longitudinal arm, lateral arm).

**$location_{j}$** : Set of location tags.

### 3. Flight

Flight information.

**$flightNo$** : Flight number.

**$departure$** : Departure airport (IATA code).

**$arrival$** : Arrival airport (IATA code).

### 4. Appointment

Pre-assigned item-to-position appointments.

**$appointment$** : Item-to-position pre-assignment mapping.

### 5. Ballast

Ballast weight for balance correction.

**$minBallastWeight$** : Minimum ballast weight.

---

## 3. Variables

### 1. Decision Variables

**$x_{ij}$** : Whether item $i$ is assigned to position $j$, binary variable, domain is $\{0, 1\}$, for feasible pairs $(i,j)\in IJ^{feas}$; status-fixed pairs are fixed to zero or one.

**$y_{j}$** : Predicate load weight at position $j$, continuous variable (kg), domain is $[0, MLW_j]$, $\forall j \in Positions$.

**$z_{j}$** : Recommended load weight at position $j$, integer variable (kg), domain is $[0, \lfloor MLW_j \rfloor]$, $\forall j \in Positions$.

### 2. Auxiliary Variables

**$u_{ij}$** : Adjustment variable for item $i$ at position $j$, balance-ternary variable, domain is $\{-1,0,1\}$, for adjustment pairs $(i,j)\in IJ^{adj}$; non-adjustment pairs are fixed to zero.

---

## 4. Predicates

### 1. Item Status Predicates

**`stowageNeeded(item)`** : Item requires position assignment (status is `Preassigned` or `Optional`).

**`adjustmentNeeded(item)`** : Item requires position adjustment (status is `AdjustmentNeeded`).

**`loaded(item)`** : Item is loaded (status is `Loaded` or `AdjustmentNeeded`).

### 2. Position Status Predicates

**`stowageNeeded(position)`** : Position requires item assignment.

**`available(position)`** : Position is available for loading.

**`predicateWeightNeeded(position)`** : Position requires predicate weight variable.

**`recommendedWeightNeeded(position)`** : Position requires recommended weight variable.

---

## 5. Sets

### 1. Items

**$I$** : Set of all cargo items.

**$I^{pre}$** : Subset of items satisfying predicate stowageNeeded, items requiring position assignment.

**$I^{adj}$** : Subset of items satisfying predicate adjustmentNeeded, items requiring position adjustment.

**$I^{opt}$** : Subset of items with Optional status, optionally loaded items.

### 2. Positions

**$J$** : Set of all stowage positions.

**$J^{avl}$** : Subset of positions satisfying predicate available, positions available for loading.

**$J^{pw}$** : Subset of positions satisfying predicate predicateWeightNeeded, positions requiring predicate weight.

**$J^{rw}$** : Subset of positions satisfying predicate recommendedWeightNeeded, positions requiring recommended weight.

### 3. Item-Position Pairs

**$IJ^{feas}$** : Set of feasible item-position assignment pairs, i.e., $(i, j)$ pairs satisfying stowageNeeded(item, position).

---

## 6. Intermediate Values

### 1. Load Amount

**Description**: Number of items loaded at each position.

$$
loadAmount_j = \sum_{i \in I} stowage_{ij}, \; \forall j \in J
$$

### 2. Actual Load Weight

**Description**: Actual load weight at each position.

$$
actualLoadWeight_j = \sum_{i \in I} weight_i \cdot stowage_{ij}, \; \forall j \in J
$$

### 3. Estimate Load Weight

**Description**: Estimated load weight including predicate and recommended weights.

$$
estimateLoadWeight_j = \sum_{i \in I} weight_i \cdot stowage_{ij} + y_j + z_j, \; \forall j \in J
$$

The `stowage` expression is the status-aware bridge
$s_{ij}=x_{ij}+u_{ij}+loaded_{ij}$ for pairs that need stowage and is zero for
inapplicable pairs. The model fixes `y` and `z` according to the
`predicateWeightNeeded` and `recommendedWeightNeeded` position predicates.

## 7. Assertions

The data model requires unique item/position identifiers and a valid aircraft
weight unit. Loaded items and unavailable positions have fixed contributions;
they are not free solver decisions. Appointment pairs are a subset of feasible
pairs, and every item in $I^{pre}$ has at least one feasible position.

---

## 8. Constraints

### 1. Item Assignment Limit

**[CN]**: 货物分配限制

**Description**: Each item requiring stowage must be assigned to exactly one position.

$$
s.t. \quad \sum_{j \in J} x_{ij} = 1, \; \forall i \in I^{pre}
$$

### 2. Load Amount Limit

**[CN]**: 装载数量限制

**Description**: Load amount per position must not exceed the maximum load amount (MLA).

$$
s.t. \quad \sum_{i \in I} stowage_{ij} \leq mla_j, \; \forall j \in J
$$

### 3. Load Weight Limit

**[CN]**: 装载重量限制

**Description**: Load weight per position must not exceed the maximum load weight (MLW).

$$
s.t. \quad actualLoadWeight_j \leq mlw_j, \; \forall j \in J
$$

### 4. Appointment Limit

**[CN]**: 预约限制

**Description**: Pre-assigned item-to-position appointments must be respected.

$$
s.t. \quad x_{ij} = 1, \; \forall (i, j) \in Appointment
$$

---

## 9. Objective Function (if applicable)

This context does not define an independent objective function; it only provides constraints.

---

## 10. Algorithm References

| Algorithm | Source boundary | Used for |
| --- | --- | --- |
| Benders decomposition | mode-specific application master/subproblem registration | separating stowage and airworthiness pipelines |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Item | `Item` | Cargo unit to be loaded |
| Position | `Position` | Cargo loading position on aircraft |
| Stowage | `Stowage` | Item-to-position assignment decision |
| Load | `Load` | Load weight and amount at position |
| Payload | `Payload` | Total cargo weight on aircraft |
| Total Weight | `TotalWeight` | Aircraft total weight per flight phase |
| Max Load Weight | `MaxLoadWeight` | Maximum allowable load weight at position |
| Ballast | `Ballast` | Ballast weight for balance |
| Predicate Load Weight | `PLW` | Predicted load weight |
| Max Load Amount | `MLA` | Maximum load amount at position |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale |
|----------|--------------|-----------|
| Stowage Mode Selection | FullLoad / Predistribution / WeightRecommendation | Different modes for different business scenarios |
| Benders Decomposition | Master/Sub problem separation | Airworthiness constraints in sub problem, others in master |

---

## 13. Change Log

| Version | Change | Reason |
| --- | --- | --- |
| 1.1 | Clarified status predicates, balance-ternary adjustment, and local model boundary | Match the `Stowage` aggregate and pipeline registration |
