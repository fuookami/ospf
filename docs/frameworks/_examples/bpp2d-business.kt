package docs.examples.bpp2d.business

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.times
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.geometry2d.*
import fuookami.ospf.kotlin.framework.bpp2d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp2d.domain.layer_generation.*
import fuookami.ospf.kotlin.framework.bpp2d.application.service.*
import fuookami.ospf.kotlin.utils.functional.*

private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error(error.message)
    is Fatal -> error(errors.joinToString { it.message })
}

fun makeRequest(costObjective: Boolean = false, spacing: Double = 0.0): ColumnGenerationApplicationRequest {
    // #region input
    val part = ActualItem(ItemId("R"), "part", rectangleShape2(FltX(2.0) * Meter, FltX(2.0) * Meter).getOrThrow(),
        allowRotate = false, weight = FltX.one * Kilogram)
    fun sheet(id: String, side: Double) = SheetType(FltX(side) * Meter, FltX(side) * Meter,
        FltX(100.0) * Kilogram, SheetTypeId(id), isMain = true)
    val request = ColumnGenerationApplicationRequest(items = mapOf(part to UInt64(4)), itemCatalog = mapOf(part.id to part),
        sheetTypes = listOf(sheet("S", 4.0), sheet("L", 5.0)), kerf = FltX(spacing) * Meter)
    // #endregion input
    // #region objective
    return if (!costObjective) request else request.copy(executorConfig = ColumnGenerationStandardExecutorConfig(
        sheetAreaWeight = FltX.zero, sheetAmountWeight = FltX.zero, sheetTypeCostWeight = FltX.one,
        sheetTypeCosts = mapOf(SheetTypeId("S") to FltX(5.0), SheetTypeId("L") to FltX(3.0))))
    // #endregion objective
}

// #region rule
class ApprovedSheets(private val delegate: Bpp2dLayerGenerator<FltX>) : Bpp2dLayerGenerator<FltX> {
    override fun generate(request: Bpp2dLayerGenerationRequest<FltX>) =
        delegate.generate(request).filter { it.layer.sheet?.typeCode == SheetTypeId("S") }
}
// #endregion rule

suspend fun runBusiness(solver: ColumnGenerationSolver, restrictSheets: Boolean = false) {
    val request = makeRequest(costObjective = true)
    // #region entry
    val bounded = request.copy(iterationLimit = 12, maxColumnsPerIteration = 24, finalMilpEnabled = true)
    // #endregion entry
    // #region injection
    val service = if (!restrictSheets) ColumnGenerationApplicationService(solver) else
        ColumnGenerationApplicationService(solver, algorithmFactory = { req, executors ->
            val generator = ApprovedSheets(LayerGenerationContext<FltX>())
            ColumnGenerationAlgorithm(layerGenerator = generator, rmpSolver = executors.rmpSolver(),
                finalMilpSolver = executors.finalSolver(), layerRequestBuilder = executors.requestBuilder(),
                filterByReducedCostWithResult = executors.reducedCostFilter(), initialColumnsWithResult = {
                    val seed = executors.requestBuilder().build(ColumnGenerationState(iteration = 0, columns = emptyList()),
                        ColumnGenerationConfig(maxColumnsPerIteration = req.maxColumnsPerIteration))
                    Ok(generator.generate(seed).map { it.layer })
                })
        })
    val response = service.solve(bounded).getOrThrow()
    // #endregion injection
    println("kpi=${response.kpi}, final=${response.result.finalInfo}")
}
