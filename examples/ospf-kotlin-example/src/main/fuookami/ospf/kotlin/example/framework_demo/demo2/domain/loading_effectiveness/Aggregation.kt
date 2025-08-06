package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.PositionPair
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.model.*

class Aggregation(
    aircraftModel: AircraftModel,
    stowageMode: StowageMode,
    internal val flight: Flight,
    internal val items: List<Item>,
    internal val positions: List<Position>,
    val trailers: List<Trailer>,
    internal val stowage: Stowage,
    load: Load
){
    companion object {
        private fun combineAdjacentByLoadingOrder(positions: List<Position>): List<PositionPair> {
            TODO("not implemented yet")
        }

        private fun combineByLoadingOrder(positions: List<Position>): List<PositionPair> {
            TODO("not implemented yet")
        }
    }

    val sources = items.mapNotNull { it.source }.distinct()
    val destinations = items.mapNotNull {
        if (it.destination != flight.arrival) {
            it.destination
        } else {
            null
        }
    }.distinct()
    val adjacentPositions = combineAdjacentByLoadingOrder(positions)
    val orderedPositions = combineByLoadingOrder(positions)

    val adviceLoading = when (stowageMode) {
        StowageMode.Predistribution -> {
            AdviceLoading(
                aircraftModel = aircraftModel,
                positions = positions,
                load = load
            )
        }

        StowageMode.FullLoad, StowageMode.WeightRecommendation -> {
            null
        }
    }

    val transferAdjacentLoading = TransferAdjacentLoading(
        adjacentPositions = adjacentPositions,
        sources = sources,
        destinations = destinations,
        load = load
    )

    val sequentialLoading = when (stowageMode) {
        StowageMode.Predistribution -> {
            SequentialLoading(
                items = items,
                positions = positions,
                orderedPositions = orderedPositions,
                stowage = stowage
            )
        }

        StowageMode.FullLoad, StowageMode.WeightRecommendation -> {
            null
        }
    }

    val trailerLoading = when (stowageMode) {
        StowageMode.FullLoad -> {
            TrailerLoading(
                items = items,
                positions = positions,
                adjacentPositions = adjacentPositions,
                trailers = trailers,
                stowage = stowage,
                load = load
            )
        }

        StowageMode.Predistribution, StowageMode.WeightRecommendation -> {
            null
        }
    }

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = transferAdjacentLoading.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (adviceLoading != null) {
            when (val result = adviceLoading.register(model)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        if (sequentialLoading != null) {
            when (val result = sequentialLoading.register(model)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        if (trailerLoading != null) {
            when (val result = trailerLoading.register(model)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    fun registerForBendersMP(
        model: AbstractLinearMetaModel
    ): Try {
        TODO("not implemented yet")
    }

    fun registerForBendersSP(
        model: AbstractLinearMetaModel,
        solution: List<Flt64>
    ): Try {
        TODO("not implemented yet")
    }

    private fun flushForBendersSP(
        model: AbstractLinearMetaModel,
        solution: List<Flt64>
    ): Try {
        TODO("not implemented yet")
    }
}
