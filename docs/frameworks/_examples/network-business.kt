package docs.examples.network.business

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

fun makeProblem(dockCapacity: Double = 2.0): FlowProblem<Flt64> {
    fun q(value: Double) = Quantity(Flt64(value), NoneUnit)
    // #region input
    val source = NetworkNode<Flt64>(NetworkNodeId("source")).getOrThrow()
    val sink = NetworkNode<Flt64>(NetworkNodeId("sink")).getOrThrow()
    val arc = NetworkArc(id = NetworkArcId("delivery"), from = source.id, to = sink.id, cost = q(3.0)).getOrThrow()
    val bounds = CapacityBounds(q(0.0), q(3.0)).getOrThrow()
    val goods = FlowCommodity("goods", listOf(
        SupplyDemand(source.id, NodeBalance(q(2.0)).getOrThrow()).getOrThrow(),
        SupplyDemand(sink.id, NodeBalance(q(-2.0)).getOrThrow()).getOrThrow())).getOrThrow()
    // #endregion input
    // #region rule
    val dock = FlowCapacity("dock", CapacityBounds(q(0.0), q(dockCapacity)).getOrThrow(),
        usage = mapOf(FlowKey("goods", arc.id) to q(1.0)))
    return FlowProblem(graph = NetworkGraph(listOf(source, sink), listOf(arc)).getOrThrow(),
        arcs = listOf(FlowArc(arc, bounds).getOrThrow()), commodities = listOf(CommodityFlow(goods, NoneUnit)),
        zero = Flt64.zero, resources = listOf(dock))
    // #endregion rule
}

fun makeSlackProblem(): FlowProblem<Flt64> {
    fun q(value: Double) = Quantity(Flt64(value), NoneUnit)
    val base = makeProblem()
    val source = NetworkNodeId("source")
    val sink = NetworkNodeId("sink")
    val arc = NetworkArc(id = NetworkArcId("delivery"), from = source, to = sink, cost = q(1.0)).getOrThrow()
    val goods = FlowCommodity("goods", listOf(
        SupplyDemand(source, NodeBalance(q(3.0)).getOrThrow()).getOrThrow(),
        SupplyDemand(sink, NodeBalance(q(-4.0)).getOrThrow()).getOrThrow())).getOrThrow()
    // #region slack
    val slacks = listOf(
        FlowSlack("goods", source, unusedSupply = q(1.0), unmetDemand = q(0.0), penalty = q(5.0)),
        FlowSlack("goods", sink, unusedSupply = q(0.0), unmetDemand = q(2.0), penalty = q(7.0)))
    return FlowProblem(graph = NetworkGraph(base.graph.nodes, listOf(arc)).getOrThrow(),
        arcs = listOf(FlowArc(arc, CapacityBounds(q(0.0), q(2.0)).getOrThrow()).getOrThrow()),
        commodities = listOf(CommodityFlow(goods, NoneUnit)), zero = Flt64.zero, slacks = slacks)
    // #endregion slack
}

// #region entry
suspend fun runBusiness(solver: ColumnGenerationSolver, dockCapacity: Double = 2.0) =
    NetworkFlowApplicationService(solver, Flt64NetworkSchedulingSolverValueAdapter).solve(makeProblem(dockCapacity))
// #endregion entry
