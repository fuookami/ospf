package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.service.limits

import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model.*

class UnsymmetricalLinearDensityLimit(
    private val aircraftModel: AircraftModel,
    private val maxUnsymmetricalLinearDensity: MaxUnsymmetricalLinearDensity,
    private val linearDensity: LinearDensity,
    private val positions: List<Position>,
    override val name: String = "unsymmetrical_linear_density_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (zone in maxUnsymmetricalLinearDensity.limitZones) {
            for (line in zone.lines) {
                if (line.positions.none { it.status.available }) {
                    continue
                }

                assert(line.positions.size == 2)
                for ((l, limit) in zone.limits.withIndex()) {
                    val poly = MutableLinearPolynomial()
                    if (limit.leftCoefficient != null) {
                        val j = positions.indexOf(line.positions.find { it.coordinate.onLeft })
                        poly += limit.leftCoefficient * linearDensity.linearDensity[j].to(aircraftModel.linearDensityUnit)!!.value
                    }
                    if (limit.rightCoefficient != null) {
                        val j = positions.indexOf(line.positions.find { it.coordinate.onRight })
                        poly += limit.rightCoefficient * linearDensity.linearDensity[j].to(aircraftModel.linearDensityUnit)!!.value
                    }
                    when (val result = model.addConstraint(
                        Quantity(poly, aircraftModel.linearDensityUnit) leq limit.maxSum,
                        name = "${name}_${line.zone.name}_${line.arm.value}_${l}"
                    )) {
                        is Ok -> {}

                        is Failed -> {
                            return Failed(result.error)
                        }
                    }
                }
            }
        }

        return ok
    }
}
