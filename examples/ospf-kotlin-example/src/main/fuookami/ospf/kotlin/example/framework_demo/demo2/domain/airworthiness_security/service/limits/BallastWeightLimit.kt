package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class BallastWeightLimit(
    private val ballast: Ballast,
    override val name: String = "ballast_weight_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        if (ballast.ballastPositions.none { it.status.available } || ballast.minBallastWeight == null) {
            return ok
        }

        when (val result = model.addConstraint(
            ballast.ballastWeight geq ballast.minBallastWeight!!,
            "ballast_weight_limit"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
