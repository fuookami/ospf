package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.service.limits

import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model.*

class CumulativeLoadWeightLimit(
    private val aircraftModel: AircraftModel,
    private val maxCumulativeLoadWeight: MaxCumulativeLoadWeight,
    private val positions: List<Position>,
    private val load: Load,
    override val name: String = "cumulative_load_weight_limit",
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (checkPoint in maxCumulativeLoadWeight.checkPoints) {
            if (checkPoint.parts.none { it.position.status.available }) {
                continue
            }

            val poly = MutableLinearPolynomial()
            for (part in checkPoint.parts) {
                val j = positions.indexOf(part.position)
                poly += (part.weight * load.estimateLoadWeight[j]).to(aircraftModel.weightUnit)!!.value
            }
            when (val result = model.addConstraint(
                poly leq checkPoint.maxSum.to(aircraftModel.weightUnit)!!.value,
                name = "${name}_${checkPoint.zone.name}_${checkPoint.toArm.value}"
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
