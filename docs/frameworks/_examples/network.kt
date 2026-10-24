package docs.examples.network

import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.quantities.quantity.Quantity
import fuookami.ospf.kotlin.quantities.unit.NoneUnit
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.network_scheduling.infrastructure.*
import fuookami.ospf.kotlin.framework.network_scheduling.domain.flow.*
import fuookami.ospf.kotlin.framework.network_scheduling.domain.flow.model.*
import fuookami.ospf.kotlin.framework.network_scheduling.application.service.*
import fuookami.ospf.kotlin.utils.functional.*

private fun <T> Ret<T>.getOrThrow(): T = when (this) {
    is Ok -> value
    is Failed -> error("${error.code}: ${error.message}")
    is Fatal -> error(errors.joinToString { "${it.code}: ${it.message}" })
}

suspend fun runExample(solver: ColumnGenerationSolver) {
    fun q(value: Double) = Quantity(Flt64(value), NoneUnit)
    val source = NetworkNode<Flt64>(NetworkNodeId("source")).getOrThrow()
    val sink = NetworkNode<Flt64>(NetworkNodeId("sink")).getOrThrow()
    val arc = NetworkArc(
        id = NetworkArcId("delivery"), from = source.id, to = sink.id, cost = q(3.0)
    ).getOrThrow()
    val bounds = CapacityBounds(lower = q(0.0), upper = q(3.0)).getOrThrow()
    val commodity = FlowCommodity("goods", listOf(
        SupplyDemand(source.id, NodeBalance(q(2.0)).getOrThrow()).getOrThrow(),
        SupplyDemand(sink.id, NodeBalance(q(-2.0)).getOrThrow()).getOrThrow()
    )).getOrThrow()
    val problem = FlowProblem(
        graph = NetworkGraph(listOf(source, sink), listOf(arc)).getOrThrow(),
        arcs = listOf(FlowArc(arc, bounds).getOrThrow()),
        commodities = listOf(CommodityFlow(commodity, NoneUnit)), zero = Flt64.zero
    )
    val outcome = NetworkFlowApplicationService(solver, Flt64NetworkSchedulingSolverValueAdapter)
        .solve(problem).getOrThrow()
    println(outcome.report)
    val solution = outcome.solution
    if (solution == null) {
        println("No flow; inspect infeasibility or the no-incumbent termination")
    } else {
        println("flows=${solution.flows}, cost=${solution.cost}")
    }
}
