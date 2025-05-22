package fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.model.ServiceBandwidth
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Assignment
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Node
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Service
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.normal

class ServiceCapacityConstraint(
    private val nodes: List<Node>,
    private val services: List<Service>,
    private val assignment: Assignment,
    private val serviceBandwidth: ServiceBandwidth,
    override val name: String = "service_capacity_constraint"
) : Pipeline<LinearMetaModel> {
    override fun invoke(model: LinearMetaModel): Try {
        val x = assignment.x
        val outFlow = serviceBandwidth.outFlow
        for (node in nodes.filter(normal)) {
            for (service in services) {
                model.addConstraint(
                    service.capacity * (UInt64.one - x[node, service]) + outFlow[node, service] leq service.capacity,
                    "${name}_($node,$service)"
                )
            }
        }
        return ok
    }
}
