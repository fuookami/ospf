package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.model

import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class ExperimentalLongitudinalBalance(
    private val aircraftModel: AircraftModel,
    private val positions: List<Position>,
    private val load: Load,
    private val payload: Payload,
    private val redundancy: Redundancy
) {
    lateinit var mainActualLongitudinalTorque: QuantityLinearIntermediateSymbol
    lateinit var predicateLongitudinalTorque: QuantityLinearIntermediateSymbol
    lateinit var longitudinalTorqueSlack: QuantityLinearIntermediateSymbol

    val minLongitudinalTorque: QuantityLinearIntermediateSymbol by lazy {
        TODO("not implemented yet")
    }

    val maxLongitudinalTorque: QuantityLinearIntermediateSymbol by lazy {
        TODO("not implemented yet")
    }

    fun register(model: AbstractLinearMetaModel): Try {
        if (!::mainActualLongitudinalTorque.isInitialized) {
            val poly = MutableLinearPolynomial()
            for ((j, position) in positions.withIndex()) {
                if (position.location.main) {
                    poly += load.loadActualLongitudinalTorque[j].to(aircraftModel.torqueUnit)!!.value
                }
            }
            mainActualLongitudinalTorque = Quantity(
                LinearExpressionSymbol(
                    poly,
                    name = "main_actual_longitudinal_torque"
                ),
                aircraftModel.torqueUnit
            )
        }
        when (val result = model.add(mainActualLongitudinalTorque)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::predicateLongitudinalTorque.isInitialized) {
            TODO("not implemented yet")
        }
        when (val result = model.add(predicateLongitudinalTorque)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::longitudinalTorqueSlack.isInitialized) {
            longitudinalTorqueSlack = Quantity(
                SlackRangeFunction(
                    x = LinearPolynomial(mainActualLongitudinalTorque.to(aircraftModel.torqueUnit)!!.value),
                    lb = LinearPolynomial(minLongitudinalTorque.to(aircraftModel.torqueUnit)!!.value),
                    ub = LinearPolynomial(maxLongitudinalTorque.to(aircraftModel.torqueUnit)!!.value),
                    name = "longitudinal_torque_slack"
                ),
                aircraftModel.torqueUnit
            )
        }
        when (val result = model.add(longitudinalTorqueSlack)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
