package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model.*

class Aggregation(
    internal val aircraftModel: AircraftModel,
    internal val fuselage: Fuselage,
    internal val positions: List<Position>,
    linearDensityLimitZones: List<LinearDensity.LimitZone>,
    surfaceDensityLimitZones: List<SurfaceDensity.LimitZone>,
    val maxZoneLoadWeight: MaxZoneLoadWeight,
    val maxCumulativeLoadWeight: MaxCumulativeLoadWeight,
    val maxUnsymmetricalLinearDensity: MaxUnsymmetricalLinearDensity?,
    maxCLIMPoints: List<MaxCLIM.Point>?,
    minLowPayloadPoints: List<MinLowPayload.Point>,
    envelopeBuilders: (FlightPhase, TotalWeight) -> List<AbstractEnvelope>,
    internal val load: Load,
    internal val payload: Payload,
    internal val totalWeight: TotalWeight,
    internal val ballast: Ballast?,
    internal val torque: Torque,
    internal val horizontalStabilizers: Map<HorizontalStabilizer.Key, HorizontalStabilizer>
) {
    val linearDensity = LinearDensity(
        aircraftModel = aircraftModel,
        limitZones = linearDensityLimitZones,
        load = load,
        positions = positions
    )

    val surfaceDensity = SurfaceDensity(
        aircraftModel = aircraftModel,
        limitsZones = surfaceDensityLimitZones,
        load = load,
        positions = positions
    )

    val maxCLIM = if (aircraftModel.wideBody && !maxCLIMPoints.isNullOrEmpty()) {
        MaxCLIM(
            aircraftModel = aircraftModel,
            points = maxCLIMPoints,
            totalWeight = totalWeight
        )
    } else {
        null
    }

    val minLowPayload = MinLowPayload(
        aircraftModel = aircraftModel,
        points = minLowPayloadPoints,
        totalWeight = totalWeight
    )

    val envelopes = FlightPhase.entries.associateWith { phase ->
        envelopeBuilders(phase, totalWeight)
    }

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = linearDensity.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = surfaceDensity.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (maxCLIM != null) {
            when (val result = maxCLIM.register(model)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        when (val result = minLowPayload.register(model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        envelopes.values.forEach { envelopes ->
            envelopes.forEach { envelope ->
                when (val result = envelope.register(model)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }
        }

        return ok
    }

    fun registerForBendersMP(model: AbstractLinearMetaModel): Try {
        TODO("not implemented yet")
    }

    fun registerForBendersSP(model: AbstractLinearMetaModel, solution: List<Flt64>): Try {
        TODO("not implemented yet")
    }

    private fun flushForBendersSP(model: AbstractLinearMetaModel, solution: List<Flt64>): Try {
        TODO("not implemented yet")
    }
}
