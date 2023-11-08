package fuookami.ospf.kotlin.example.framework_demo.demo1.domain.route_context.service.limits

import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.domain.route_context.model.*

class NodeAssignmentConstraint(
    private val nodes: List<Node>,
    private val assignment: Assignment,
    override val name: String = "node_assignment"
) : Pipeline<LinearMetaModel> {
    override fun invoke(model: LinearMetaModel): Try<Error> {
        for (node in nodes.asSequence().filter(normal)) {
            model.addConstraint(
                assignment.nodeAssignment[node]!! leq UInt64.one,
                "${name}_$node"
            )
        }
        return Ok(success)
    }
}
