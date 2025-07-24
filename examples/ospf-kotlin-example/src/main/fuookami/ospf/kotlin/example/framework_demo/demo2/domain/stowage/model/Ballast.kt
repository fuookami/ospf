package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*

class Ballast(
    private val aircraftModel: AircraftModel,
    private val positions: List<Position>,
    val ballastPositions: List<Position>,
    val minBallastWeight: Quantity<Flt64>?,
    val adviceBallastWeight: Quantity<Flt64>?,
    val load: Load
) {
    companion object {
        operator fun invoke(
            aircraftModel: AircraftModel,
            positions: List<Position>,
            minBallastWeight: Quantity<Flt64>?,
            load: Load
        ): Ballast {
            TODO("not implemented yet")
        }
    }

    lateinit var ballastWeight: QuantityLinearIntermediateSymbol
    lateinit var adaptiveMinBallastWeight: QuantityLinearIntermediateSymbol

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        if (!::ballastWeight.isInitialized) {
            val poly = MutableLinearPolynomial()
            for (position in ballastPositions) {
                val j = positions.indexOf(position)
                poly += load.estimateLoadWeight[j].to(aircraftModel.weightUnit)!!.value
            }
            ballastWeight = Quantity(
                LinearExpressionSymbol(
                    poly,
                    "ballast_weight"
                ),
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(ballastWeight)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (stowageMode.withSoftSecurity) {
            if (!::adaptiveMinBallastWeight.isInitialized) {
                adaptiveMinBallastWeight = if (minBallastWeight != null) {
                    Quantity(
                        LinearExpressionSymbol(
                            minBallastWeight.to(aircraftModel.weightUnit)!!.value,
                            "min_ballast_weight"
                        ),
                        aircraftModel.weightUnit
                    )
                } else {
                    val poly = MutableLinearPolynomial()
                    // todo
                    Quantity(
                        LinearExpressionSymbol(
                            poly,
                            "min_ballast_weight"
                        ),
                        aircraftModel.weightUnit
                    )
                }
            }
            when (val result = model.add(adaptiveMinBallastWeight)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }
}
