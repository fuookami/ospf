package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.recommended_weight_equalization.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class RecommendedWeightEqualizationLimit(
    private val aircraftModel: AircraftModel,
    private val positions: List<Position>,
    private val load: Load,
    override val name: String = "recommended_weight_equalization_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((j1, position1) in positions.withIndex()) {
            if (!position1.status.recommendedWeightNeeded) {
                continue
            }

            for ((j2, position2) in positions.withIndex()) {
                if (j2 <= j1 || !position2.status.recommendedWeightNeeded) {
                    continue
                }

                when (val result = model.addConstraint(
                    load.z[j1] leq load.z[j2] + position1.mlw.mlw * load.actualLoaded[j2],
                    name = "${name}_${position1}_${position2}"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }

                when (val result = model.addConstraint(
                    load.z[j2] leq load.z[j1] + position2.mlw.mlw * load.actualLoaded[j2],
                    name = "${name}_${position2}_${position1}"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }
        }

        return ok
    }
}
