@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTracingPort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import kotlin.time.Duration
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class RemoteSolverServiceStopPolicyTest {
    @Test
    fun stopRunningTaskShouldNotHardInterruptWhenNodeDoesNotSupportInterrupt() {
        val fixture = createFixture(supportsInterrupt = false)

        runSuspend {
            val task = fixture.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/stop-soft")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            fixture.taskStatePort.upsertTask(
                task.copy(status = TaskStatus.RUNNING, assignedNodeId = NodeId.of(fixture.nodeId))
            )
            primeRunningHandle(fixture.service, task.taskId.value, fixture.nodeId)

            val stopped = fixture.service.stopTask(task.taskId, reason = "manual-stop-soft")
            assertNotNull(stopped)
            assertEquals(TaskStatus.STOPPED, stopped.status)
        }

        assertEquals(0, fixture.solverExecutionPort.stopCallCount)
    }

    @Test
    fun stopRunningTaskShouldHardInterruptWhenNodeSupportsInterrupt() {
        val fixture = createFixture(supportsInterrupt = true)

        runSuspend {
            val task = fixture.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/stop-hard")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            fixture.taskStatePort.upsertTask(
                task.copy(status = TaskStatus.RUNNING, assignedNodeId = NodeId.of(fixture.nodeId))
            )
            primeRunningHandle(fixture.service, task.taskId.value, fixture.nodeId)

            val stopped = fixture.service.stopTask(task.taskId, reason = "manual-stop-hard")
            assertNotNull(stopped)
            assertEquals(TaskStatus.STOPPED, stopped.status)
        }

        assertEquals(1, fixture.solverExecutionPort.stopCallCount)
    }

    private data class Fixture(
        val service: RemoteSolverService,
        val taskStatePort: InMemoryTaskStatePort,
        val solverExecutionPort: RecordingSolverExecutionPort,
        val nodeId: String
    )

    private fun createFixture(supportsInterrupt: Boolean): Fixture {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val schedulerEngine = SchedulerEngine(clock)
        val taskStatePort = InMemoryTaskStatePort()
        val nodeStatePort = InMemoryNodeStatePort()
        val eventPort = InMemoryEventPort(clock, idGenerator)
        val checkpointPort = InMemoryCheckpointPort()
        val budgetPort = InMemoryBudgetPort()
        val costLedgerPort = InMemoryCostLedgerPort()
        val distributedLockPort = InMemoryDistributedLockPort(clock, idGenerator)
        val metricsPort = InMemoryMetricsPort()
        val tracingPort = InMemoryTracingPort()
        val solverExecutionPort = RecordingSolverExecutionPort()
        val service = RemoteSolverService(
            schedulerEngine = schedulerEngine,
            taskStatePort = taskStatePort,
            nodeStatePort = nodeStatePort,
            eventPort = eventPort,
            checkpointPort = checkpointPort,
            budgetPort = budgetPort,
            costLedgerPort = costLedgerPort,
            distributedLockPort = distributedLockPort,
            solverExecutionPort = solverExecutionPort,
            metricsPort = metricsPort,
            tracingPort = tracingPort,
            clock = clock,
            idGenerator = idGenerator,
            config = RemoteSolverConfig()
        )

        val nodeId = "node-stop-policy"
        runSuspend {
            service.registerNode(
                NodeCapabilityProfile(
                    nodeId = nodeId,
                    solverType = "test",
                    performanceScore = 1.0,
                    pricePerSecond = 0.1,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = supportsInterrupt,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
        }
        return Fixture(
            service = service,
            taskStatePort = taskStatePort,
            solverExecutionPort = solverExecutionPort,
            nodeId = nodeId
        )
    }

    @Suppress("UNCHECKED_CAST")
    private fun primeRunningHandle(service: RemoteSolverService, taskId: String, nodeId: String) {
        val field = RemoteSolverService::class.java.getDeclaredField("runningHandleByTaskId")
        field.isAccessible = true
        val handles = field.get(service) as MutableMap<String, ExecutionHandle>
        handles[taskId] = ExecutionHandle(
            handleId = "handle-$taskId",
            taskId = taskId,
            sliceId = "slice-$taskId",
            nodeId = nodeId,
            startedAtEpochMs = System.currentTimeMillis()
        )
    }

    private class RecordingSolverExecutionPort : SolverExecutionPort {
        var stopCallCount: Int = 0
            private set

        override suspend fun start(
            payload: SolvePayload,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle {
            throw UnsupportedOperationException("start is not used in this test")
        }

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle {
            throw UnsupportedOperationException("resume is not used in this test")
        }

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult {
            throw UnsupportedOperationException("awaitSliceEnd is not used in this test")
        }

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? {
            throw UnsupportedOperationException("exportCheckpoint is not used in this test")
        }

        override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? {
            throw UnsupportedOperationException("fetchFinalResult is not used in this test")
        }

        override suspend fun stop(handle: ExecutionHandle): Boolean {
            stopCallCount += 1
            return true
        }
    }
}
