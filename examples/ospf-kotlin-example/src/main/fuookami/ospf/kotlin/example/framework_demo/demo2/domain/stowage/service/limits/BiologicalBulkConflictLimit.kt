package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class BiologicalBulkConflictLimit(
    private val items: List<Item>,
    private val positions: List<Position>,
    private val biologicalLimit: BiologicalLimit,
    private val stowage: Stowage,
    override val name: String = "biological_bulk_conflict_limit"
): Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((j, position) in positions.withIndex()) {
            if (position.status.unavailable || !position.location.bulk) {
                continue
            }

            for ((type1, type2) in biologicalLimit.bulkConflictLimit) {
                if (position.loadedItems.any { it.cargo.contains(type1) } && !position.loadedItems.none { it.cargo.contains(type2) }) {
                    for ((i, item) in items.withIndex()) {
                        if (Stowage.stowageNeeded(item, position) && item.cargo.contains(type2)) {
                            when (val result = model.addConstraint(
                                stowage.stowage[i, j] eq false,
                                "${name}_${item}_${position}"
                            )) {
                                is Ok -> {}

                                is Failed -> {
                                    return Failed(result.error)
                                }
                            }
                        }
                    }
                }
                if (position.loadedItems.any { it.cargo.contains(type2) } && !position.loadedItems.none { it.cargo.contains(type1) }) {
                    for ((i, item) in items.withIndex()) {
                        if (Stowage.stowageNeeded(item, position) && item.cargo.contains(type1)) {
                            when (val result = model.addConstraint(
                                stowage.stowage[i, j] eq false,
                                "${name}_${item}_${position}"
                            )) {
                                is Ok -> {}

                                is Failed -> {
                                    return Failed(result.error)
                                }
                            }
                        }
                    }
                }
                if (position.loadedItems.none { it.cargo.contains(type1) || it.cargo.contains(type2) }) {
                    for ((i1, item1) in items.withIndex()) {
                        if (!Stowage.stowageNeeded(item1, position)) {
                            continue
                        }

                        for (i2 in (i1 + 1) until items.size) {
                            val item2 = items[i2]
                            if (!Stowage.stowageNeeded(item2, position)) {
                                continue
                            }

                            if ((item1.cargo.contains(type1) && item2.cargo.contains(type2))
                                || (item1.cargo.contains(type2) && item2.cargo.contains(type1))
                            ) {
                                when (val result = model.addConstraint(
                                    stowage.stowage[i1, j] + stowage.stowage[i2, j] leq 1,
                                    "${name}_${item1}_${item2}_${position}"
                                )) {
                                    is Ok -> {}

                                    is Failed -> {
                                        return Failed(result.error)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        return ok
    }
}
