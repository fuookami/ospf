package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.payload_maximization.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

/**
 * 最大业载限制：业载最大化
 *
 * @property aircraftModel          机型
 * @property payload                业载
 * @property coefficient            奖励系数
 */
class MaxPayloadLimit(
    private val aircraftModel: AircraftModel,
    private val payload: Payload,
    private val coefficient: () -> Flt64 = { Flt64.one },
    override val name: String = "max_payload_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.maximize(
            coefficient() * payload.estimatePayload.to(aircraftModel.weightUnit)!!.value
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
