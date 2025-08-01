package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.model.*

class LateralBalanceLimit(
    private val aircraftModel: AircraftModel,
    private val lateralBalance: LateralBalance,
    private val coefficient: () -> Flt64,
    override val name: String = "longitudinal_balance_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            coefficient() * lateralBalance.slack.to(aircraftModel.torqueUnit)!!.value,
            "longitudinal balance"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
