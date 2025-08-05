package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class TransferAdjacentLoading(
    private val adjacentPositions: List<PositionPair>,
    private val sources: List<FlightNo>,
    private val destinations: List<IATA>,
    private val load: Load
) {
    lateinit var sameSourceAdjacent: LinearIntermediateSymbols2
    lateinit var sameDestinationAdjacent: LinearIntermediateSymbols2

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::sameSourceAdjacent.isInitialized) {
            sameSourceAdjacent = LinearIntermediateSymbols2("same_source_adjacent", Shape2(sources.size, adjacentPositions.size)) { _, v ->
                val source = sources[v[0]]
                val position1 = adjacentPositions[v[1]].first
                val position2 = adjacentPositions[v[1]].second

                val loadAmount1 = load.loadAmountOf(position1) { item ->
                    item.source == source
                }
                val loadAmount2 = load.loadAmountOf(position2) { item ->
                    item.source == source
                }

                if (position1.status.stowageNeeded || position1.status.adjustmentNeeded) {
                    IfFunction(
                        (loadAmount1 + loadAmount2) geq Flt64.two,
                        name = "same_source_adjacent_${source}_${position1}_${position2}",
                    )
                } else if (loadAmount1.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "same_source_adjacent_${source}_${position1}_${position2}"
                    )
                } else {
                    LinearExpressionSymbol(
                        LinearPolynomial(loadAmount1),
                        name = "same_source_adjacent_${source}_${position1}_${position2}"
                    )
                }
            }
        }
        when (val result = model.add(sameSourceAdjacent)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::sameDestinationAdjacent.isInitialized) {
            sameDestinationAdjacent = LinearIntermediateSymbols2("same_destination_adjacent", Shape2(destinations.size, adjacentPositions.size)) { _, v ->
                val destination = destinations[v[0]]
                val position1 = adjacentPositions[v[1]].first
                val position2 = adjacentPositions[v[1]].second

                val loadAmount1 = load.loadAmountOf(position1) { item ->
                    item.destination == destination
                }
                val loadAmount2 = load.loadAmountOf(position2) { item ->
                    item.destination == destination
                }

                if (position1.status.stowageNeeded || position1.status.adjustmentNeeded) {
                    IfFunction(
                        (loadAmount1 + loadAmount2) geq Flt64.two,
                        name = "same_destination_adjacent_${destination}_${position1}_${position2}",
                    )
                } else if (loadAmount1.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "same_destination_adjacent_${destination}_${position1}_${position2}"
                    )
                } else {
                    LinearExpressionSymbol(
                        LinearPolynomial(loadAmount1),
                        name = "same_destination_adjacent_${destination}_${position1}_${position2}"
                    )
                }
            }
        }
        when (val result = model.add(sameDestinationAdjacent)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
