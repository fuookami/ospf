package docs.examples.csp2d

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.times
import fuookami.ospf.kotlin.quantities.unit.Meter
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.csp2d.application.model.*
import fuookami.ospf.kotlin.framework.csp2d.application.service.*
import fuookami.ospf.kotlin.framework.csp2d.domain.material.model.*
import fuookami.ospf.kotlin.framework.csp2d.infrastructure.CuttingProcess2
import fuookami.ospf.kotlin.utils.functional.*

suspend fun runExample(solver: ColumnGenerationSolver) {
    val sheet = Material2(
        id = materialId2Of("SHEET"), name = "sheet",
        width = FltX(100.0) * Meter, height = FltX(60.0) * Meter
    )
    val product = Product2(
        id = productId2Of("P1"), name = "P1",
        width = FltX(40.0) * Meter, height = FltX(30.0) * Meter
    )
    val problem = Csp2dProblem(
        materials = listOf(sheet), products = listOf(product),
        demands = listOf(ProductDemand2(product = product, demand = DemandValue2.Amount(UInt64(3)))),
        kerf = FltX.zero * Meter, process = CuttingProcess2.Freeform,
        config = Csp2dConfiguration(maxCandidatesPerIteration = 12, iterationLimit = 4)
    )
    when (val result = Csp2dMilp(solver).solve(problem)) {
        is Ok -> {
            val solution = result.value
            // #region result
            println("covered=${solution.covers(problem)}, sheets=${solution.sheetCount}, length=${solution.consumedLength}")
            println("coverage=${solution.coverage}, used=${solution.usedArea}, waste=${solution.wasteArea}, utilization=${solution.utilization}")
            for (usage in solution.usages) {
                check(usage.plan.validate(problem.kerf) is Ok) { "Invalid cutting pattern" }
                println("plan=${usage.plan.id}, material=${usage.plan.pattern.material.id}, usages=${usage.usage}")
                usage.plan.pattern.placements.forEach { println("${it.itemId}: (${it.x}, ${it.y}), width=${it.width}, height=${it.height}") }
                println("cut tree=${usage.plan.pattern.cutTree}")
            }
            check(solution.covers(problem)) { "Demand is not fully covered" }
            // #endregion result
        }
        is Failed -> error("${result.error.code}: ${result.error.message}")
        is Fatal -> error(result.errors.joinToString { "${it.code}: ${it.message}" })
    }
}
