package docs.examples.bpp1d

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.*
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.bpp.infrastructure.*
import fuookami.ospf.kotlin.framework.bpp1d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp1d.application.*
import fuookami.ospf.kotlin.framework.bpp1d.domain.packing.*
import fuookami.ospf.kotlin.utils.functional.*

suspend fun runExample(solver: ColumnGenerationSolver) {
    val problem = PackingProblem(
        items = listOf(Item(
            id = "a", length = Quantity(Flt64(40), Centimeter),
            weight = Quantity(Flt64(2), Kilogram), amount = UInt64(3)
        )),
        binTypes = listOf(
            BinType("small", Quantity(Flt64(1), Meter), Quantity(Flt64(5), Kilogram), Flt64.one, UInt64.one),
            BinType("large", Quantity(Flt64(2), Meter), Quantity(Flt64(10), Kilogram), Flt64(3), UInt64.one)
        ),
        spacing = Quantity(Flt64(20), Centimeter),
        objective = PackingObjective.BinCount
    )
    when (val result = Bpp1dMilp(solver, Flt64PackingValueAdapter).solve(problem)) {
        is Ok -> {
            val solution = result.value
            // #region result
            println("status=${solution.status}, candidateComplete=${solution.candidateComplete}, objective=${solution.objective}")
            val packing = solution.packing
            packing.plans.forEach { plan ->
                println("bin=${plan.bin.id}, type=${plan.bin.type.id}")
                plan.placements.forEach { println("${it.itemId}: [${it.start}, ${it.start + it.length})") }
            }
            packing.remnants.forEach { println("tail ${it.binId}: [${it.start}, ${it.start + it.length})") }
            val kpi = packing.kpi(Meter).getOrThrow()
            println("bins=${kpi.binCount}, items=${kpi.itemCount}, cost=${kpi.cost}, utilization=${kpi.utilization}")
            println("consumed=${kpi.consumedLength}, tail=${kpi.remnantLength}")
            // #endregion result
            packing.plans.forEach { plan ->
                plan.placements.zipWithNext().forEach { (left, right) ->
                    println("spacing ${plan.bin.id}: [${left.start + left.length}, ${right.start})")
                }
            }
            val counts = packing.plans.flatMap { it.placements }.groupingBy { it.itemId }.eachCount()
            check(problem.items.all { UInt64((counts[it.id] ?: 0).toULong()) == it.amount })
        }
        is Failed -> error("${result.error.code}: ${result.error.message}")
        is Fatal -> error(result.errors.joinToString { "${it.code}: ${it.message}" })
    }
}

private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error("${error.code}: ${error.message}")
    is Fatal -> error(errors.joinToString { "${it.code}: ${it.message}" })
}
