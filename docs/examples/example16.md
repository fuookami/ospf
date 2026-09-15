# Example 16: Production and inventory across months

## Problem and data

The four periods are March, April, May, and June. Their production capacities and demands are:

| Month | March | April | May | June |
| :---: | ---: | ---: | ---: | ---: |
| Productivity | 50 | 180 | 280 | 270 |
| Demand | 100 | 200 | 180 | 300 |

The source parameters are production price $C^p=40$, delay-delivery price $C^d=2$, and storage price $C^s=0.5$. The model has total capacity 780 and total demand 780, but the constraints themselves remain inequalities.

## Sets and parameters

Let $M$ be the ordered period set. $Productivity_i$ and $Demand_i$ are the source fields on `Produce`; $i<j$ means an earlier period $i$ and a later period $j$.

## Decision variable

 $x_{ij}\in\mathbb{Z}_{\ge0}$ is the quantity produced in period $i$ and assigned to satisfy demand in period $j$, for every $(i,j)\in M\times M$. The current source uses UIntVariable2 and does not fix off-diagonal directions.

## Intermediate values

$$
Produce_i=\sum_{j\in M}x_{ij},\qquad
Supply_i=\sum_{j\in M}x_{ji}.
$$

The current cost symbols are exactly:

$$
Cost^d=C^d\sum_{i<j}(j-i)^2x_{ji},\qquad
Cost^s=C^s\sum_{i<j}(j-i)x_{ij},\qquad
Cost^p=C^p\sum_{i\in M}x_{ii}.
$$

Thus late production sent backward is penalized by a positive squared delay, early production held forward by a positive linear storage delay, and only diagonal production is charged production price. This last detail is an implementation fact; it is not the usual cost of all production.

## Objective

$$
\min Cost^d+Cost^s+Cost^p.
$$

## Constraints

$$
Supply_i\ge Demand_i\quad(\forall i\in M),\qquad
Produce_i\le Productivity_i\quad(\forall i\in M).
$$

The old page's $(i-j)x_{ji}$ has the wrong sign, and its $\sum_i Produce_i$ production cost omits both $C^p$ and the source's diagonal-only restriction.

## Implementation notes and expected result

`Demo16` uses `UIntVariable2`, `LinearIntermediateSymbols1<Flt64>`, `LinearExpressionSymbol`, and a current `LinearMetaModel<Flt64>`. A numeric assignment should be regenerated from this objective; the former table is not a verified optimum for the current diagonal-only production-cost implementation. The invariant result is a non-negative integer matrix whose column sums meet demand and whose row sums do not exceed productivity.

## Minimal current Kotlin example

```kotlin
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*
import fuookami.ospf.kotlin.example.solveLinearMetaModel

val model = LinearMetaModel<Flt64>("demo16", converter = flt64Converter)
val x = UIntVariable2("x", Shape2(produces.size, produces.size))
val produce = LinearIntermediateSymbols1<Flt64>("produce", Shape1(produces.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[produces[i], _a]), name = "produce_${produces[i].month}")
}
val supply = LinearIntermediateSymbols1<Flt64>("supply", Shape1(produces.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, produces[i]]), name = "supply_${produces[i].month}")
}
val delay = LinearExpressionSymbol(
    sum(produces.withIndex().flatMap { (i, _) -> produces.withIndex().mapNotNull { (j, _) ->
        if (i < j) Flt64(j - i).sqr() * delayDeliveryPrice * x[produces[j], produces[i]] else null
    } }),
    name = "delay_delivery_cost"
)
val storage = LinearExpressionSymbol(
    sum(produces.withIndex().flatMap { (i, _) -> produces.withIndex().mapNotNull { (j, _) ->
        if (i < j) Flt64(j - i) * stowagePrice * x[produces[i], produces[j]] else null
    } }),
    name = "storage_cost"
)
val production = LinearExpressionSymbol(productPrice * sum(x[_a, _a]), name = "produce_cost")
model.add(x)
model.add(produce)
model.add(supply)
model.add(delay)
model.add(storage)
model.add(production)
model.minimize(delay + storage + production, "cost")
for (p in produces) {
    model.addConstraint(supply[p] geq p.demand)
    model.addConstraint(produce[p] leq p.productivity)
}

suspend fun solve() = solveLinearMetaModel(ScipLinearSolver(), model)
```

## Source and verification

### Kotlin/Rust correspondence

- [Rust counterpart: `src/core/demo16.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo16.rs)

This Rust file is not objective-equivalent to the current Kotlin implementation: Rust charges production price for every `x_ij`, while Kotlin currently charges it only for diagonal `x_ii`; storage, delay, and capacity/demand structures otherwise correspond.

- [Current implementation: `Demo16.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo16.kt)
- [Core build-structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

::: code-group

```kotlin [Kotlin]
// See the linked Kotlin implementation for the complete model.
`` 

```rust [Rust]
// See the linked Rust implementation for the equivalent model.
`` 

:::

