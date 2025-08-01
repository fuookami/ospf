package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.model.*

class Aggregation(
    internal val aircraftModel: AircraftModel,
    formula: Formula,
    totalWeight: TotalWeight,
    torque: Torque,
    internal val horizontalStabilizers: Map<HorizontalStabilizer.Key, HorizontalStabilizer>
) {
    val macRange = MACRange(
        aircraftModel = aircraftModel,
        formula = formula,
        totalWeight = totalWeight
    )

    val longitudinalBalance = LongitudinalBalance(
        aircraftModel = aircraftModel,
        macRange = macRange,
        torque = torque
    )

    val lateralBalance = if (aircraftModel.wideBody) {
        LateralBalance(
            aircraftModel = aircraftModel,
            torque = torque
        )
    } else {
        null
    }

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = longitudinalBalance.register(stowageMode, model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (lateralBalance != null) {
            when (val result = lateralBalance.register(model)) {
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
