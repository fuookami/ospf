package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class ItemAdjustmentLimit(
    private val items: List<Item>,
    private val stowage: Stowage,
    override val name: String = "item_adjustment_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((i, item) in items.withIndex()) {
            when (item.status) {
                ItemStatus.AdjustmentNeeded -> {
                    when (val result = model.addConstraint(
                        sum(stowage.u[i, _a]) eq BalancedTrivalent.Unknown,
                        name = "${name}_${item}"
                    )) {
                        is Ok -> {}

                        is Failed -> {
                            return Failed(result.error)
                        }
                    }
                }

                else -> {}
            }
        }

        return ok
    }
}
