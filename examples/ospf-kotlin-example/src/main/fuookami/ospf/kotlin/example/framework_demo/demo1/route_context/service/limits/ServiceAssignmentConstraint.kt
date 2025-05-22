package fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Assignment
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Service

class ServiceAssignmentConstraint(
    private val services: List<Service>,
    private val assignment: Assignment,
    override val name: String = "service_assignment"
) : Pipeline<LinearMetaModel> {
    override fun invoke(model: LinearMetaModel): Try {
        for (service in services) {
            model.addConstraint(
                assignment.serviceAssignment[service] leq 1,
                "${name}_$service"
            )
        }
        return ok
    }
}
