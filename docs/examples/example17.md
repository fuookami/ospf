# Example 17: Vehicle Routing Problem

## Problem Description

The Vehicle Routing Problem (VRP) is a classic operations research optimization problem that has been extensively studied in discrete combinatorial optimization. It has strong application value in the logistics industry, as optimizing vehicle routes can effectively reduce logistics distribution costs. The vehicle routing problem was first proposed by Dantzig and Ramser in 1959. The problem involves a certain number ($n$) of customers, each with different quantities of goods demand ($q_i$). A distribution center or depot supplies goods to customers, and a fleet ($K$ vehicles) is responsible for distributing the goods. The goal is to organize appropriate driving routes to minimize total travel cost and time consumption while satisfying customer demands and certain constraints (such as vehicle load capacity limits $Q_k$ and mileage length limits $L$).

For the VRPTW (Vehicle Routing Problem with Time Windows) problem, the basic definition is first given: Given a graph $G(V, A)$, where $V = \{ 0, 1, 2, \dots, n + 1 \}$ is the set of all nodes in the graph. For modeling convenience, virtual distribution centers are $0$ and $n + 1$, representing the starting point and ending point respectively. $A$ is the set of all arcs in the graph, $(i, j) \in A$, and $\forall i, j \in V, \; i \neq j$. The unit transportation cost for arc $(i, j)$ is $c_{ij}$, the unit transportation time is $t_{ij}$, the material demand at node $i$ is $q_{i}$, the service time window at node $i$ is $[e_{i}, l_{i}]$, and the required service duration at node $i$ is $h_i$. The vehicle set at the distribution center (depot) is $K$, the load capacity of each vehicle $k \in K$ is $Q_k$, and the fixed usage cost for each vehicle is $F_k$.

## Mathematical Model

### Sets

$V$: Node set.

$O$: Starting point set.

$E$: Ending point set.

$K$: Vehicle set at the distribution center (depot).

### Parameters

$c_{ij}$: Transportation cost for arc $(i, j)$.

$t_{ij}$: Transportation time for arc $(i, j)$, positively correlated with $c_{ij}$.

$h_{i}$: Required service duration at node $i$.

$q_{i}$: Material demand quantity at node $i$.

$[e_{i}, \; l_{i}]$: Service time window at node $i$.

$Q_{k}$: Load capacity of each vehicle $k \in K$.

$F_{k}$: Fixed usage cost of each vehicle.

### Variables

$x_{ijk}$: Whether vehicle $k$ traverses arc $(i, j)$, i.e., whether vehicle $k$ serves node $i$ and node $j$.

$s_{ik}$: Start service time of vehicle $k$ at node $i$.

### Intermediate Values

#### 1. Departure from Distribution Center (Implies Vehicle Usage)

$$
\text{Origin}_{k} = \sum_{i \in O} \sum_{j \in V} x_{ijk}, \; \forall k \in K
$$

#### 2. Return to Distribution Center

$$
\text{Destination}_{k} = \sum_{i \in V} \sum_{j \in E} x_{ijk}, \; \forall k \in K
$$

#### 3. Vehicle Inflow to Node

$$
\text{In}_{jk} = \sum_{i \in V - E}  x_{ijk}, \; \forall j \in (V - (O \cup E)), \; \forall k \in K
$$

#### 4. Vehicle Outflow from Node

$$
\text{Out}_{ik} = \sum_{j \in V - O} x_{ijk}, \; \forall i \in (V - (O \cup E)), \; \forall k \in K
$$

#### 5. Service Received by Node

$$
\text{Service}_{i} = \sum_{k \in K} \sum_{j \in V - O} x_{ijk}, \; \forall i \in (V - (O \cup E))
$$

#### 6. Vehicle Load Capacity Constraint

$$
\text{Capacity}_{k} = \sum_{i \in V} \sum_{j \in V} q_{j} \cdot x_{ijk}, \; \forall k \in K
$$

### Objective Function

#### 1. Minimize Vehicle Usage Cost

$$
\min \quad \sum_{k \in K} \text{Origin}_{k} \cdot F_k
$$

#### 2. Minimize Transportation Cost

$$
\min \quad \sum_{k \in K} \sum_{i \in V} \sum_{j \in V} c_{ij} \cdot x_{ijk}
$$

### Constraints

#### 1. Vehicle Departing from Distribution Center Can Only Reach One Node (Vehicle Must Depart from Distribution Center or Not Be Used)

$$
\text{s.t.} \quad \text{Origin}_{k} \leq 1, \; \forall k \in K
$$

#### 2. Flow Balance Constraint: If a Vehicle Arrives at a Node, It Must Depart from That Node

