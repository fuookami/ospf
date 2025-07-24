package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class RecommendLoadWeightLimit(
    private val positions: List<Position>,
    private val load: Load,
    override val name: String = "recommend_load_weight_limit",
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((j, position) in positions.withIndex()) {
            if ((position.status.stowageNeeded || position.status.adjustmentNeeded)
                && position.status.recommendedWeightNeeded
            ) {
                when (val result = model.addConstraint(
                    load.z[j] + position.mlw.mlw * load.actualLoaded[j] leq position.mlw.mlw,
                    "recommend_load_weight_limit_${position}",
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }
        }

        return ok
    }
}
