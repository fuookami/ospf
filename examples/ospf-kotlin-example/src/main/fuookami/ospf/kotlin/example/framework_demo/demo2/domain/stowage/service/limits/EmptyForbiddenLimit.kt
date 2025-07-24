package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class EmptyForbiddenLimit(
    private val items: List<Item>,
    private val positions: List<Position>,
    private val load: Load,
    override val name: String = "empty_forbidden_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((j, position) in positions.withIndex()) {
            if (position.status.available && position.type.contains(PositionTypeCode.EmptyForbidden)) {
                when (val result = model.addConstraint(
                    load.estimateLoaded[j] eq true,
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
