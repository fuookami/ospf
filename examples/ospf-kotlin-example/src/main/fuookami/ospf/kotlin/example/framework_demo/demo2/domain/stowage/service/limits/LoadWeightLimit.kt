package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class LoadWeightLimit(
    private val positions: List<Position>,
    private val load: Load,
    private val maxLoadWeight: MaxLoadWeight,
    override val name: String = "load_weight_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((j, position) in positions.withIndex()) {
            if (position.status.available) {
                when (val result = model.addConstraint(
                    load.estimateLoadWeight[j] leq maxLoadWeight.maxLoadWeight[j],
                    "${name}_${position}"
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
