# Example 17: Vehicle routing with time windows

## Problem and data

This demo models a capacitated vehicle-routing problem with service time windows. The current source contains one origin node, 100 demand nodes, one end node, and 25 identical vehicles. The origin and end are both at $(40,50)$ with time window $[0,1236]$. Each vehicle has capacity 200 and fixed-used cost 500. Every demand node has a positive integer demand, a source-provided time window, and service duration 90.

The source uses Euclidean geometry: Node.distance is the distance between the two Point2 positions, and both Node.cost and Node.time return that distance. Thus travel cost and travel time share the same distance unit; there is no independent cost or speed matrix.

## Sets and parameters

Let $N$ be all 102 nodes, $O=\{o\}$ the origin, $E=\{e\}$ the end, $D=N\setminus(O\cup E)$ the 100 demand nodes, and $K$ the 25-vehicle set. Let

$$
A=\{(i,j)\in N^2:i\notin E,\ j\notin O,\ i\ne j\}.
$$

be the allowed arc set implemented by the source. For demand node $j$, $q_j$ is its integer demand and $h_j=90$ is its service duration; $q_o=q_e=0$ and the origin/end service duration is zero. Each node has source data $(position_i,[e_i,l_i])$. For vehicle $k$, $Q_k=200$ and $F_k=500$.

## Decision variables

For $(i,j)\in A$ and $k\in K$:

$$
x_{ijk}\in\{0,1\}
$$

indicates that vehicle k uses arc $i\to j$. The source creates BinVariable3 over all node pairs and vehicles, fixes disallowed entries to false, and registers only the allowed entries.

For every $i\in N,k\in K$, $s_{ik}\in\mathbb R_{\ge0}$ is the service-start time, implemented by URealVariable2. Every $s_{ik}$ is bounded again by the node's time-window constraints.

## Intermediate values

For each vehicle $k$ and demand node $d$:

$$
Origin_k=\sum_{j:(o,j)\in A}x_{ojk},\qquad
Destination_k=\sum_{i:(i,e)\in A}x_{iek},
$$
$$
In_{dk}=\sum_{i:(i,d)\in A}x_{idk},\qquad
Out_{dk}=\sum_{j:(d,j)\in A}x_{djk}.
$$

For each demand node $d$:

$$
Service_d=\sum_{k\in K}\sum_{j:(d,j)\in A}x_{djk}.
$$

For each vehicle:

$$
Capacity_k=\sum_{i\in N}\sum_{j\in N}q_jx_{ijk}.
$$

The source registers two objectives:

$$
UsedCost=\sum_{k\in K}F_kOrigin_k,\qquad
TravelCost=\sum_{k\in K}\sum_{(i,j)\in A}distance_{ij}x_{ijk}.
$$

## Objective

Demo17 calls minimize twice, first for UsedCost and then for TravelCost. The source does not construct a weighted sum or document a scalar coefficient between them; the exact multi-objective handling is left to the current model/solver policy.

## Constraints and domain

Vehicle use and route flow:

$$
Origin_k\le1,\qquad Destination_k\le1\quad(\forall k\in K),
$$
$$
In_{dk}=Out_{dk}\quad(\forall d\in D,\ k\in K),
$$
$$
Service_d=1\quad(\forall d\in D).
$$

The source implements the equality $In=Out$ as both greater-than-or-equal and less-than-or-equal constraints. For each $i,j\in N$ and $k\in K$, it adds the time implication with the source's big-M:

$$
s_{ik}+h_i+distance_{ij}-M(1-x_{ijk})\le s_{jk},
\qquad M=1236.
$$

For every node and vehicle:

$$
e_i\le s_{ik}\le l_i\quad(\forall i\in N,\ k\in K),
$$

and for every vehicle:

$$
Capacity_k\le Q_k=200.
$$

Disallowed arcs are fixed to zero before these expressions are registered. The source still creates time constraints over all node pairs, exactly as shown; the fixed-zero entries make those implications inactive for disallowed arcs.

## Implementation differences and notes

The source builds the model through initVariable, initSymbol, initObject, initConstraint, solve, and analyzeSolution. It uses current core symbols and solves with ScipLinearSolver configured with a 300-second time limit. The two objective registrations, the 1236 big-M value, the allowed-arc filtering, and the nonnegative real time variables are implementation facts. This core demo is not a generic VRPTW formulation with optional customers: every demand node is required exactly once.

## Expected result

A successful solve returns routes and service times for all 100 demand nodes, respecting each source time window and each vehicle's capacity. The current build test verifies model construction only; it does not assert a route list, objective values, or a unique optimum. Running this instance can be expensive and is subject to the five-minute solver limit.

