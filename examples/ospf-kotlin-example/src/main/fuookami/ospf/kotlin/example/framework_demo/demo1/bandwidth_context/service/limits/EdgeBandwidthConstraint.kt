package fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.model.EdgeBandwidth
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Assignment
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Edge
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Service
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.from
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.normal

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
                    (UInt64.one - assignment[service]) * edge.maxBandwidth + y[edge, service] leq edge.maxBandwidth,
                    "${name}_($edge,$service)"
                )
            }
        }
        return ok
    }
}
