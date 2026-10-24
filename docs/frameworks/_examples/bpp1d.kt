package docs.examples.bpp1d

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.Quantity
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.bpp.infrastructure.*
import fuookami.ospf.kotlin.framework.bpp1d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp1d.application.*
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
            println("status=${solution.status}, objective=${solution.objective}")
            println(solution.packing)
        }
        is Failed -> error("${result.error.code}: ${result.error.message}")
        is Fatal -> error(result.errors.joinToString { "${it.code}: ${it.message}" })
    }
}
