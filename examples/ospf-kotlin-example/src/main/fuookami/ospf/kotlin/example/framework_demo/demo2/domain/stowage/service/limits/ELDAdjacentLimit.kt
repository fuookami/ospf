package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class ELDAdjacentLimit(
    private val items: List<Item>,
    private val positions: List<Position>,
    private val neighbours: List<Neighbour>,
    private val stowage: Stowage,
    override val name: String = "eld_adjacent_limit"
): Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((i1, item1) in items.withIndex()) {
            if (!item1.cargo.contains(CargoCode.ELD)) {
                continue
            }

            for (i2 in (i1 + 1) until items.size) {
                val item2 = items[i2]
                if (!item2.cargo.contains(CargoCode.ELD)) {
                    continue
                }

                for (neighbour in neighbours) {
                    val j1 = positions.indexOfFirst { it.base == neighbour.pair.first }
                    val position1 = positions[j1]
                    val j2 = positions.indexOfFirst { it.base == neighbour.pair.second }
                    val position2 = positions[j2]

                    if (Stowage.stowageNeeded(item1, position1) || Stowage.stowageNeeded(item2, position2)) {
                        when (val result = model.addConstraint(
                            stowage.stowage[i1, j1] + stowage.stowage[i2, j2] leq 1,
                            name = "${name}_${item1}_${item2}_${position1}_${position2}"
                        )) {
                            is Ok -> {}

                            is Failed -> {
                                return Failed(result.error)
                            }
                        }
                    }
                    if (Stowage.stowageNeeded(item1, position2) || Stowage.stowageNeeded(item2, position1)) {
                        when (val result = model.addConstraint(
                            stowage.stowage[i1, j2] + stowage.stowage[i2, j1] leq 1,
                            name = "${name}_${item1}_${item2}_${position2}_${position1}"
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

        return ok
    }
}
