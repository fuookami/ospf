@file:OptIn(kotlin.time.ExperimentalTime::class)
package docs.examples.gantt

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
import fuookami.ospf.kotlin.utils.functional.*

private data class Task(val value: String) : TaskId
private data class Resource(val value: String) : ExecutorId

suspend fun runExample(solver: ColumnGenerationSolver) {
    val origin = Instant.parse("2026-10-22T00:00:00Z")
    fun at(hour: Int) = origin + hour.hours
    fun q(value: Double) = Quantity(Flt64(value), NoneUnit)
    val a = Task("a")
    val b = Task("b")
    val machine = Resource("machine")
    val alternate = Resource("alternate")
    val problem = ActivitySchedulingProblem(
        horizon = TimeRange(at(0), at(6)), costUnit = NoneUnit,
        activities = listOf(
            Activity(a, listOf(
                ActivityMode("fast", 2.hours, listOf(at(0), at(2)), listOf(ActivityResourceUse(machine, q(1.0))), q(3.0)),
                ActivityMode("slow", 3.hours, listOf(at(0), at(2)), listOf(ActivityResourceUse(alternate, q(1.0))), q(1.0))
            )),
            Activity(b, listOf(ActivityMode("next", 2.hours, listOf(at(0), at(2)), listOf(ActivityResourceUse(machine, q(1.0))), q(0.0))))
        ),
        resources = listOf(ActivityResource(machine, q(1.0)), ActivityResource(alternate, q(1.0))),
        dependencies = listOf(ActivityDependency(a, b))
    )
    when (val result = ActivitySchedulingApplicationService(solver, schedulingSolverValueAdapter).solve(problem)) {
        is Ok -> {
            val outcome = result.value
            println(outcome.report)
            val schedule = outcome.schedule
            if (schedule == null) {
                println("No schedule; retain the report instead of inventing a plan")
            } else {
                // #region result
                println("cost=${schedule.cost}, cancelled=${schedule.cancelled}, candidates=${schedule.candidateScope.size}")
                for (p in schedule.placements) {
                    println("task=${p.key.task}, mode=${p.key.mode}, start=${p.time.start}, end=${p.time.end}")
                    p.reservations.forEach { println("resource=${it.resource}, time=${it.time}, quantity=${it.quantity}") }
                }
                println("executions=${schedule.executions}, inventory=${schedule.materialInventory}")
                check(schedule.placements.size == 2 && schedule.cancelled.isEmpty())
                // #endregion result
            }
        }
        is Failed -> error("${result.error.code}: ${result.error.message}")
        is Fatal -> error(result.errors.joinToString { "${it.code}: ${it.message}" })
    }
}
