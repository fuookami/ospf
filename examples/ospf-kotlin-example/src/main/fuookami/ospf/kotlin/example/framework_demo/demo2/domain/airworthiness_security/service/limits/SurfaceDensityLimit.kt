package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model.*

class SurfaceDensityLimit(
    private val surfaceDensity: SurfaceDensity,
    private val positions: List<Position>,
    override val name: String = "surface_density_limit",
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (limitZone in surfaceDensity.limitsZones) {
            for ((j, position) in positions.withIndex()) {
                if (position.status.unavailable) {
                    continue
                }

                if (position.coordinate.withIntersectionWith(limitZone.frontArm, limitZone.backArm)) {
                    when (val result = model.addConstraint(
                        surfaceDensity.surfaceDensity[j] leq limitZone.maxSurfaceDensity,
                        name = "${name}_${limitZone.name}_${position}"
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
