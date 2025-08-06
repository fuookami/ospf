package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.model.*

class ExperimentalLongitudinalBalanceLimit(
    private val aircraftModel: AircraftModel,
    private val longitudinalBalance: ExperimentalLongitudinalBalance,
    private val coefficient: () -> Flt64,
    override val name: String = "experimental_longitudinal_balance_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            coefficient() * longitudinalBalance.longitudinalTorqueSlack.to(aircraftModel.torqueUnit)!!.value,
            "experimental longitudinal balance"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
