# Bandwidth Context Domain Model

> English | [中文](../../../zh-cn/examples/framework-example1/domain-bandwidth/domain-model)

[toc]

## 1. Overview

The Bandwidth Context consumes the Route Context's graph, services, and assignment aggregate; it registers per-edge/per-service bandwidth variables, bandwidth intermediates, demand and capacity pipelines, and the bandwidth-cost objective. The authoritative implementation is [`BandwidthContext.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/BandwidthContext.kt), [`EdgeBandwidth.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/model/EdgeBandwidth.kt), and [`PipelineListGenerator.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/service/PipelineListGenerator.kt).

### 1. Dependent Contexts

1. Route Context — supplies `Graph`, `Service`, the assignment variable $x_{n,s}$, and the service/node assignment intermediates $A_s$ and $A_n$.

---

## 2. Concepts / Entities

### 1. Edge

A directed edge from the Route Context graph. Only edges whose source is a normal node can receive a nonzero bandwidth variable in this context.

**$from(e)$** : The source node of edge $e$.
**$to(e)$** : The target node of edge $e$.
**$maxBandwidth_{e}$** : The maximum bandwidth bound of edge $e$, an unsigned integer.
**$costPerBandwidth_{e}$** : The per-unit bandwidth cost of edge $e$, an unsigned integer.

### 2. Service

A Route Context service whose traffic is allocated on graph edges.

**$capacity_{s}$** : The service's bandwidth capacity, an unsigned integer.
**$cost_{s}$** : The service-use cost used by the Route Context objective, an unsigned integer.

### 3. Node

A Route Context graph node, either normal or client.

**$demand_{n}$** : The incoming bandwidth requirement, defined for client nodes.
**$x_{n,s}$** : The imported Route Context assignment variable; it is not redeclared by this context.

### 4. Assignment Counts

The Route Context exposes the registered intermediate values used here.

**$A_s$** : The number of normal nodes assigned to service $s$.
**$A_n$** : The number of services assigned to normal node $n$ (and the zero polynomial for a client node).

---

## 3. Variables

### 1. Decision Variables

**$y_{e,s}$** : Bandwidth allocated to service $s$ on edge $e$, a dimensionless nonnegative unsigned integer, with domain and registration bounds

$$
0 \le y_{e,s} \le maxBandwidth_e,\quad \forall e \in E^{normal},\ \forall s \in S;
\qquad y_{e,s}=0,\quad \forall e \in E^{client},\ \forall s \in S.
$$

The Kotlin declaration is `UIntVariable2("y", Shape2(edges.size, services.size))`; the range is set per edge in `EdgeBandwidth.register`. The Route Context variable $x_{n,s}$ is an imported decision variable, not a Bandwidth Context declaration.

### 2. Auxiliary Variables

No separate auxiliary decision variables are declared. All bandwidth totals and node/service flows in Section 6 are registered linear intermediate symbols.

---

## 4. Predicates

### 1. Node Type

> Predicates classify entity sets; each predicate defines a subset.

**$normal(n)$** : Node $n$ is a `NormalNode`.
**$client(n)$** : Node $n$ is a `ClientNode`.

### 2. Edge Incidence

**$from(e)=n$** : Edge $e$ leaves node $n$.
**$to(e)=n$** : Edge $e$ enters node $n$.
**$from\_normal(e)$** : The source of $e$ satisfies $normal$; this is the derived filter used by all active edge bandwidth and cost pipelines.

---

## 5. Sets

### 1. Nodes

**$N$** : The universal set of nodes in the Route Context graph.

**$N^{normal}$** : The normal/transit nodes, satisfying $normal$.
**$N^{client}$** : The client/terminal nodes, satisfying $client$.

### 2. Edges

**$E$** : The universal set of directed graph edges.

**$E^{normal}$** : The subset $\{e \in E \mid from(e) \in N^{normal}\}$, whose $y_{e,s}$ variables have the edge maximum as their registration upper bound.
**$E^{client}$** : The subset $E \setminus E^{normal}$, whose $y_{e,s}$ variables are fixed to zero. The source code uses `!from(normal)` for this range assignment.

### 3. Services

**$S$** : The Route Context's generated service set.

### 4. Entity Pairs / Relations

**$R_{inc}$** : The edge-to-node incidence relation represented by the source and target predicates $from(e)$ and $to(e)$; no separate relation variable is registered.

---

## 6. Intermediate Values

### 1. Edge Total Bandwidth

**Description**: `EdgeBandwidth.bandwidth` sums all service bandwidths on an edge. The implementation uses the zero polynomial for edges whose source is not normal.

$$
B_e = \begin{cases}
\displaystyle\sum_{s \in S} y_{e,s},& e \in E^{normal}\\
0,& e \in E^{client}
\end{cases}
\qquad \forall e \in E
$$

