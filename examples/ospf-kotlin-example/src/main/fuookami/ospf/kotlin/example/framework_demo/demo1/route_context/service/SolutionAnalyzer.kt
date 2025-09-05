package fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.service

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.model.*

class SolutionAnalyzer(
    private val aggregation: Aggregation
) {
    operator fun invoke(model: LinearMetaModel, result: List<Flt64>): Ret<Map<Service, Node>> {
        val solution = HashMap<Service, Node>()

        for (token in model.tokens.tokens) {
            if (token.variable.belongsTo(aggregation.assignment.x)) {
                if (result[token.solverIndex] eq Flt64.one) {
                    val vector = token.variable.vectorView
                    solution[aggregation.services.find { it.index == vector[1] }!!] =
                        aggregation.graph.nodes.find { it.index == vector[0] }!!
                }
            }
        }

        return Ok(solution)
    }
}
