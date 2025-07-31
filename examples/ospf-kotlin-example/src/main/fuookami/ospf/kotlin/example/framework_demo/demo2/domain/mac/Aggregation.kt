package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.register
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.MAC
import fuookami.ospf.kotlin.utils.functional.Failed.Companion.invoke


class Aggregation(
    aircraftModel: AircraftModel,
    fuselage: Fuselage,
    fuel: Map<FlightPhase, FuelConstant>,
    formula: Formula,
    positions: List<Position>,
    load: Load,
    totalWeight: TotalWeight,
    horizontalStabilizers: HashMap<HorizontalStabilizer.Key, Pair<List<HorizontalStabilizer.Point>, HorizontalStabilizer.Limit>>
) {
    val torque = Torque(
        aircraftModel = aircraftModel,
        fuselage = fuselage,
        fuel = fuel,
        formula = formula,
        positions = positions,
        load = load
    )

    val mac = MAC(
        aircraftModel = aircraftModel,
        formula = formula,
        totalWeight = totalWeight,
        torque = torque
    )

    val horizontalStabilizers = horizontalStabilizers.mapValues {
        HorizontalStabilizer(
            aircraftModel = aircraftModel,
            key = it.key,
            points = it.value.first,
            limit = it.value.second,
            totalWeight = totalWeight,
            mac = mac
        )
    }

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = torque.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = mac.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        horizontalStabilizers.values.forEach {
            when (val result = it.register(stowageMode, model)) {
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
