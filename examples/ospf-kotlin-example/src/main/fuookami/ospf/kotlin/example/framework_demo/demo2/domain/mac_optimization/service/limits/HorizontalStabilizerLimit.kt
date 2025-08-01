package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*

class HorizontalStabilizerLimit(
    private val horizontalStabilizers: Map<HorizontalStabilizer.Key, HorizontalStabilizer>,
    private val coefficient: (HorizontalStabilizer.Key) -> Flt64,
    override val name: String = "horizontal_stabilizer_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((key, hs) in horizontalStabilizers) {
            when (val result = model.minimize(
                coefficient(key) * hs.warnSlack,
                "horizontal stabilizer $key"
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
