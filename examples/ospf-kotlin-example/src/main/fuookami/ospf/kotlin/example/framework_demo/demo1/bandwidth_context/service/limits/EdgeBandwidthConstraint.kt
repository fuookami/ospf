package fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.model.*

class EdgeBandwidthConstraint(
    private val edges: List<Edge>,
    private val services: List<Service>,
    private val assignment: Assignment,
    private val edgeBandwidth: EdgeBandwidth,
    override val name: String = "edge_bandwidth_constraint"
) : Pipeline<LinearMetaModel> {
    override fun invoke(model: LinearMetaModel): Try {
        val y = edgeBandwidth.y
        val assignment = assignment.serviceAssignment

        for (edge in edges.filter(from(normal))) {
            for (service in services) {
                model.addConstraint(
                    edge.maxBandwidth * (UInt64.one - assignment[service]) + y[edge, service] leq edge.maxBandwidth,
                    "${name}_($edge,$service)"
                )
            }
        }

        for (edge in edges.filter(from(normal))) {
            model.addConstraint(
                sum(y[edge, _a]) leq edge.maxBandwidth,
                "${name}_($edge)"
            )
        }

        return ok
    }
}
