# Route Context Domain Model

> English | [中文](../../../zh-cn/examples/framework-example1/domain-route/domain-model)

[toc]

## 1. Overview

The Route Context builds the `Graph`, generated `Service` list, and service-to-node assignment aggregate from the demo input; it supplies the route variables and route-side pipelines consumed by the Bandwidth Context. The authoritative implementation is the Kotlin `demo1` source, especially [`RouteContext.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/route_context/RouteContext.kt), [`Assignment.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/route_context/model/Assignment.kt), and [`PipelineListGenerator.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1/route_context/service/PipelineListGenerator.kt).

### 1. Dependent Contexts

None. The Bandwidth Context depends on this context's graph, services, assignment variables, and assignment intermediate symbols.

---

## 2. Concepts / Entities

### 1. Node

A graph vertex. A node is either a `NormalNode`, which can host a service, or a `ClientNode`, which has a bandwidth demand. The Kotlin object identity distinguishes graph nodes; the input client identifier is not assumed to be globally unique with normal-node identifiers.

**$id_{n}$** : The input identifier stored on node $n$, an unsigned integer used as node data.
**$edges_{n}$** : The mutable list of outgoing graph edges attached to node $n$ by `RouteContext.init`.

#### 1.1 Normal Node

A transit/hosting node created for each index from $0$ through `normalNodeAmount - 1`. Only normal nodes have free route-assignment rows and outgoing bandwidth variables with nonzero registration bounds.

#### 1.2 Client Node

A terminal node created from a `ClientNodeDTO`; it consumes bandwidth and has no free service-assignment row.

**$demand_{n}$** : The required incoming bandwidth of client node $n$, an unsigned integer, defined only for $n \in N^{client}$.

### 2. Edge

A directed graph edge. For each input `EdgeDTO`, initialization creates one edge in each direction with the same maximum bandwidth and unit cost. For each client, initialization adds one edge from its linked normal node to the client, with maximum bandwidth equal to the client demand and zero unit cost.

**$from(e)$** : The source node of directed edge $e$.
**$to(e)$** : The target node of directed edge $e$.
**$maxBandwidth_{e}$** : The edge's maximum bandwidth, an unsigned integer.
**$costPerBandwidth_{e}$** : The cost per unit of bandwidth on edge $e$, an unsigned integer.

### 3. Service

A candidate service that may be assigned to one normal node and whose bandwidth is allocated by the dependent Bandwidth Context.

**$id_{s}$** : The generated service identifier, an unsigned integer.
**$capacity_{s}$** : The service bandwidth capacity, an unsigned integer. In `RouteContext.init`, every generated service receives the total client demand.
**$cost_{s}$** : The service-use cost, an unsigned integer. In the demo input, every generated service receives `input.serviceCost`.

### 4. Graph

The route aggregate's network container, holding the node and directed-edge lists.

**$nodes$** : The ordered list of all normal and client node objects.
**$edges$** : The ordered list of all generated directed edge objects.

---

## 3. Variables

### 1. Decision Variables

**$x_{n,s}$** : Binary service-assignment variable, dimensionless, domain is $\{0,1\}$, where $1$ means that service $s$ is hosted at normal node $n$ and $0$ means it is not, $\forall n \in N,\ \forall s \in S$. Rows with $n \in N^{client}$ are fixed to $0$ by `Assignment.register`; the unknown decision rows are therefore $\forall n \in N^{normal},\ \forall s \in S$.

### 2. Auxiliary Variables

No separate auxiliary decision variables are declared. The registered assignment counts are intermediate symbols defined in Section 6.

---

## 4. Predicates

### 1. Node Type

> Predicates classify entity sets; each predicate defines a subset.

**$normal(n)$** : Node $n$ is an instance of `NormalNode`.
**$client(n)$** : Node $n$ is an instance of `ClientNode`.

### 2. Edge Incidence

**$from(e)=n$** : Edge $e$ has source node $n$; the implementation also exposes predicates matching a specified node or a node predicate.
**$to(e)=n$** : Edge $e$ has target node $n$.

---

## 5. Sets

### 1. Nodes

**$N$** : The universal set of graph node objects in `Graph.nodes`.

**$N^{normal}$** : The subset satisfying $normal$, the normal nodes that may host services.
**$N^{client}$** : The subset satisfying $client$, the client nodes whose demands must be received by the bandwidth model.

### 2. Edges

**$E$** : The universal set of directed graph edges in `Graph.edges`, including both generated directions for each input edge and the generated normal-to-client edges.

**$E^{normal}$** : The subset of edges satisfying $from(e) \in N^{normal}$, the edges eligible for nonzero bandwidth variables in the Bandwidth Context.
**$E^{client}$** : The subset $E \setminus E^{normal}$, whose bandwidth variables are fixed to zero by the Bandwidth Context.

### 3. Services

**$S$** : The universal set of generated candidate services. If $r$ is the input `normalNodeAmount`, then `RouteContext.init` creates $\lfloor r/2 \rfloor$ services.

### 4. Entity Pairs / Relations

The implementation uses the edge incidence relations $from(e)$ and $to(e)$ above; it registers no separate route relation or path set.

---

## 6. Intermediate Values

### 1. Node Assignment Count

**Description**: The number of services assigned to a node. `Assignment.register` sums the binary rows for normal nodes and installs the zero polynomial for client-node rows.

$$
A_n = \begin{cases}
\displaystyle\sum_{s \in S} x_{n,s},& n \in N^{normal}\\
0,& n \in N^{client}
\end{cases}
\qquad \forall n \in N
$$

### 2. Service Assignment Count

**Description**: The number of normal nodes assigned to service $s$. This is the symbol used by the service-assignment constraint and imported by the bandwidth edge gate.

$$
A_s = \sum_{n \in N^{normal}} x_{n,s},\qquad \forall s \in S
$$

### 3. Generated Service Capacity and Count

**Description**: These are input-derived entity attributes rather than model symbols. Let $D$ be the total client demand and let $r$ be `normalNodeAmount`.

$$
D = \sum_{n \in N^{client}} demand_n,\qquad |S| = \left\lfloor\frac{r}{2}\right\rfloor,\qquad capacity_s = D\quad \forall s \in S
$$

The generated service cost is $cost_s = input.serviceCost$ for every $s \in S$.

---

## 7. Assertions

> Assertions are properties that always hold in the registered route aggregate and its data shape.

### 1. Client Assignment Rows Are Fixed

**Description**: `Assignment.register` fixes every client-node row of the binary variable array to false.

$$
\forall n \in N^{client}\;\forall s \in S\;\bigl(x_{n,s}=0\bigr)
$$

### 2. Client Assignment Count Is Zero

**Description**: Because client rows are represented by zero polynomials, the node assignment intermediate for every client is zero.

$$
\forall n \in N^{client}\;\bigl(A_n=0\bigr)
$$

### 3. Assignment Counts Are Bounded by Their Registered Binary Rows

**Description**: Before the route pipelines are applied, the sums are nonnegative and integral; the active constraints in Section 8 impose the upper bounds $A_n \le 1$ and $A_s \le 1$. No equality requiring every service to be assigned is registered.

$$
\forall n \in N^{normal}\;\forall s \in S\;\bigl(x_{n,s}\in\{0,1\}\bigr)
$$

---

## 8. Constraints

> Constraints are the route pipelines returned by `route_context/service/PipelineListGenerator.kt`; the assignment variable ranges are registered by `Assignment.register` before those pipelines run.

### 1. Node Assignment Constraint

**Node Assignment Constraint [节点分配约束]**
**Description**: A normal node hosts at most one service.

$$
s.t.\quad A_n \le 1,\qquad \forall n \in N^{normal}
$$

### 2. Service Assignment Constraint

**Service Assignment Constraint [服务分配约束]**
**Description**: A service is assigned to at most one normal node. It may remain unassigned in this context; client demand and edge gating are enforced by the dependent bandwidth pipelines.

$$
s.t.\quad A_s \le 1,\qquad \forall s \in S
$$

---

## 9. Objective Function (if applicable)

**Description**: `ServiceCostObjective` minimizes the sum of each generated service's cost when that service is assigned. This context registers this objective in addition to the bandwidth objective registered by the dependent context; the source does not state a weighted or lexicographic combination.

$$
\min\quad Z_{route}=\sum_{s \in S} cost_s A_s
$$

---

## 10. Algorithm References

No standalone algorithm document is referenced by the Route Context domain model. Route initialization and pipeline construction are ordinary context operations in `RouteContext.kt` and `PipelineListGenerator.kt`.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| None | — | — | No standalone route algorithm is registered. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Node | $n$ | A graph node object. |
| Normal node | $N^{normal}$ | A transit/hosting node eligible for service assignment. |
| Client node | $N^{client}$ | A terminal node with incoming demand. |
| Edge | $e$ | A directed graph edge. |
| Service | $s$ | A generated candidate service. |
| Assignment | $x_{n,s}$ | Binary decision that places a service at a normal node. |
| Node assignment count | $A_n$ | Number of services assigned to node $n$. |
| Service assignment count | $A_s$ | Number of normal nodes assigned to service $s$. |
| Demand | $demand_n$ | Required incoming bandwidth of client node $n$. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Materialize two directed edges for each input edge | Keep the input edge undirected | The source constructs and indexes both directions explicitly. | Current source |
| Add one normal-to-client edge with demand capacity and zero cost | Represent demand only as a node equation | The current graph lets incoming bandwidth satisfy each client through an explicit edge. | Current source |
| Generate $\lfloor r/2\rfloor$ services with capacity equal to total client demand | Read service count and capacities from separate input fields | This is the exact `RouteContext.init` demo rule. | Current source |
| Fix client rows of $x$ to zero | Leave all node rows free | Client nodes are consumers, not service hosts, in the implemented model. | Current source |

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| Current | Rewritten as a 13-section, source-aligned domain model | Replace shorthand and unsupported route formulas with the actual Kotlin registration semantics. |
