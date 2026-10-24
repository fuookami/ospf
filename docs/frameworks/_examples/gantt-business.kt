@file:OptIn(kotlin.time.ExperimentalTime::class)
package docs.examples.gantt.business

import kotlin.time.Instant
import kotlin.time.Duration.Companion.hours
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.quantities.quantity.Quantity
import fuookami.ospf.kotlin.quantities.unit.NoneUnit
import fuookami.ospf.kotlin.framework.solver.ColumnGenerationSolver
import fuookami.ospf.kotlin.framework.gantt_scheduling.infrastructure.TimeRange
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.task.model.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.activity.model.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.application.service.*

private data class Task(val value: String) : TaskId
private data class Resource(val value: String) : ExecutorId

fun makeProblem(): ActivitySchedulingProblem<Flt64> {
    val origin = Instant.parse("2026-10-22T00:00:00Z")
    fun at(hour: Int) = origin + hour.hours
    fun q(value: Double) = Quantity(Flt64(value), NoneUnit)
    val machine = Resource("machine")
    // #region input
    val resource = ActivityResource(machine, q(2.0),
        unavailableTimes = listOf(TimeRange(at(1), at(2))),
        capacities = listOf(ActivityCapacity(TimeRange(at(3), at(4)), q(1.0))))
    val task = Activity(Task("a"), listOf(ActivityMode("normal", 2.hours,
        listOf(at(0), at(2), at(4)), listOf(ActivityResourceUse(machine, q(2.0))), q(0.0))))
    // #endregion input
    // #region rule
    val lateStartFee: (ActivityPlacement<Flt64>) -> fuookami.ospf.kotlin.utils.functional.Ret<Quantity<Flt64>> = { placement ->
        fuookami.ospf.kotlin.utils.functional.Ok(q(if (placement.key.start >= at(4)) 2.0 else 0.0))
    }
    return ActivitySchedulingProblem(horizon = TimeRange(at(0), at(6)), activities = listOf(task),
        resources = listOf(resource), costUnit = NoneUnit, additionalCost = lateStartFee)
    // #endregion rule
}

// #region entry
suspend fun runBusiness(solver: ColumnGenerationSolver) =
    ActivitySchedulingApplicationService(solver, schedulingSolverValueAdapter).solve(makeProblem())
// #endregion entry
