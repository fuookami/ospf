package docs.examples.csp1d

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.*
import fuookami.ospf.kotlin.quantities.unit.Meter
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.csp1d.application.model.*
import fuookami.ospf.kotlin.framework.csp1d.application.service.*
import fuookami.ospf.kotlin.framework.csp1d.domain.material.model.*

suspend fun runExample(solver: ColumnGenerationSolver) {
    fun meters(value: Double) = Quantity(Flt64(value), Meter)
    val product = Product(
        id = productIdOf("P1"), name = "strip", width = listOf(meters(0.8))
    )
    val material = Material(
        id = materialIdOf("M1"), name = "stock",
        widthRange = WidthRange(
            width = QuantityRange(lowerBound = meters(0.5), upperBound = meters(1.5)),
            step = meters(0.1)
        )
    )
    val problem = Csp1dProblem(
        products = listOf(product), materials = listOf(material), machines = emptyList(),
        demands = listOf(ProductDemand.legacyRoll(product, Flt64(3.0)))
    )
    // This API returns a structured solution, not Ret<Csp1dSolution>.
    val solution = Csp1dMilp<Flt64>(solver).solve(problem)
    // #region result
    println("status=${solution.status}, failure=${solution.failureMessage}, kpi=${solution.kpi}")
    println("unmet=${solution.produce.unmetDemands}")
    for (usage in solution.produce.cuttingPlans) {
        val plan = usage.plan
        println("plan=${plan.id}, material=${plan.material.id}, usages=${usage.amount}, rest=${plan.restWidth}")
        plan.slices.forEach { println("width=${it.width}, pieces per usage=${it.amount}") }
        plan.demandContributions.forEach { println("${it.product.id}: per usage=${it.quantity}, multiplier=${usage.amount}") }
    }
    check(solution.produce.unmetDemands.isEmpty()) { "Demand is not fully covered" }
    // #endregion result
}
