package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class Torque(
    private val aircraftModel: AircraftModel,
    private val fuselage: Fuselage,
    private val fuel: Map<FlightPhase, FuelConstant>,
    private val formula: Formula,
    private val positions: List<Position>,
    private val load: Load
) {
    lateinit var longitudinalTorque: Map<FlightPhase, QuantityLinearIntermediateSymbol>
    lateinit var lateralTorque: QuantityLinearIntermediateSymbol
    lateinit var clim: QuantityLinearIntermediateSymbol
    lateinit var index: Map<FlightPhase, QuantityLinearIntermediateSymbol>

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::longitudinalTorque.isInitialized) {
            longitudinalTorque = FlightPhase.entries.associateWith { phase ->
                val poly = MutableLinearPolynomial()
                for ((j, _) in positions.withIndex()) {
                    poly += load.loadEstimateLongitudinalTorque[j].to(aircraftModel.torqueUnit)!!.value
                }
                when (phase) {
                    FlightPhase.TakeOff, FlightPhase.Landing -> {
                        poly += fuel[phase]!!.weight.to(aircraftModel.torqueUnit)!!.value
                    }

                    FlightPhase.ZeroFuel -> {}
                }
                poly += (fuselage.dow * fuselage.balancedArm).to(aircraftModel.torqueUnit)!!.value
                poly += fuselage.liferaft?.let {
                    val arm = formula.arm(it.index, it.weight)
                    (it.weight * arm).to(aircraftModel.torqueUnit)!!.value
                } ?: Flt64.zero
                Quantity(
                    LinearExpressionSymbol(
                        poly,
                        name = "index_${phase.name.lowercase()}"
                    ),
                    aircraftModel.torqueUnit
                )
            }
        }
        longitudinalTorque.values.forEach {
            when (val result = model.add(it)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        if (aircraftModel.wideBody) {
            if (!::lateralTorque.isInitialized) {
                val poly = MutableLinearPolynomial()
                for ((j, _) in positions.withIndex()) {
                    poly += load.loadLateralTorque[j].to(aircraftModel.torqueUnit)!!.value
                }
                lateralTorque = Quantity(
                    LinearExpressionSymbol(
                        poly,
                        name = "lateral_torque"
                    ),
                    aircraftModel.torqueUnit
                )
            }
            when (val result = model.add(lateralTorque)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }

            if (!::clim.isInitialized) {
                val poly = MutableLinearPolynomial()
                for ((j, _) in positions.withIndex()) {
                    poly += load.loadCLIM[j].to(aircraftModel.torqueUnit)!!.value
                }
                clim = Quantity(
                    LinearExpressionSymbol(
                        poly,
                        name = "clim"
                    ),
                    aircraftModel.torqueUnit
                )
            }
            when (val result = model.add(clim)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        if (!::index.isInitialized) {
            index = FlightPhase.entries.associateWith { phase ->
                val poly = MutableLinearPolynomial()
                for ((j, _) in positions.withIndex()) {
                    poly += load.loadIndex[j].to(aircraftModel.torqueUnit)!!.value
                }
                when (phase) {
                    FlightPhase.TakeOff, FlightPhase.Landing -> {
                        poly += fuel[phase]!!.index.to(aircraftModel.torqueUnit)!!.value
                    }

                    FlightPhase.ZeroFuel -> {}
                }
                poly += fuselage.doi.to(aircraftModel.lengthUnit)!!.value
                poly += fuselage.liferaft?.index?.let {
                    it.to(aircraftModel.torqueUnit)!!.value
                } ?: Flt64.zero
                Quantity(
                    LinearExpressionSymbol(
                        poly,
                        name = "index_${phase.name.lowercase()}"
                    ),
                    aircraftModel.weightUnit
                )
            }
        }
        index.values.forEach {
            when (val result = model.add(it)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }
}
