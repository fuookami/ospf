package docs.examples.bpp2d

import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.quantities.quantity.times
import fuookami.ospf.kotlin.quantities.unit.*
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.geometry2d.*
import fuookami.ospf.kotlin.framework.bpp2d.domain.item.model.*
import fuookami.ospf.kotlin.framework.bpp2d.application.service.*
import fuookami.ospf.kotlin.utils.functional.*

private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error("${error.code}: ${error.message}")
    is Fatal -> error(errors.joinToString { "${it.code}: ${it.message}" })
}

suspend fun runExample(solver: ColumnGenerationSolver) {
    fun rectangle(id: String, width: Double, height: Double) = ActualItem(
        id = ItemId(id), name = id,
        shape = rectangleShape2(FltX(width) * Meter, FltX(height) * Meter).getOrThrow(),
        allowRotate = true, weight = FltX.one * Kilogram
    )
    val items = listOf(rectangle("R1", 4.0, 3.0) to UInt64(6), rectangle("R2", 2.0, 2.0) to UInt64(8))
    val sheet = SheetType(
        width = FltX(20.0) * Meter, height = FltX(12.0) * Meter,
        maxLoad = FltX(10000.0) * Kilogram, typeCode = SheetTypeId("STD"), isMain = true
    )
    val request = ColumnGenerationApplicationRequest(
        items = items.toMap(), itemCatalog = items.associate { it.first.id to it.first },
        sheetTypes = listOf(sheet), iterationLimit = 12, maxColumnsPerIteration = 12
    )
    val response = ColumnGenerationApplicationService(solver = solver).solve(request).getOrThrow()
    val packing = response.packing ?: error("No deliverable packing; inspect the solve result")
    val bins = packing.aggregation.bins
    val counts = bins.flatMap { it.items }.groupingBy { it.placement.itemId }.eachCount()
    // #region result
    println("final=${response.result.finalSolved}, limited=${response.result.terminatedByIterationLimit}")
    println("kpi=${response.kpi}, placed=$counts")
    for (bin in bins) {
        println("sheet=${bin.name}, type=${bin.type.typeCode}")
        for (item in bin.items) {
            val p = item.placement
            println("${p.itemId}: (${p.x}, ${p.y}) -> (${p.maxX().getOrThrow()}, ${p.maxY().getOrThrow()}), rotated=${p.rotated}")
        }
    }
    packing.remnants.forEach { println("${it.sheetName}: free=${it.rectangle}, area=${it.area}") }
    // #endregion result
    for ((item, demand) in items) {
        check(UInt64(counts[item.id.value] ?: 0) == demand) { "Demand mismatch: ${item.id}" }
    }
}
