package docs.examples.csp2d.business

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.times
import fuookami.ospf.kotlin.quantities.unit.Meter
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.csp2d.application.model.*
import fuookami.ospf.kotlin.framework.csp2d.application.service.*
import fuookami.ospf.kotlin.framework.csp2d.domain.material.model.*
import fuookami.ospf.kotlin.framework.csp2d.domain.pattern_generation.*
import fuookami.ospf.kotlin.framework.csp2d.domain.produce.*
import fuookami.ospf.kotlin.framework.csp2d.infrastructure.*
import fuookami.ospf.kotlin.framework.geometry2d.*
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.utils.functional.*

fun makeProblem(kerf: Double = 0.0): Csp2dProblem {
    fun m(value: Double) = FltX(value) * Meter
    // #region input
    val sheet = Material2(materialId2Of("S"), "sheet", m(10.0), m(6.0))
    val a = Product2(productId2Of("a"), "a", m(4.0), m(6.0), allowRotate = false)
    val b = Product2(productId2Of("b"), "b", m(6.0), m(6.0), allowRotate = false)
    return Csp2dProblem(materials = listOf(sheet), products = listOf(a, b),
        demands = listOf(a, b).map { ProductDemand2(it, DemandValue2.Amount(UInt64.one)) },
        kerf = m(kerf), process = CuttingProcess2.GuillotineTwoStage)
    // #endregion input
}

// #region rule
class NoMixedProducts(private val delegate: PatternGenerator2<FltX>) : PatternGenerator2<FltX> {
    override fun generate(request: PatternGenerationRequest2<FltX>): Ret<List<PatternGenerationResult2<FltX>>> {
        val candidates = ArrayList<PatternGenerationResult2<FltX>>()
        // Generate alternatives as well as rejecting mixed historical patterns.
        for (product in request.products) {
            when (val result = delegate.generate(request.copy(products = listOf(product)))) {
                is Ok -> candidates.addAll(result.value.filter { it.pattern.productAmounts().size <= 1 })
                is Failed -> return Failed(result.error)
                is Fatal -> return Fatal(result.errors)
            }
        }
        return Ok(candidates)
    }
}
// #endregion rule

// Domain generation seam: the high-level solvers do not accept this generator.
fun restrictedPlans(): Ret<List<CuttingPlan2<FltX>>> {
    val problem = makeProblem()
    // #region injection
    val generator: PatternGenerator2<FltX> = NoMixedProducts(PatternGenerationContext2())
    val request = PatternGenerationRequest2(material = problem.materials.first(),
        products = problem.products.map { it to UInt64.one }, kerf = problem.kerf, process = problem.process)
    return when (val generated = generator.generate(request)) {
        is Ok -> {
            val plans = ArrayList<CuttingPlan2<FltX>>()
            for ((index, result) in generated.value.withIndex()) {
                when (val plan = cuttingPlan2Of(planId2Of("restricted-$index"), result.pattern, problem.productsById())) {
                    is Ok -> plans.add(plan.value)
                    is Failed -> return Failed(plan.error)
                    is Fatal -> return Fatal(plan.errors)
                }
            }
            Ok(plans)
        }
        is Failed -> Failed(generated.error)
        is Fatal -> Fatal(generated.errors)
    }
    // #endregion injection
}

private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error(error.message)
    is Fatal -> error(errors.joinToString { it.message })
}

// Specialized one-through-cut templates for this lesson's full-height products.
fun oneCutPlans(kerf: Double = 0.5): List<CuttingPlan2<FltX>> {
    val problem = makeProblem(kerf)
    val material = problem.materials.first()
    val zero = FltX.zero * Meter
    // #region cut
    val region = Region2(zero, zero, material.width, material.height!!)
    return problem.products.map { product ->
        val (left, right) = region.cut(CutOrientation2.Vertical, product.width, problem.kerf).getOrThrow()
        val tree = CutTree2.Split2(region, CutLine2(CutOrientation2.Vertical, product.width),
            CutTree2.Leaf2(left, pieceId = product.id.value), CutTree2.Leaf2(right))
        val placement = Placement2(rectangleShape2(product.width, product.height).getOrThrow(),
            QuantityPoint2(zero, zero), itemId = product.id.value)
        val pattern = Pattern2(patternId2Of("cut-${product.id.value}"), material, region, listOf(placement),
            cutTree = tree, process = problem.process)
        pattern.validate(problem.kerf).getOrThrow()
        cuttingPlan2Of(planId2Of("plan-${product.id.value}"), pattern, problem.productsById()).getOrThrow()
    }
    // #endregion cut
}

suspend fun solveRestricted(solver: ColumnGenerationSolver, kerf: Double = 0.0) {
    val problem = makeProblem(kerf)
    val plans = if (kerf == 0.0) restrictedPlans().getOrThrow() else oneCutPlans(kerf)
    // #region master
    val model = LinearMetaModel<FltX>(name = "restricted-csp2d", objectCategory = ObjectCategory.Minimum,
        configuration = MetaModelConfiguration(), converter = IntoValue.fromConverter(FltX))
    val assignment = Csp2dPatternAssignment(plans)
    assignment.register(model, problem.totalPieceDemand()).getOrThrow()
    Csp2dDemandConstraint(plans, assignment, problem.demands).invoke(model).getOrThrow()
    Csp2dMaterialAvailabilityConstraint(plans, assignment, problem.materials).invoke(model).getOrThrow()
    Csp2dUsageObjective(plans, assignment, { FltX.one }).invoke(model).getOrThrow()
    val solved = solver.solveMILPAs("restricted-csp2d", model).getOrThrow()
    model.setSolution(solved.values)
    plans.forEachIndexed { index, plan ->
        val count = model.tokens.find(assignment.usage[index])?.doubleResult ?: error("Missing plan usage")
        println("plan=${plan.id}, usages=$count, per usage=${plan.pattern.productAmounts()}")
    }
    // #endregion master
}

// #region entry
suspend fun runBusiness(solver: ColumnGenerationSolver) {
    val problem = makeProblem(kerf = 0.5).copy(config = Csp2dConfiguration(maxCandidatesPerIteration = 24, iterationLimit = 8))
    val milp = Csp2dMilp(solver).solve(problem)
    val cg = Csp2dColumnGeneration(solver).solve(problem)
    println("milp=$milp, cg=$cg")
}
// #endregion entry
