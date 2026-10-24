package docs.examples.csp1d.business

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.*
import fuookami.ospf.kotlin.quantities.unit.Meter
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.csp1d.application.model.*
import fuookami.ospf.kotlin.framework.csp1d.application.service.*
import fuookami.ospf.kotlin.framework.csp1d.domain.material.model.*
import fuookami.ospf.kotlin.framework.csp1d.domain.produce.model.Csp1dExtensionSet
import fuookami.ospf.kotlin.framework.csp1d.domain.cutting_plan_generation.service.NSameGenerator
import fuookami.ospf.kotlin.utils.functional.*

fun initialGenerator(): NSameGenerator<Flt64> {
    val arithmetic = when (val result = DefaultQuantityArithmetic.resolveFor(Flt64.zero)) {
        is Ok -> result.value
        is Failed -> error(result.error.message)
        is Fatal -> error(result.errors.joinToString { it.message })
    }
    return NSameGenerator(arithmetic = arithmetic, allAmount = false)
}

fun makeProblem(): Csp1dProblem<Flt64> {
    fun m(value: Double) = Quantity(Flt64(value), Meter)
    // #region input
    val product = Product(id = productIdOf("A"), name = "strip", width = listOf(m(0.8)))
    val stock = Material(id = materialIdOf("M1"), name = "stock",
        widthRange = WidthRange(QuantityRange(m(0.8), m(1.6)), m(0.8)), availableBatches = UInt64(3))
    return Csp1dProblem(products = listOf(product), materials = listOf(stock, stock.copy(id = materialIdOf("M2"))),
        machines = emptyList(), demands = listOf(ProductDemand.legacyRoll(product, Flt64(6.0))))
    // #endregion input
}

suspend fun runBusiness(solver: ColumnGenerationSolver) {
    val problem = makeProblem()
    // #region rule
    val quarantine = object : Csp1dDomainPolicy<Flt64> {
        override val name = "quarantine-M2"
        override fun isFeasible(context: Csp1dDomainCalculationContext<Flt64>) = context.material.id != materialIdOf("M2")
    }
    val config = Csp1dSolveConfig(extensionSet = Csp1dExtensionSet(domainPolicies = listOf(quarantine)))
    val milp = Csp1dMilp<Flt64>(solver, initialGenerator = initialGenerator()).solve(problem.copy(solveConfig = config))
    // #endregion rule
    println("milp=${milp.status}, unmet=${milp.produce.unmetDemands}")
    // #region entry
    val bounded = config.copy(columnGeneration = Csp1dConfiguration(
        maxInitialPlans = Int64(64), maxPricingPlans = Int64(16), iterationLimit = Int64(8)))
    val cg = Csp1dColumnGeneration<Flt64>(solver, initialGenerator = initialGenerator()).solveWithTrace(problem, bounded)
    println("termination=${cg.trace.terminationReason}, final=${cg.trace.finalMilpStatus}")
    // #endregion entry
}
