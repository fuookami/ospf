package fuookami.ospf.kotlin.example.framework_demo.demo1.route_context

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Assignment
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Graph
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.Service

class Aggregation(
    val graph: Graph,
    val services: List<Service>,
    val assignment: Assignment
) {
    fun register(model: LinearMetaModel): Try {
        val subprocesses = arrayListOf(
            { return@arrayListOf assignment.register(model) }
        )

        for (subprocess in subprocesses) {
            when (val result = subprocess()) {
                is Failed -> {
                    return Failed(result.error)
                }

                is Ok -> {}
            }
        }
        return ok
    }
}
