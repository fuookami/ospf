package docs.examples.bpp3d.business

import docs.examples.bpp3d.makeItem
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.times
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.bpp3d.infrastructure.*
import fuookami.ospf.kotlin.framework.bpp3d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp3d.domain.layer_generation.*
import fuookami.ospf.kotlin.framework.bpp3d.application.service.*

private object Grid

fun makeRequest(costObjective: Boolean = false, smallOnly: Boolean = false): ColumnGenerationApplicationRequest {
    // #region input
    val item = makeItem(edge = 2.0)
    fun bin(id: String, depth: Double) = BinType(width = FltX(4.0) * Meter, height = FltX(4.0) * Meter,
        depth = FltX(depth) * Meter, capacity = FltX(100.0) * Kilogram,
        longitudinalBalance = null, lateralBalance = null, typeCode = binTypeIdOf(id))
    val small = bin("S", 4.0)
    val large = bin("L", 6.0)
    val physicalBins = if (smallOnly) listOf(small, small) else listOf(small, small, large)
    // #endregion input
    // Enumerate actual prefix placements, not selected bins or solver answers.
    val layers = physicalBins.distinct().flatMap { type ->
        val slots = (0 until if (type.typeCode == small.typeCode) 8 else 10).map { n ->
            itemPlacement3Of(item, QuantityPoint3(FltX((n % 2) * 2.0) * Meter,
                FltX(((n / 2) % 2) * 2.0) * Meter, FltX((n / 4) * 2.0) * Meter))
        }
        (1..slots.size).map { n -> BinLayer(iteration = Int64.zero, from = Grid::class,
            bin = type, shape = type.asContainer3Shape(), units = slots.take(n)) }
    }
    // #region rule
    val generator = object : Bpp3dLayerGenerator<FltX> {
        override suspend fun generate(request: Bpp3dLayerGenerationRequest<FltX>) =
            layers.filter { !smallOnly || it.bin?.typeCode == small.typeCode }
                .map { Bpp3dLayerGenerationResult<FltX>(layer = it, source = "approved-grid") }
    }
    val request = ColumnGenerationApplicationRequest(itemDemands = listOf(item to UInt64(10)),
        initialColumns = layers, generators = listOf(generator),
        finalBins = physicalBins.map { layerBinOf(shape = it, units = emptyList()) })
    // #endregion rule
    // #region objective
    return if (!costObjective) request else request.copy(executorConfig = ColumnGenerationStandardExecutorConfig(
        finalBinVolumeWeight = FltX.zero, finalBinAmountCoefficient = FltX.zero, finalBinTypeCostWeight = FltX.one,
        finalBinTypeCosts = mapOf(small.typeCode to FltX(4.0), large.typeCode to FltX(10.0))))
    // #endregion objective
}

// #region entry
suspend fun runBusiness(solver: ColumnGenerationSolver) {
    val request = makeRequest(costObjective = true).copy(cgConfig = ColumnGenerationConfig(iterationLimit = 12, maxColumnsPerIteration = 24))
    val response = ColumnGenerationApplicationService(solver).solve(request, packingAnalyzer = ColumnGenerationPackingAnalyzer())
    println(response)
}
// #endregion entry
