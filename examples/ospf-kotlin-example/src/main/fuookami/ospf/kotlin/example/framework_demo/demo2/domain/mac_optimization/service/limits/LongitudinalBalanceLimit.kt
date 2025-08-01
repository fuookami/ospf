package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.model.*

class LongitudinalBalanceLimit(
    private val aircraftModel: AircraftModel,
    private val longitudinalBalance: LongitudinalBalance,
    private val coefficient: (MACRange.Type) -> Flt64,
    override val name: String = "longitudinal_balance_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        val poly = MutableLinearPolynomial()
        for ((range, slack) in longitudinalBalance.slack) {
            poly += coefficient(range) * slack.to(aircraftModel.torqueUnit)!!.value
        }

        when (val result = model.minimize(poly, "longitudinal balance")) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
