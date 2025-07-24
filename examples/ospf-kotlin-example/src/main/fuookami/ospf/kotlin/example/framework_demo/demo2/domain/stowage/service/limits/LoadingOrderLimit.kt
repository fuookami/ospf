package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class LoadingOrderLimit(
    private val positions: List<Position>,
    private val neighbours: List<Neighbour>,
    private val load: Load,
    override val name: String = "loading_order_limit"
): Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (neighbour in neighbours) {
            val j1 = positions.indexOfFirst { it.base == neighbour.pair.first }
            val position1 = positions[j1]
            val j2 = positions.indexOfFirst { it.base == neighbour.pair.second }
            val position2 = positions[j2]

            if (position1.status.unavailable
                || position2.status.unavailable
                || position1.location.bulk
                || position2.location.bulk
            ) {
                continue
            }

            when (val result = model.addConstraint(
                load.actualLoaded[j1] geq load.actualLoaded[j2],
                name = "${name}_${position1}_${position2}"
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
