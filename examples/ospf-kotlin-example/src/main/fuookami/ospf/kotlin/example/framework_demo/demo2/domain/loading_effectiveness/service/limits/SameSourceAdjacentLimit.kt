package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.model.*

class SameSourceAdjacentLimit(
    private val adjacentPositions: List<PositionPair>,
    private val sources: List<FlightNo>,
    private val loading: TransferAdjacentLoading,
    private val coefficient: (FlightNo, Position, Position) -> Flt64 = { _, _, _ -> Flt64.one },
    override val name: String = "same_source_adjacent_limit",
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.maximize(
            sum(sources.flatMapIndexed { s, source ->
                adjacentPositions.mapIndexed { p, (position1, position2) ->
                    coefficient(source, position1, position2) * loading.sameSourceAdjacent[s, p]
                }
            }),
            name = "same source adjacent"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
