package fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.Aggregation
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits.BandwidthCostObjective
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits.DemandConstraint
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits.EdgeBandwidthConstraint
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.service.limits.ServiceCapacityConstraint
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Assignment
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Graph
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Service

class PipelineListGenerator(
    private val aggregation: Aggregation,
    private val graph: Graph,
    private val services: List<Service>,
    private val assignment: Assignment,
) {
    operator fun invoke(): Ret<PipelineList<LinearMetaModel>> {
        val list = ArrayList<Pipeline<LinearMetaModel>>()

        list.add(EdgeBandwidthConstraint(graph.edges, services, assignment, aggregation.edgeBandwidth))
        list.add(DemandConstraint(graph.nodes, aggregation.nodeBandwidth))
        list.add(ServiceCapacityConstraint(graph.nodes, services, assignment, aggregation.serviceBandwidth))

        list.add(BandwidthCostObjective(graph.edges, aggregation.edgeBandwidth))

        return Ok(list)
    }
}
