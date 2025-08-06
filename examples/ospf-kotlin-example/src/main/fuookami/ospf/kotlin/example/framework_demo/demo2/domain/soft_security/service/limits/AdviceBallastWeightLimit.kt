package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class AdviceBallastWeightLimit(
    private val aircraftModel: AircraftModel,
    private val ballast: Ballast,
    private val coefficient: () -> Flt64 = { Flt64.one },
    override val name: String = "advice_ballast_weight_limit",
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        if (ballast.adviceBallastWeight != null) {
            val slack = SlackFunction(
                x = LinearPolynomial(ballast.ballastWeight.to(aircraftModel.weightUnit)!!.value),
                threshold = LinearPolynomial(ballast.adviceBallastWeight.to(aircraftModel.weightUnit)!!.value),
                withPositive = false,
                name = "advice_ballast_weight_slack"
            )
            when (val result = model.minimize(
                coefficient() * slack,
                name = "advice ballast weight"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }
}
