@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RemoteSolverServiceAcceptanceTest {
    @Test
    fun mixedLoadSevenToThreeShouldCompleteAndPersistComplexSnapshots() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 12000L,
                complexTaskQuantumMs = 4000L,
                complexTaskQuantumMinMs = 4000L,
                complexTaskQuantumMaxMs = 4000L,
                maxSchedulingBatch = 256
            )
        )

        runSuspend {
            runtime.service.registerNode(defaultNodeProfile("node-mix-a", pricePerSecond = 0.01))
            runtime.service.registerNode(defaultNodeProfile("node-mix-b", pricePerSecond = 0.015))

            val simpleTaskIds = (1..7).map {
                runtime.service.submitTask(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/mix-simple-$it")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    priority = 5
                ).taskId
            }
            val complexTaskIds = (1..3).map {
                runtime.service.submitTask(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/mix-complex-$it")),
                    complexity = TaskComplexity.COMPLEX,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    priority = 5
                ).taskId
            }

            repeat(100) {
                runtime.service.scheduleOnce()
                val allDone = (simpleTaskIds + complexTaskIds).all { taskId ->
                    runtime.service.getTask(taskId)?.status == TaskStatus.COMPLETED
                }
                if (allDone) {
                    return@repeat
                }
            }

            simpleTaskIds.forEach { taskId ->
                assertEquals(TaskStatus.COMPLETED, runtime.service.getTask(taskId)?.status)
                assertEquals(1, runtime.service.getSlices(taskId).size)
            }
            complexTaskIds.forEach { taskId ->
                assertEquals(TaskStatus.COMPLETED, runtime.service.getTask(taskId)?.status)
                val slices = runtime.service.getSlices(taskId)
                assertTrue(slices.size >= 3)
                assertTrue(slices.any { it.checkpointRef != null })
            }
        }
    }

    @Test
    fun smallBurstShouldDrainQueueWithoutFailures() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 12000L,
                maxSchedulingBatch = 512
            )
        )

        runSuspend {
            runtime.service.registerNode(defaultNodeProfile("node-burst-a", pricePerSecond = 0.01))
            runtime.service.registerNode(defaultNodeProfile("node-burst-b", pricePerSecond = 0.012))

            val taskIds = (1..120).map {
                runtime.service.submitTask(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/burst-$it")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    priority = 1
                ).taskId
            }

            repeat(200) {
                runtime.service.scheduleOnce()
                val allDone = taskIds.all { taskId ->
                    runtime.service.getTask(taskId)?.status == TaskStatus.COMPLETED
                }
                if (allDone) {
                    return@repeat
                }
            }

            val finalStatuses = taskIds.mapNotNull { runtime.service.getTask(it)?.status }
            assertEquals(120, finalStatuses.size)
            assertEquals(120, finalStatuses.count { it == TaskStatus.COMPLETED })
            assertEquals(0, finalStatuses.count { it == TaskStatus.FAILED })
        }
    }

    private fun defaultNodeProfile(nodeId: String, pricePerSecond: Double): NodeCapabilityProfile =
        NodeCapabilityProfile(
            nodeId = nodeId,
            solverType = "gurobi",
            performanceScore = 1.0,
            pricePerSecond = pricePerSecond,
            minBillingUnitSeconds = 1L,
            supportsInterrupt = true,
            supportsCheckpoint = true,
            supportsWarmStart = true,
            parallelUnits = 1
        )
}
