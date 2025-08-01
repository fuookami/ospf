package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*

class LateralBalance(
    private val aircraftModel: AircraftModel,
    private val torque: Torque
) {
    lateinit var slack: QuantityLinearIntermediateSymbol

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::slack.isInitialized) {
            slack = Quantity(
                SlackFunction(
                    x = LinearPolynomial(torque.lateralTorque.to(aircraftModel.torqueUnit)!!.value),
                    y = LinearPolynomial(Flt64.zero),
                    name = "lateral_balance_slack"
                ),
                aircraftModel.torqueUnit
            )
        }
        when (val result = model.add(slack)) {
            is Ok -> {}

            is Failed -> {

            }
        }

        return ok
    }
}
