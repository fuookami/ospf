@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.port.TaskEventQueryPort
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class TaskTimelineReplayerTest {
    @Test
    fun replayShouldBuildTimelineForTask() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        runSuspend {
            runtime.service.registerNode(
                fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile(
                    nodeId = "node-replay",
                    solverType = "gurobi",
                    performanceScore = 1.0,
                    pricePerSecond = 0.01,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
            val task = runtime.service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/replay")
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            repeat(10) {
                runtime.service.scheduleOnce()
            }
            val finalTask = runtime.service.getTask(task.taskId)
            assertTrue(finalTask != null)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)

            val replayer = TaskTimelineReplayer(
                taskStatePort = runtime.service.taskStatePort(),
                checkpointPort = runtime.service.checkpointPort(),
                costLedgerPort = runtime.service.costLedgerPort(),
                clock = runtime.service.clockPort()
            )
            val report = replayer.replay(task.taskId)
            assertEquals(task.taskId.value, report.taskId)
            assertTrue(report.events.isNotEmpty())
            assertTrue(report.events.any { it.type == "TASK_CREATED" })
            assertTrue(report.events.any { it.type == "SLICE_FINISHED" || it.type == "TASK_TERMINAL" })
        }
    }

    @Test
    fun replayShouldPreferEventLogWhenQueryPortIsAvailable() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        runSuspend {
            runtime.service.registerNode(
                fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile(
                    nodeId = "node-replay-event",
                    solverType = "gurobi",
                    performanceScore = 1.0,
                    pricePerSecond = 0.01,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
            val task = runtime.service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/replay-event")
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            repeat(10) {
                runtime.service.scheduleOnce()
            }
            val replayer = TaskTimelineReplayer(
                taskStatePort = runtime.service.taskStatePort(),
                checkpointPort = runtime.service.checkpointPort(),
                costLedgerPort = runtime.service.costLedgerPort(),
                clock = runtime.service.clockPort(),
                taskEventQueryPort = runtime.service.eventPort() as? TaskEventQueryPort
            )
            val report = replayer.replay(task.taskId)
            assertTrue(report.events.isNotEmpty())
            assertTrue(report.events.any { it.attributes["source"] == "event_log" })
            assertTrue(report.events.any { it.type == "SolvingRequest" || it.type == "TaskResult" })
        }
    }
}
