package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.model.*

class Aggregation(
    internal val aircraftModel: AircraftModel,
    internal val mainDeck: Deck,
    internal val items: List<Item>,
    internal val positions: List<Position>,
    internal val stowage: Stowage,
    internal val load: Load,
    internal val ballast: Ballast?
) {
    val divideEmptyLoading = DivideEmptyLoading(
        positions = positions,
        load = load
    )

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = divideEmptyLoading.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
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
