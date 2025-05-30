package fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.model.EdgeBandwidth
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Edge
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.from
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.normal

class BandwidthCostObjective(
    private val edges: List<Edge>,
    private val edgeBandwidth: EdgeBandwidth,
    override val name: String = "bandwidth_cost"
) : Pipeline<LinearMetaModel> {
    override fun invoke(model: LinearMetaModel): Try {
        model.minimize(
            sum(edges.filter(from(normal))) { it.costPerBandwidth * edgeBandwidth.bandwidth[it] },
            "bandwidth cost"
        )
        return ok
    }
}
