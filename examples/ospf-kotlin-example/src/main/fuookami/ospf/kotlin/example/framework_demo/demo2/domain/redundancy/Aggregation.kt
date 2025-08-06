package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.model.*

class Aggregation(
    internal val aircraftModel: AircraftModel,
    flight: Flight,
    items: List<Item>,
    positions: List<Position>,
    stowage: Stowage,
    load: Load,
    payload: Payload
) {
    val redundancy = Redundancy(
        aircraftModel = aircraftModel,
        flight = flight,
        items = items,
        positions = positions,
        stowage = stowage,
        load = load,
        payload = payload
    )

    val experimentalLongitudinalBalance = ExperimentalLongitudinalBalance(
        aircraftModel = aircraftModel,
        positions = positions,
        load = load,
        payload = payload,
        redundancy = redundancy
    )

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = redundancy.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = experimentalLongitudinalBalance.register(model)) {
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