$$
\text{s.t.} \quad \text{In}_{ik} - \text{Out}_{ik} = 0, \; \forall k \in K, \; \forall i \in (V - (O \cup E))
$$

#### 3. If a Vehicle Is Used, It Must Return to the Distribution Center

$$
\text{s.t.} \quad \text{Destination}_{k} \leq 1, \; \forall k \in K
$$

#### 4. Each Node Must Receive Service

$$
\text{s.t.} \quad \text{Service}_{i} = 1, \; \forall i \in (V - (O \cup E))
$$

#### 5. Arrival Time Relationship Between Visits to Two Nodes (This Constraint Also Serves to Eliminate Cycles, Removing Infeasible Cycles Generated During Single Vehicle Travel)

$$
\text{s.t.} \quad s_{ik} + h_{i} + t_{ij} - M(1 - x_{ijk}) \leq s_{jk}, \; \forall k \in K, \; \forall i \in V, \; \forall j \in V
$$

#### 6. Node Service Time Window Constraint

$$
\text{s.t.} \quad e_{i} \leq s_{ik} \leq l_{i}, \; \forall i \in V
$$

#### 7. Vehicle Load Capacity Constraint That Cannot Be Violated

$$
\text{s.t.} \quad \text{Capacity}_{k} \leq Q_{k}, \; \forall k \in K
$$

## Code Implementation

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

sealed interface Node : Indexed {
    val demand: UInt64 get() = UInt64.zero
    val position: Point2
    val timeWindow: ValueRange<UInt64>

    fun distance(other: Node): Flt64 {
        return position.distance(other.position)
    }

    fun cost(other: Node): Flt64 {
        return distance(other)
    }

    fun time(other: Node): Flt64 {
        return distance(other)
    }
}

data class OriginNode(
    override val position: Point2,
    override val timeWindow: ValueRange<UInt64>
) : Node, AutoIndexed(Node::class)

data class EndNode(
    override val position: Point2,
    override val timeWindow: ValueRange<UInt64>
) : Node, AutoIndexed(Node::class)

data class DemandNode(
    override val position: Point2,
    override val timeWindow: ValueRange<UInt64>,
    override val demand: UInt64,
    val serviceTime: UInt64
) : Node, AutoIndexed(Node::class)

data class Vehicle(
    val capacity: UInt64,
    val fixedUsedCost: UInt64,
) : AutoIndexed(Vehicle::class)

val nodes: List<Node> = ... // Node list
val vehicles: List<Vehicle> = ... // Vehicle list

// Create model instance
val metaModel = LinearMetaModel("demo17")

// Define variables
val x = BinVariable3(
    "x",
    Shape3(nodes.size, nodes.size, vehicles.size)
)
for (n1 in nodes) {
    for (n2 in nodes) {
        for (v in vehicles) {
            val xi = x[n1, n2, v]
            if (n1 !is EndNode && n2 !is OriginNode && n1 != n2) {
                xi.name = "${x.name}_(${n1.index},${n2.index},${v.index})"
                metaModel.add(xi)
            } else {
                xi.range.eq(false)
            }
        }
    }
}

val s = URealVariable2(
    "s",
    Shape2(nodes.size, vehicles.size)
)
for (n in nodes) {
    for (v in vehicles) {
        val si = s[n, v]
        si.name = "${s.name}_(${n.index}_${v.index})"
    }
}
metaModel.add(s)

// Define intermediate values
val origin = LinearIntermediateSymbols1(
    "origin",
    Shape1(vehicles.size)
) { i, _ ->
    val v = vehicles[i]
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<OriginNode>().flatMap { n1 -> x[n1, _a, v] }), 
        "origin_${v.index}"
    )
}
metaModel.add(origin)

val destination = LinearIntermediateSymbols1(
    "destination",
    Shape1(vehicles.size)
) { i, _ ->
    val v = vehicles[i]
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<EndNode>().flatMap { n2 -> x[_a, n2, v] }), 
        "destination_${v.index}"
    )
}
metaModel.add(destination)

val inFlow = LinearIntermediateSymbols2(
    "in",
    Shape2(nodes.size, vehicles.size)
) { _, vec ->
    val n2 = nodes[vec[0]]
    val v = vehicles[vec[1]]
    if (n2 is OriginNode) {
        LinearExpressionSymbol(
            LinearPolynomial(), 
            "in_(${n2.index},${v.index})"
        )
    } else {
        LinearExpressionSymbol(
            sum(nodes.filterIsNotInstance<EndNode, Node>().map { n1 -> x[n1, n2, v] }), 
            "in_(${n2.index},${v.index})"
        )
    }
}
metaModel.add(inFlow)

