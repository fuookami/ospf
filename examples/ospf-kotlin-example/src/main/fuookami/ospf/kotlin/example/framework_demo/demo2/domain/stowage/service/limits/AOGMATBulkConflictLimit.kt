package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class AOGMATBulkConflictLimit(
    private val items: List<Item>,
    private val positions: List<Position>,
    private val stowage: Stowage,
    override val name: String = "aot_mat_bulk_conflict_limit"
): Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((j, position) in positions.withIndex()) {
            if (position.status.unavailable || !position.location.bulk) {
                continue
            }

            if (position.type.contains(PositionTypeCode.AOGMATAppointed)
                || position.loadedItems.any { it.cargo.contains(CargoCode.AOG) || it.cargo.contains(CargoCode.MAT) }
            ) {
                for ((i, item) in items.withIndex()) {
                    if (Stowage.stowageNeeded(item, position)
                        && item.location.bulk
                        && !item.cargo.contains(CargoCode.AOG)
                        && !item.cargo.contains(CargoCode.MAT)
                    ) {
                        when (val result = model.addConstraint(
                            stowage.stowage[i, j] leq 0,
                            name = "${name}_${item}_${position}"
                        )) {
                            is Ok -> {}

                            is Failed -> {
                                return Failed(result.error)
                            }
                        }
                    }
                }
                continue
            }

            if (
                position.type.contains(PositionTypeCode.NormalBulkAppointed)
                    || (position.loadedItems.any { it.location.bulk && !it.cargo.contains(CargoCode.AOG) && !it.cargo.contains(CargoCode.MAT) }
                        && position.loadedItems.none { it.cargo.contains(CargoCode.AOG) || it.cargo.contains(CargoCode.MAT) }
                    )
            ) {
                for ((i, item) in items.withIndex()) {
                    if (Stowage.stowageNeeded(item, position)
                        && (item.cargo.contains(CargoCode.AOG) || item.cargo.contains(CargoCode.MAT))
                    ) {
                        when (val result = model.addConstraint(
                            stowage.stowage[i, j] leq 0,
                            name = "${name}_${item}_${position}"
                        )) {
                            is Ok -> {}

                            is Failed -> {
                                return Failed(result.error)
                            }
                        }
                    }
                }
                continue
            }

            for ((i1, item1) in items.withIndex()) {
                if (!Stowage.stowageNeeded(item1, position)
                    || (!item1.cargo.contains(CargoCode.AOG) && !item1.cargo.contains(CargoCode.MAT))
                ) {
                    continue
                }

                for (i2 in (i1 + 1) until items.size) {
                    val item2 = items[i2]
                    if (!Stowage.stowageNeeded(item2, position)
                        || !item2.location.bulk
                        || item2.cargo.contains(CargoCode.AOG)
                        || item2.cargo.contains(CargoCode.MAT)
                    ) {
                        continue
                    }

                    when (val result = model.addConstraint(
                        stowage.stowage[i1, j] + stowage.stowage[i2, j] leq 1,
                        name = "${name}_${item1}_${item2}_${position}"
                    )) {
                        is Ok -> {}

                        is Failed -> {
                            return Failed(result.error)
                        }
                    }
                }
            }
        }

        return ok
    }
}
