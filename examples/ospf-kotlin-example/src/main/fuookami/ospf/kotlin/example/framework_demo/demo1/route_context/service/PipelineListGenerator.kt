package fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.service

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.Aggregation
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.service.limits.NodeAssignmentConstraint
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.service.limits.ServiceAssignmentConstraint
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.service.limits.ServiceCostObjective

class PipelineListGenerator(
    private val aggregation: Aggregation
) {
    operator fun invoke(): Ret<PipelineList<LinearMetaModel>> {
        val list = ArrayList<Pipeline<LinearMetaModel>>()

        list.add(NodeAssignmentConstraint(aggregation.graph.nodes, aggregation.assignment))
        list.add(ServiceAssignmentConstraint(aggregation.services, aggregation.assignment))

        list.add(ServiceCostObjective(aggregation.services, aggregation.assignment))

        return Ok(list)
    }
}
