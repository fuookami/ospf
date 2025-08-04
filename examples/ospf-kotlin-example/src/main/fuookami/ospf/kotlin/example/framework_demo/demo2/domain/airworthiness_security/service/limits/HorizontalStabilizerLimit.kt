package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.*

class HorizontalStabilizerLimit(
    private val horizontalStabilizers: Map<HorizontalStabilizer.Key, HorizontalStabilizer>,
    private val stowageMode: StowageMode,
    override val name: String = "horizontal_stabilizer_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((key, horizontalStabilizer) in horizontalStabilizers) {
            if (stowageMode == StowageMode.WeightRecommendation && horizontalStabilizer.limit.warnMaxTrim != null) {
                when (val result = model.addConstraint(
                    horizontalStabilizer.trim leq horizontalStabilizer.limit.warnMaxTrim!!,
                    name = "${name}_${key}_ub"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            } else if (horizontalStabilizer.limit.maxTrim != null) {
                when (val result = model.addConstraint(
                    horizontalStabilizer.trim leq horizontalStabilizer.limit.maxTrim!!,
                    name = "${name}_${key}_ub"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }

            if (stowageMode == StowageMode.WeightRecommendation && horizontalStabilizer.limit.warnMinTrim != null) {
                when (val result = model.addConstraint(
                    horizontalStabilizer.trim leq horizontalStabilizer.limit.warnMinTrim!!,
                    name = "${name}_${key}_lb"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
                when (val result = model.addConstraint(
                    horizontalStabilizer.trim geq horizontalStabilizer.limit.minTrim!!,
                    name = "${name}_${key}_lb"
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
