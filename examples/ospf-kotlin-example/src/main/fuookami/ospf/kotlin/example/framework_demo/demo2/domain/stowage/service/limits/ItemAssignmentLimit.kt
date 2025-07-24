package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class ItemAssignmentLimit(
    private val items: List<Item>,
    private val stowage: Stowage,
    override val name: String = "item_assignment_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((i, item) in items.withIndex()) {
            when (item.status) {
                ItemStatus.Preassigned, ItemStatus.AdjustmentNeeded -> {
                    when (val result = model.addConstraint(
                        stowage.loaded[i] eq true,
                        name = "${name}_${item}"
                    )) {
                        is Ok -> {}

                        is Failed -> {
                            return Failed(result.error)
                        }
                    }
                }

                ItemStatus.Optional -> {
                    when (val result = model.addConstraint(
                        stowage.loaded[i] leq true,
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