### 2. Service In-Degree Bandwidth

**Description**: `ServiceBandwidth.inDegree` is the bandwidth of service $s$ entering node $n$ through every edge whose target is $n$.

$$
I_{n,s} = \sum_{e \in E:\,to(e)=n} y_{e,s},\qquad \forall n \in N,\ \forall s \in S
$$

### 3. Service Out-Degree Bandwidth

**Description**: `ServiceBandwidth.outDegree` is the bandwidth of service $s$ leaving a normal node. It is a zero polynomial for client nodes.

$$
O_{n,s} = \begin{cases}
\displaystyle\sum_{e \in E:\,from(e)=n} y_{e,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N,\ \forall s \in S
$$

### 4. Service Net Out-Flow

**Description**: `ServiceBandwidth.outFlow` subtracts service in-degree from service out-degree at normal nodes. It is not a conservation constraint.

$$
F_{n,s} = \begin{cases}
O_{n,s}-I_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N,\ \forall s \in S
$$

### 5. Node Aggregated In-Degree Bandwidth

**Description**: `NodeBandwidth.inDegree` aggregates all services entering each graph node.

$$
I_n = \sum_{s \in S} I_{n,s},\qquad \forall n \in N
$$

### 6. Node Aggregated Out-Degree Bandwidth

**Description**: `NodeBandwidth.outDegree` aggregates service out-degree at normal nodes and uses a zero polynomial for client nodes.

$$
O_n = \begin{cases}
\displaystyle\sum_{s \in S} O_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N
$$

### 7. Node Aggregated Net Out-Flow

**Description**: `NodeBandwidth.outFlow` aggregates service net out-flow at normal nodes and uses a zero polynomial for client nodes.

$$
F_n = \begin{cases}
\displaystyle\sum_{s \in S} F_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N
$$

### 8. Candidate Maximum Outgoing Capacity (Not a Registered Symbol)

**Description**: `Node.maxOutDegree()` computes the sum of maximum bandwidths on the node's attached outgoing edges for the unregistered `TransferNodeBandwidthConstraint`. It is a helper value, not an intermediate added by `Aggregation.register`.

$$
M_n = \sum_{e \in E:\,from(e)=n} maxBandwidth_e,\qquad \forall n \in N^{normal}
$$

---

## 7. Assertions

> Assertions describe properties guaranteed by variable ranges and symbol construction. They do not imply an additional pipeline constraint.

### 1. Non-Normal-Source Edge Bandwidth Is Zero

**Description**: The range assignment in `EdgeBandwidth.register` fixes every service bandwidth variable on an edge whose source is not normal to zero.

$$
\forall e \in E^{client}\;\forall s \in S\;\bigl(y_{e,s}=0\bigr)
$$

### 2. Bandwidth Is Nonnegative and Edge-Bounded

**Description**: Every normal-source edge variable is an unsigned integer bounded by that edge's maximum bandwidth.

$$
\forall e \in E^{normal}\;\forall s \in S\;\bigl(y_{e,s}\in\mathbb{Z}_{\ge 0}\ \wedge\ y_{e,s}\le maxBandwidth_e\bigr)
$$

### 3. Net Out-Flow Is a Definition, Not Conservation

**Description**: The implementation defines $F_{n,s}=O_{n,s}-I_{n,s}$ and $F_n=\sum_sF_{n,s}$, but no pipeline registers $O_{n,s}=I_{n,s}$ or another flow-conservation equality.

$$
\forall n \in N^{normal}\;\forall s \in S\;\bigl(F_{n,s}=O_{n,s}-I_{n,s}\bigr)
$$

---

## 8. Constraints

> The active constraints are exactly the pipelines returned by `bandwidth_context/service/PipelineListGenerator.kt`. A class existing in `service/limits` is not active unless that generator returns it.

### 1. Edge Bandwidth Constraint

**Edge Bandwidth Constraint [边带宽约束]**
**Description**: The service-level assignment count gates bandwidth on every normal-source edge. If a service is not assigned to any normal node, its bandwidth on each such edge is zero; if it is assigned once, the edge range still caps the bandwidth at the edge maximum.

$$
s.t.\quad (1-A_s)\,maxBandwidth_e+y_{e,s}\le maxBandwidth_e,
\qquad \forall e \in E^{normal},\ \forall s \in S
$$

**Corollary**: Since the Route Context constraint gives $A_s \le 1$, the inequality is equivalent to $y_{e,s}\le maxBandwidth_e A_s$ on the active domain.

$$
(1-A_s)\,maxBandwidth_e+y_{e,s}\le maxBandwidth_e
\ \Longleftrightarrow\ y_{e,s}\le maxBandwidth_e A_s
$$

### 2. Demand Constraint

**Demand Constraint [需求约束]**
**Description**: Every client node receives at least its declared demand through incoming service bandwidth.

$$
s.t.\quad I_n\ge demand_n,\qquad \forall n \in N^{client}
$$

### 3. Service Capacity Constraint

**Service Capacity Constraint [服务容量约束]**
**Description**: At a normal node, service net out-flow is gated by the imported node-service assignment variable. The code permits negative net out-flow; it registers only this upper-bound inequality.

$$
s.t.\quad (1-x_{n,s})\,capacity_s+F_{n,s}\le capacity_s,
\qquad \forall n \in N^{normal},\ \forall s \in S
$$

**Corollary**: The active inequality is algebraically equivalent to $F_{n,s}\le capacity_s x_{n,s}$.

$$
(1-x_{n,s})\,capacity_s+F_{n,s}\le capacity_s
\ \Longleftrightarrow\ F_{n,s}\le capacity_s x_{n,s}
$$

### 4. Transfer Node Bandwidth Constraint (Not Registered)

**Transfer Node Bandwidth Constraint [传输节点带宽约束]**
**Description**: `TransferNodeBandwidthConstraint` exists and would gate a normal node's aggregate net out-flow by the sum of its outgoing edge maxima. However, `PipelineListGenerator` returns only `EdgeBandwidthConstraint`, `DemandConstraint`, `ServiceCapacityConstraint`, and `BandwidthCostObjective`; it does not return this class. The following inequality is therefore not active in the current Demo1 model.

$$
s.t.\quad (1-A_n)\,M_n+F_n\le M_n,
\qquad \forall n \in N^{normal}
$$

**Corollary**: This candidate is a one-sided out-flow gate only. It does not imply $O_n=I_n$; no flow-conservation equality is registered anywhere in the current bandwidth pipeline list.

---

## 9. Objective Function (if applicable)

**Description**: `BandwidthCostObjective` minimizes the total cost of bandwidth on every edge whose source is a normal node, including the generated normal-to-client edges. The Route Context separately registers the service-cost objective.

$$
\min\quad Z_{bandwidth}=\sum_{e \in E^{normal}} costPerBandwidth_e\,B_e
$$

---

## 10. Algorithm References

The only standalone algorithm-like operation used by this context after solving is DFS path extraction in `SolutionAnalyzer.kt`; it is not a model constraint or objective.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| DFS path extraction | [`service/SolutionAnalyzer.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/bandwidth_context/service/SolutionAnalyzer.kt) | `BandwidthContext.analyze` | Reads positive assignment and bandwidth tokens, then traces non-repeating edge links from assigned nodes to client nodes. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Bandwidth variable | $y_{e,s}$ | Integer bandwidth allocated to service $s$ on edge $e$. |
| Edge total bandwidth | $B_e$ | Sum of all service bandwidths on edge $e$, or zero for a non-normal-source edge. |
| Service in-degree | $I_{n,s}$ | Bandwidth of service $s$ entering node $n$. |
| Service out-degree | $O_{n,s}$ | Bandwidth of service $s$ leaving normal node $n$. |
| Service net out-flow | $F_{n,s}$ | Service out-degree minus service in-degree at a normal node. |
| Node in-degree | $I_n$ | Incoming bandwidth aggregated across services. |
| Node net out-flow | $F_n$ | Net out-flow aggregated across services at a normal node. |
| Service assignment count | $A_s$ | Imported number of normal nodes assigned to service $s$. |
| Normal-source edge | $E^{normal}$ | Edge whose source satisfies the `normal` predicate. |
| Candidate node capacity | $M_n$ | Sum of maximum outgoing edge bandwidths, used only by the unregistered transfer-node constraint. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Declare $y$ as an unsigned integer array and bound it per edge | Use a real-valued or globally bounded array | Matches `UIntVariable2` and the per-edge `maxBandwidth` range assignments. | Current source |
| Fix variables on non-normal-source edges to zero | Gate every edge only through a later constraint | This is an explicit registration range in `EdgeBandwidth.register`. | Current source |
| Define in-degree for all nodes but out-degree/out-flow as zero for clients | Define all three only on normal nodes | Matches the `flatMap` branches in `ServiceBandwidth` and `NodeBandwidth`. | Current source |
| Keep transfer-node gating as a separate class but omit it from the pipeline generator | Register it automatically with other bandwidth constraints | The current pipeline generator omits the class, so its inequality is not active. | Current source |
| Do not add flow conservation implicitly from net out-flow definitions | Register an equality such as $O_n=I_n$ | The source defines differences only; no conservation pipeline is returned. | Current source |

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| Current | Rewritten as a 13-section, source-aligned domain model | Replace invented transfer formulas and incorrect assignment gates with the exact registered variables, intermediates, constraints, objective, and inactive-class status. |
