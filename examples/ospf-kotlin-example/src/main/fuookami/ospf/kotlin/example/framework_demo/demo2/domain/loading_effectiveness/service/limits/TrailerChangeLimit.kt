package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.model.*

class TrailerChangeLimit(
    private val adjacentPositions: List<PositionPair>,
    private val orderedTrailers: List<Pair<Trailer, Trailer>>,
    private val loading: TrailerLoading,
    private val coefficient: (Pair<Position, Trailer>, Pair<Position, Trailer>) -> Flt64 = { _, _ -> Flt64.one },
    override val name: String = "trailer_change_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            sum(orderedTrailers.flatMapIndexed { p1, (trailer1, trailer2) ->
                adjacentPositions.mapIndexed { p2, (position1, position2) ->
                    coefficient(position2 to trailer1, position1 to trailer2) * loading.trailerChange[p1, p2]
                }
            }),
            "trailer change"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
