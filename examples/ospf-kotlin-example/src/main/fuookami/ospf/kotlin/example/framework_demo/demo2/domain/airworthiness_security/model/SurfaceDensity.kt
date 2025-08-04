package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class SurfaceDensity(
    private val aircraftModel: AircraftModel,
    val limitsZones: List<LimitZone>,
    private val positions: List<Position>,
    private val load: Load
) {
    data class LimitZone(
        val name: String,
        val locations: Set<DeckLocation>,
        val frontArm: Quantity<Flt64>,
        val backArm: Quantity<Flt64>,
        val maxSurfaceDensity: Quantity<Flt64>
    )

    lateinit var surfaceDensity: QuantityLinearIntermediateSymbols1

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::surfaceDensity.isInitialized) {
            surfaceDensity = QuantityLinearIntermediateSymbols1("surface_density", Shape1(positions.size)) { j, _ ->
                val position = positions[j]
                Quantity(
                    LinearExpressionSymbol(
                        (load.estimateLoadWeight[j] / position.shape.area).to(aircraftModel.surfaceDensityUnit)!!.value,
                        name = "surface_density_${position}",
                    ),
                    aircraftModel.linearDensityUnit
                )
            }
        }
        for ((j, position) in positions.withIndex()) {
            if (limitsZones.any { position.coordinate.withIntersectionWith(it.frontArm, it.backArm) }) {
                when (val result = model.add(surfaceDensity[j])) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }
        }

        return ok
    }
}
