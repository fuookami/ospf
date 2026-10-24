package docs.examples.bpp3d

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.times
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.bpp3d.infrastructure.*
import fuookami.ospf.kotlin.framework.bpp3d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp3d.domain.layer_generation.*
import fuookami.ospf.kotlin.framework.bpp3d.application.service.*
import fuookami.ospf.kotlin.utils.functional.*

private object Cargo : AbstractCargoAttribute
private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error("${error.code}: ${error.message}")
    is Fatal -> error(errors.joinToString { "${it.code}: ${it.message}" })
}

fun makeItem(edge: Double = 1.0): ActualItem {
    val material = Material(
        no = MaterialNo("M1"), type = MaterialType.RawMaterial,
        cargo = Cargo, name = "goods", weight = FltX.one * Kilogram
    )
    return ActualItem(
        id = itemIdOf("I1"), name = "carton",
        pack = Package.innerPackage(
            shape = PackageShape(
                width = FltX(edge) * Meter, height = FltX(edge) * Meter, depth = FltX(edge) * Meter,
                weight = FltX.one * Kilogram, packageType = PackageType.CartonContainer
            ),
            materials = mapOf(material to UInt64.one)
        ),
        enabledOrientations = listOf(Orientation.Upright), batchNo = BatchNo("batch-1"),
        packageAttribute = PackageAttribute(
            packageType = PackageType.CartonContainer, weightAttribute = WeightAttribute(),
            deformationAttribute = LinearDeformationAttribute(FltX.zero),
            hangingPolicy = AbsoluteHangingPolicy(FltX.zero), stackingOnPolicy = FilterStackingOnPolicy()
        )
    )
}

suspend fun runExample(solver: ColumnGenerationSolver) {
    val item = makeItem()
    val binType = BinType(
        width = FltX(3.0) * Meter, height = FltX(3.0) * Meter, depth = FltX(3.0) * Meter,
        capacity = FltX(100.0) * Kilogram, longitudinalBalance = null, lateralBalance = null,
        typeCode = binTypeIdOf("BOX")
    )
    // Supply an actual placement, not just a demand-coverage number.
    val layer = BinLayer(
        iteration = Int64.zero, from = Cargo::class, bin = binType, shape = binType.asContainer3Shape(),
        units = listOf(itemPlacement3Of(item, QuantityPoint3(
            FltX.zero * Meter, FltX.zero * Meter, FltX.zero * Meter
        )))
    )
    val response = ColumnGenerationApplicationService(solver).solve(
        request = ColumnGenerationApplicationRequest(
            itemDemands = listOf(item to UInt64.one), initialColumns = listOf(layer),
            finalBins = listOf(layerBinOf(shape = binType, units = emptyList())),
            generators = listOf(object : Bpp3dLayerGenerator<FltX> {
                override suspend fun generate(request: Bpp3dLayerGenerationRequest<FltX>) =
                    emptyList<Bpp3dLayerGenerationResult<FltX>>()
            })
        ),
        packingAnalyzer = ColumnGenerationPackingAnalyzer()
    ).getOrThrow()
    println("finalSolved=${response.result.finalSolved}")
    val packing = response.packingSnapshot ?: error("No deliverable loading plan")
    // #region result
    val bins = packing.packingResult.aggregation.bins
    println("bins=${bins.size}, objective=${response.result.finalObjective}")
    for (bin in bins) {
        println("bin=${bin.name}, type=${bin.type.typeCode}")
        for (packed in bin.items) {
            val p = packed.placement
            println("${(p.unit as? ActualItem)?.id}: (${p.x}, ${p.y}, ${p.z}), orientation=${p.orientation}, order=${packed.loadingOrder}")
        }
    }
    check(bins.flatMap { it.items }.size == 1) { "Expected exactly one carton" }
    // #endregion result
}
