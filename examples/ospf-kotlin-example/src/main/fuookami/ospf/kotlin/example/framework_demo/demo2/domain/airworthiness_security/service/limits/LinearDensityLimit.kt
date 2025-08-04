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

class LinearDensityLimit(
    private val aircraftModel: AircraftModel,
    private val linearDensity: LinearDensity,
    private val positions: List<Position>,
    override val name: String = "linear_density_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (line in linearDensity.limitLines) {
            if (line.positions.none { it.status.available }) {
                continue
            }

            val poly = MutableLinearPolynomial()
            for (position in line.positions) {
                val j = positions.indexOf(position)
                poly += linearDensity.linearDensity[j].to(aircraftModel.linearDensityUnit)!!.value
            }
            when (val result = model.addConstraint(
                Quantity(poly, aircraftModel.linearDensityUnit) leq line.zone.maxLinearDensity,
                "${name}_${line.zone.name}_${line.arm.value}"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }
}
