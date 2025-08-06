package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.model.*

class RedundancyLimit(
    private val redundancy: Redundancy,
    private val coefficient: () -> Flt64,
    override val name: String = "redundancy"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            coefficient() * redundancy.redundancySlack,
            "redundancy"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
