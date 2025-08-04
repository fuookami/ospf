package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class LinearDensity(
    private val aircraftModel: AircraftModel,
    val limitsZones: List<LimitZone>,
    val limitLines: List<LimitLine>,
    private val positions: List<Position>,
    private val load: Load
) {
    data class LimitZone(
        val name: String,
        val locations: Set<DeckLocation>,
        val frontArm: Quantity<Flt64>,
        val backArm: Quantity<Flt64>,
        val maxLinearDensity: Quantity<Flt64>
    )

    data class LimitLine(
        val zone: LimitZone,
        val arm: Quantity<Flt64>,
        val positions: List<Position>
    )

    companion object {
        operator fun invoke(
            aircraftModel: AircraftModel,
            limitZones: List<LimitZone>,
            positions: List<Position>,
            load: Load
        ): LinearDensity {
            TODO("not implemented yet")
        }
    }

    lateinit var linearDensity: QuantityLinearIntermediateSymbols1

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::linearDensity.isInitialized) {
            linearDensity = QuantityLinearIntermediateSymbols1("linear_density", Shape1(positions.size)) { j, _ ->
                val position = positions[j]
                Quantity(
                    LinearExpressionSymbol(
                        (load.estimateLoadWeight[j] / position.shape.length).to(aircraftModel.linearDensityUnit)!!.value,
                        name = "linear_density_${position}",
                    ),
                    aircraftModel.linearDensityUnit
                )
            }
        }
        when (val result = model.add(linearDensity)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
