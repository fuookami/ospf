@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import kotlin.time.Instant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class SchedulerEngineTest {
    @Test
    fun shouldPreferLowerQueueDelayWhenQueueWeightDominates() {
        val now = 1_700_000_000_000L
        val scheduler = SchedulerEngine(
            clock = FixedClockPort(now),
            weights = SchedulerWeights(
                costWeight = 0.0,
                deadlineRiskWeight = 0.0,
                queueDelayWeight = 1.0
            )
        )
        val task = task(deadline = null)

        val congestedNode = node(
            nodeId = "congested",
            performanceScore = 1.0,
            pricePerSecond = 1.0,
            parallelUnits = 4,
            availableUnits = 1
        )
        val idleNode = node(
            nodeId = "idle",
            performanceScore = 1.0,
            pricePerSecond = 1.0,
            parallelUnits = 4,
            availableUnits = 4
        )

        val selected = scheduler.chooseNode(task, listOf(congestedNode, idleNode))
        assertNotNull(selected)
        assertEquals("idle", selected.nodeId.value)
    }

    @Test
    fun shouldPreferDeadlineSatisfiedNodeEvenWhenItIsMoreExpensive() {
        val now = 1_700_000_000_000L
        val scheduler = SchedulerEngine(clock = FixedClockPort(now))
        val task = task(deadline = now + 12_000L)

        val cheapButLate = node(
            nodeId = "cheap-late",
            performanceScore = 1.0,
            pricePerSecond = 0.001
        )
        val expensiveButOnTime = node(
            nodeId = "expensive-on-time",
            performanceScore = 2.0,
            pricePerSecond = 10.0
        )

        val selected = scheduler.chooseNode(task, listOf(cheapButLate, expensiveButOnTime))
        assertNotNull(selected)
        assertEquals("expensive-on-time", selected.nodeId.value)
    }

    @Test
    fun shouldPreferLowestDeadlineRiskWhenNoNodeCanMeetDeadline() {
        val now = 1_700_000_000_000L
        val scheduler = SchedulerEngine(clock = FixedClockPort(now))
        val task = task(deadline = now + 5_000L)

        val cheapButVeryLate = node(
            nodeId = "cheap-risky",
            performanceScore = 0.1,
            pricePerSecond = 0.001
        )
        val expensiveButLessLate = node(
            nodeId = "expensive-less-risky",
            performanceScore = 2.0,
            pricePerSecond = 10.0
        )

        val selected = scheduler.chooseNode(task, listOf(cheapButVeryLate, expensiveButLessLate))
        assertNotNull(selected)
        assertEquals("expensive-less-risky", selected.nodeId.value)
    }

    private fun task(deadline: Long?): TaskState =
        TaskState(
            taskId = "task-test",
            requestId = "req-test",
            status = TaskStatus.QUEUED,
            complexity = TaskComplexity.SIMPLE,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            priority = 1,
            deadline = deadline,
            payload = SolvePayload(modelRef = ObjectRef.of(path = "model/test")),
            createdAt = 1_700_000_000_000L,
            updatedAt = 1_700_000_000_000L,
            budgetScope = "budget-test"
        )

    private fun node(
        nodeId: String,
        performanceScore: Double,
        pricePerSecond: Double,
        parallelUnits: Int = 1,
        availableUnits: Int = parallelUnits
    ): NodeState =
        NodeState(
            nodeId = nodeId,
            profile = NodeCapabilityProfile(
                nodeId = nodeId,
                solverType = "test",
                performanceScore = performanceScore,
                pricePerSecond = pricePerSecond,
                minBillingUnitSeconds = 1L,
                supportsInterrupt = true,
                supportsCheckpoint = true,
                supportsWarmStart = true,
                parallelUnits = parallelUnits
            ),
            availableUnits = availableUnits,
            lastHeartbeatEpochMs = 1_700_000_000_000L,
            online = true
        )

    private data class FixedClockPort(private val now: Long) : ClockPort {
        override fun now(): Instant = Instant.fromEpochMilliseconds(now)
    }
}
