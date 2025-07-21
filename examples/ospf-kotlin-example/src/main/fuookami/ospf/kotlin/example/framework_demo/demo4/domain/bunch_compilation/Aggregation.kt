package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation

import fuookami.ospf.kotlin.framework.gantt_scheduling.infrastructure.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.bunch_compilation.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.bunch_compilation.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.rule.model.*

class Aggregation(
    timeWindow: TimeWindow,
    recoveryNeededAircrafts: List<Aircraft>,
    recoveryNeededFlightTasks: List<FlightTask>,
    val originBunches: List<FlightTaskBunch>,
    val flows: List<Flow>,
    val links: LinkMap
): BunchCompilationAggregation<FlightTaskBunch, FlightTask, Aircraft, FlightTaskAssignment>(
    tasks = recoveryNeededFlightTasks,
    executors = recoveryNeededAircrafts
) {
    val taskTime = BunchSchedulingTaskTime(
        timeWindow = timeWindow,
        tasks = recoveryNeededFlightTasks,
        compilation = compilation
    )

    val flow = BunchSchedulingConnectionResourceUsage(
        timeWindow = timeWindow,
        resources = flows,
        name = "flow"
    )
}