val outFlow = LinearIntermediateSymbols2(
    "out",
    Shape2(nodes.size, vehicles.size)
) { _, vec ->
    val n1 = nodes[vec[0]]
    val v = vehicles[vec[1]]
    if (n1 is EndNode) {
        LinearExpressionSymbol(
            LinearPolynomial(), 
            "out_(${n1.index},${v.index})"
        )
    } else {
        LinearExpressionSymbol(
            sum(nodes.filterIsNotInstance<OriginNode, Node>().map { n2 -> x[n1, n2, v] }), 
            "out_(${n1.index},${v.index})"
        )
    }
}
metaModel.add(outFlow)

val service = LinearIntermediateSymbols1(
    "service",
    Shape1(nodes.size)
) { i, _ ->
    val n1 = nodes[i]
    if (n1 is OriginNode || n1 is EndNode) {
        LinearExpressionSymbol(
            LinearPolynomial(), 
            "service_(${n1.index})"
        )
    } else {
        LinearExpressionSymbol(
            sum(nodes.filterIsNotInstance<OriginNode, Node>().flatMap { n2 -> x[n1, n2, _a] }), 
            "service_(${n1.index})"
        )
    }
}
metaModel.add(service)

val capacity = LinearIntermediateSymbols1(
    "capacity",
    Shape1(vehicles.size)
) { i, _ ->
    val v = vehicles[i]
    LinearExpressionSymbol(
        sum(nodes.flatMap { n1 ->
            nodes.mapNotNull { n2 ->
                (n2 as? DemandNode)?.demand?.let { it * x[n1, n2, v] }
            }
        }),
        "capacity_${v.index}"
    )
}
metaModel.add(capacity)

// Define objective function
metaModel.minimize(
    sum(vehicles.map { v -> v.fixedUsedCost * origin[v] }),
    "used cost"
)

metaModel.minimize(
    sum(nodes.flatMap { n1 ->
        nodes.map { n2 ->
            n1.cost(n2) * sum(x[n1, n2, _a])
        }
    }),
    "trans cost"
)

// Define constraints
for (v in vehicles) {
    metaModel.addConstraint(
        origin[v] leq 1,
        "origin_${v.index}"
    )
}

for (n in nodes.filterIsInstance<DemandNode>()) {
    for (v in vehicles) {
        metaModel.addConstraint(
            inFlow[n, v] eq outFlow[n, v],
            "balance_${n.index}_${v.index}",
        )
    }
}

for (v in vehicles) {
    metaModel.addConstraint(
        destination[v] leq 1,
        "destination_${v.index}"
    )
}

for (n in nodes.filterIsInstance<DemandNode>()) {
    metaModel.addConstraint(
        service[n] eq 1,
        "service_${n.index}"
    )
}

val m = nodes.filterIsInstance<EndNode>().maxOf { it.timeWindow.upperBound.value.unwrap() }
for (n1 in nodes) {
    for (n2 in nodes) {
        for (v in vehicles) {
            metaModel.addConstraint(
                s[n1, v] + ((n1 as? DemandNode)?.serviceTime ?: UInt64.zero) + n1.time(n2) - m * (1 - x[n1, n2, v]) leq s[n2, v],
                "time_window_${n1.index}_${n2.index}_${v.index}"
            )
        }
    }
}

for (n in nodes) {
    for (v in vehicles) {
        metaModel.addConstraint(
            s[n, v] geq n.timeWindow.lowerBound.value.unwrap(),
            "time_window_lb_${n.index}_${v.index}"
        )
        metaModel.addConstraint(
            s[n, v] leq n.timeWindow.upperBound.value.unwrap(),
            "time_window_ub_${n.index}_${v.index}"
        )
    }
}

for (v in vehicles) {
    metaModel.addConstraint(
        capacity[v] leq v.capacity,
        "capacity_${v.index}"
    )
}

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {
        return Failed(ret.error)
    }
}

// Parse results
val route: MutableMap<Vehicle, MutableList<Pair<Node, Node>>> = HashMap()
val time: MutableMap<Vehicle, MutableMap<Node, UInt64>> = HashMap()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo x) {
        val vector = token.variable.vectorView
        val n1 = nodes[vector[0]]
        val n2 = nodes[vector[1]]
        val v = vehicles[vector[2]]
        route.getOrPut(v) { ArrayList() }.add(n1 to n2)
    }
}
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable belongsTo s) {
        val vector = token.variable.vectorView
        val n = nodes[vector[0]]
        val v = vehicles[vector[1]]
        time.getOrPut(v) { HashMap() }[n] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo17.kt)
