package docs.examples.bpp1d.business

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.inequality.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.quantities.quantity.Quantity
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.bpp.infrastructure.*
import fuookami.ospf.kotlin.framework.bpp1d.application.*
import fuookami.ospf.kotlin.framework.bpp1d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp1d.domain.pattern_generation.*
import fuookami.ospf.kotlin.framework.bpp1d.domain.pattern_assignment.*
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.*

private val itemsCsv = """
    id,length,weight,amount
    a,6,1,2
    b,4,1,2
""".trimIndent()
private val binsCsv = """
    id,length,max_weight,cost,inventory
    small,10,100,3,2
    large,20,100,10,1
""".trimIndent()

private fun parseNumber(text: String): Ret<Flt64> =
    text.toDoubleOrNull()?.takeIf { it.isFinite() }?.let { Ok(Flt64(it)) }
        ?: Failed(ErrorCode.IllegalArgument, "Invalid number: $text")

fun loadScenario(): PackingProblem<Flt64> {
    // #region csv
    val problem = Bpp1dCsvDatasetLoader(::parseNumber).load(
        itemsCsv, binsCsv, Meter, Kilogram, Quantity(Flt64.zero, Meter)
    ).getOrThrow()
    val costProblem = problem.copy(objective = PackingObjective.Cost)
    // #endregion csv
    return costProblem
}

suspend fun solveNoMixed(solver: ColumnGenerationSolver, problem: PackingProblem<Flt64>): Ret<Bpp1dSolution<Flt64>> {
    // #region policy
    val noMix = PatternPolicy<Flt64> { p -> !("a" in p.amounts && "b" in p.amounts) }
    val generator = PatternGenerationContext(noMix)
    val result = Bpp1dMilp(solver, Flt64PackingValueAdapter, generator).solve(problem)
    // #endregion policy
    return result
}

suspend fun solveColumnGeneration(solver: ColumnGenerationSolver, problem: PackingProblem<Flt64>): Ret<Bpp1dSolution<Flt64>> {
    // #region cg
    val config = Bpp1dSolveConfig<Flt64>(
        generation = PatternGenerationConfig(maxPatterns = 10_000, maxNodes = 100_000),
        iterationLimit = 32, columnsPerIteration = 32
    )
    val result = Bpp1dColumnGeneration(solver, Flt64PackingValueAdapter).solve(problem, config)
    // #endregion cg
    return result
}

// #region extension
class BinLimit(private val limit: Flt64) : PatternModelingExtension<Flt64> {
    override fun register(context: PatternAssignmentContext<Flt64>): Try {
        if (context.stage != PackingModelStage.MILP) {
            return Failed(ErrorCode.IllegalArgument, "This BinLimit example supports ordinary MILP only")
        }
        val bins = sum(context.aggregation.variables.values.toList())
        return context.model.addConstraint(bins leq limit, name = "business_bin_limit")
    }
}
// #endregion extension

suspend fun solveWithBinLimit(solver: ColumnGenerationSolver, problem: PackingProblem<Flt64>): Ret<Bpp1dSolution<Flt64>> {
    // #region extension-injection
    val config = Bpp1dSolveConfig(extensions = listOf(BinLimit(Flt64(2))))
    val result = Bpp1dMilp(solver, Flt64PackingValueAdapter).solve(problem, config)
    // #endregion extension-injection
    return result
}

suspend fun runBusinessExample(solver: ColumnGenerationSolver) {
    val problem = loadScenario()
    for (objective in listOf(PackingObjective.BinCount, PackingObjective.ConsumedLength, PackingObjective.Cost)) {
        val solution = Bpp1dMilp(solver, Flt64PackingValueAdapter)
            .solve(problem.copy(objective = objective)).getOrThrow()
        println("$objective: ${solution.status}, complete=${solution.candidateComplete}, plans=${solution.packing.plans}")
    }
    println("noMix=${solveNoMixed(solver, problem).getOrThrow().packing.plans}")
    println("cg=${solveColumnGeneration(solver, problem).getOrThrow().status}")
    println("binLimit=${solveWithBinLimit(solver, problem).getOrThrow().packing.plans}")
}

private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error("${error.code}: ${error.message}")
    is Fatal -> error(errors.joinToString { "${it.code}: ${it.message}" })
}
