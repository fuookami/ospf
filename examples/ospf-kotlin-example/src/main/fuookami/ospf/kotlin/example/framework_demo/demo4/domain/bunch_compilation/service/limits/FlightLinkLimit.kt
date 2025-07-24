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
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.rule.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.model.*

private data class FlightLinkShadowPriceKey(
    val link: Link
) : ShadowPriceKey(FlightLinkShadowPriceKey::class) {
    override fun toString() = "Link ($link)"
}

class FlightLinkLimit(
    private val flightLink: FlightLink,
    private val coefficient: (Link) -> Flt64,
    override val name: String = "flight_link_limit"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((k, _) in flightLink.links.withIndex()) {
            when (val result = model.addConstraint(
                flightLink.slack[k].polyX geq Flt64.one,
                "${name}_${k}"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        val poly = MutableLinearPolynomial()
        for ((k, link) in flightLink.links.withIndex()) {
            poly += coefficient(link) * flightLink.slack[k]
        }

        when (val result = model.minimize(
            poly,
            "link"
        )) {
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
                    if (args.prevTask is FlightTask && args.task is FlightTask) {
                        val link = flightLink.links.find { it.prevTask == (args.prevTask!! as FlightTask).originTask
                                && it.succTask == (args.task!! as FlightTask).originTask
                        }
                        if (link != null) {
                            map[FlightLinkShadowPriceKey(link)]?.price ?: Flt64.zero
                        } else {
                            Flt64.zero
                        }
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
        for ((k, j) in model.indicesOfConstraintGroup(name)!!.withIndex()) {
            map.put(
                ShadowPrice(
                    key = FlightLinkShadowPriceKey(flightLink.links[k]),
                    price = shadowPrices[j]
                )
            )
        }

        return ok
    }
}
