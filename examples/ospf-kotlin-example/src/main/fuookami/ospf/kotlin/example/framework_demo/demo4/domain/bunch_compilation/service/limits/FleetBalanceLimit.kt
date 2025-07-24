package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.ShadowPrice
import fuookami.ospf.kotlin.framework.model.ShadowPriceKey
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.model.*

private data class FleetBalanceShadowPriceKey(
    val airport: Airport,
    val aircraftMinorType: AircraftMinorType,
) : ShadowPriceKey(FleetBalanceShadowPriceKey::class) {
    override fun toString() = "Fleet Balance ($airport, $aircraftMinorType)"
}

class FleetBalanceLimit(
    private val fleetBalance: FleetBalance,
    private val coefficient: (Airport, AircraftMinorType) -> Flt64,
    override val name: String = "fleet_balance_limit"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((l, checkPoint) in fleetBalance.limits.withIndex()) {
            when (val result = model.addConstraint(
                fleetBalance.slack[l].polyX geq checkPoint.second.amount,
                "${name}_${l}"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        val poly = MutableLinearPolynomial()
        for ((l, checkPoint) in fleetBalance.limits.withIndex()) {
            poly += coefficient(checkPoint.first.airport, checkPoint.first.aircraftMinorType) * fleetBalance.slack[l]
        }
        when (val result = model.minimize(
            poly,
            "fleet balance")
        ) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }

    override fun extractor(): ShadowPriceExtractor? {
        return { map, args: ShadowPriceArguments ->
            when (args) {
                is TaskShadowPriceArguments -> {
                    if (args.prevTask is FlightTask && args.task == null) {
                        map[FleetBalanceShadowPriceKey(
                            airport = (args.prevTask!! as FlightTask).arr,
                            aircraftMinorType = (args.prevTask!! as FlightTask).aircraft!!.minorType
                        )]?.price ?: Flt64.zero
                    } else {
                        Flt64.zero
                    }
                }

                else -> {
                    Flt64.zero
                }
            }
        }
    }

    override fun refresh(
        map: ShadowPriceMap,
        model: AbstractLinearMetaModel,
        shadowPrices: List<Flt64>
    ): Try {
        for ((l, j) in model.indicesOfConstraintGroup(name)!!.withIndex()) {
            val checkPoint = fleetBalance.limits[l].first
            map.put(
                ShadowPrice(
                    key = FleetBalanceShadowPriceKey(checkPoint.airport, checkPoint.aircraftMinorType),
                    price = shadowPrices[j]
                )
            )
        }

        return ok
    }
}