## Minimal current Kotlin example

~~~kotlin
import kotlin.time.Duration.Companion.seconds
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.algebra.value_range.*
import fuookami.ospf.kotlin.math.geometry.*
import fuookami.ospf.kotlin.math.geometry.point2
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.math.symbol.polynomial.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.config.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

val model = LinearMetaModel<Flt64>("demo17", converter = flt64Converter)
val x = BinVariable3("x", Shape3(nodes.size, nodes.size, vehicles.size))
for (from in nodes) for (to in nodes) for (vehicle in vehicles) {
    val xi = x[from, to, vehicle]
    if (from !is EndNode && to !is OriginNode && from != to) model.add(xi)
    else xi.range.eq(false)
}
val s = URealVariable2("s", Shape2(nodes.size, vehicles.size))
model.add(s)
val origin = LinearIntermediateSymbols1<Flt64>("origin", Shape1(vehicles.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<OriginNode>().flatMap { node -> x[node, _a, vehicles[i]] }),
        name = "origin_$i"
    )
}
val destination = LinearIntermediateSymbols1<Flt64>("destination", Shape1(vehicles.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.filterIsInstance<EndNode>().flatMap { node -> x[_a, node, vehicles[i]] }),
        name = "destination_$i"
    )
}
val service = LinearIntermediateSymbols1<Flt64>("service", Shape1(nodes.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.filterIsNotInstance<OriginNode, Node>().flatMap { node -> x[nodes[i], node, _a] }),
        name = "service_$i"
    )
}
val capacity = LinearIntermediateSymbols1<Flt64>("capacity", Shape1(vehicles.size)) { i, _ ->
    LinearExpressionSymbol(
        sum(nodes.flatMap { from ->
            nodes.mapNotNull { to -> (to as? DemandNode)?.demand?.let { it * x[from, to, vehicles[i]] } }
        }),
        name = "capacity_$i"
    )
}
model.add(origin)
model.add(destination)
model.add(service)
model.add(capacity)
model.minimize(sum(vehicles.map { it.fixedUsedCost * origin[it] }), "used cost")
model.minimize(
    sum(nodes.flatMap { from -> nodes.map { to -> from.cost(to) * sum(x[from, to, _a]) } }),
    "trans cost"
)
for (vehicle in vehicles) model.addConstraint(origin[vehicle] leq 1)
for (node in nodes.filterIsInstance<DemandNode>()) {
    model.addConstraint(service[node] eq 1)
    for (vehicle in vehicles) {
        model.addConstraint(inFlow[node, vehicle] geq outFlow[node, vehicle])
        model.addConstraint(inFlow[node, vehicle] leq outFlow[node, vehicle])
    }
}
for (vehicle in vehicles) {
    model.addConstraint(destination[vehicle] leq 1)
    model.addConstraint(capacity[vehicle] leq vehicle.capacity)
}
val m = nodes.filterIsInstance<EndNode>().maxOf { it.timeWindow.upperBound.value.unwrap() }
for (from in nodes) for (to in nodes) for (vehicle in vehicles) {
    model.addConstraint(
        s[from, vehicle] +
            ((from as? DemandNode)?.serviceTime ?: UInt64.zero).toFlt64() +
            from.time(to) -
            m.toFlt64() * (1 - x[from, to, vehicle]) leq s[to, vehicle]
    )
}
for (node in nodes) for (vehicle in vehicles) {
    model.addConstraint(s[node, vehicle] geq node.timeWindow.lowerBound.value.unwrap())
    model.addConstraint(s[node, vehicle] leq node.timeWindow.upperBound.value.unwrap())
}

suspend fun solve() = solveLinearMetaModel(
    ScipLinearSolver(config = SolverConfig(time = 300.seconds)),
    model
)
~~~

## Source and verification

### Kotlin/Rust correspondence

- [Rust counterpart: `src/core/demo17.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo17.rs)

The Rust file is an independent compact VRPTW sample: 4 customers, 3 vehicles (capacity 25, fixed cost 100), Big-M 500, and one combined cost objective. The current Kotlin implementation uses 100 customers/102 nodes, 25 vehicles (capacity 200, fixed cost 500), Big-M 1236, and separate used/travel objective registrations; do not share the data table or claim model-instance equivalence.

- [Current implementation: Demo17.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo17.kt)
- [Core build-structure test: CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

::: code-group

```kotlin [Kotlin]
// See the linked Kotlin implementation for the complete model.
`` 

```rust [Rust]
// See the linked Rust implementation for the equivalent model.
`` 

:::

