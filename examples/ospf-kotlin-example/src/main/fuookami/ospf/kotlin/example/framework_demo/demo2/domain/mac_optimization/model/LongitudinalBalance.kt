package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.model

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*

class LongitudinalBalance(
    private val aircraftModel: AircraftModel,
    private val macRange: MACRange,
    private val torque: Torque
) {
    lateinit var slack: Map<MACRange.Type, QuantityLinearIntermediateSymbol>

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        if (!::slack.isInitialized) {
            slack = when (stowageMode) {
                StowageMode.FullLoad -> {
                    // 全载模式判定 A、B、C 区间
                    MACRange.Type.entries.associateWithNotNull { type ->
                        when (type) {
                            MACRange.Type.OPT, MACRange.Type.C -> {
                                null
                            }

                            else -> {
                                Quantity(
                                    SlackRangeFunction(
                                        x = LinearPolynomial(torque.longitudinalTorque[FlightPhase.TakeOff]!!.to(aircraftModel.torqueUnit)!!.value),
                                        lb = LinearPolynomial(macRange.lhsPoints.find { it.type == type }!!.torque.to(aircraftModel.torqueUnit)!!.value),
                                        ub = LinearPolynomial(macRange.rhsPoints.find { it.type == type }!!.torque.to(aircraftModel.torqueUnit)!!.value),
                                        name = "longitudinal_balance_slack_${type.name}"
                                    ),
                                    aircraftModel.torqueUnit
                                )
                            }
                        }
                    }
                }

                StowageMode.Predistribution -> {
                    // 预配载模式判定最优重心
                    mapOf(
                        MACRange.Type.OPT to Quantity(
                            SlackFunction(
                                x = LinearPolynomial(torque.longitudinalTorque[FlightPhase.TakeOff]!!.to(aircraftModel.torqueUnit)!!.value),
                                y = LinearPolynomial(macRange.optPoint.torque.to(aircraftModel.torqueUnit)!!.value),
                                name = "longitudinal_balance_slack_opt"
                            ),
                            aircraftModel.torqueUnit
                        )
                    )
                }

                StowageMode.WeightRecommendation -> {
                    emptyMap()
                }
            }
        }
        slack.values.forEach {
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
