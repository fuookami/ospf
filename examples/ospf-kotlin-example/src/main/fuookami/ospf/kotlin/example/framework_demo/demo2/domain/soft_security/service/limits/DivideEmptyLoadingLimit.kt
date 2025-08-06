package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.model.*

class DivideEmptyLoadingLimit(
    private val adjacentPositions: List<PositionPair>,
    private val divideEmptyLoading: DivideEmptyLoading,
    private val emptyBetweenCargoCoefficient: (Position, Position) -> Flt64 = { _, _ -> Flt64.one },
    private val emptyCargoBetweenCargoCoefficient: (Position, Position) -> Flt64 = { _, _ -> Flt64.one },
    private val emptyBetweenEmptyCargoCoefficient: (Position, Position) -> Flt64 = { _, _ -> Flt64.one },
    override val name: String = "divide_empty_loading_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            sum(adjacentPositions.mapIndexed { p, (position1, position2) ->
                emptyBetweenCargoCoefficient(position1, position2) * divideEmptyLoading.emptyBetweenCargo[p]
            }),
            "empty between cargo"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = model.minimize(
            sum(adjacentPositions.mapIndexed { p, (position1, position2) ->
                emptyCargoBetweenCargoCoefficient(position1, position2) * divideEmptyLoading.emptyCargoBetweenCargo[p]
            }),
            "empty cargo between cargo"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = model.minimize(
            sum(adjacentPositions.mapIndexed { p, (position1, position2) ->
                emptyBetweenEmptyCargoCoefficient(position1, position2) * divideEmptyLoading.emptyBetweenEmptyCargo[p]
            })
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
