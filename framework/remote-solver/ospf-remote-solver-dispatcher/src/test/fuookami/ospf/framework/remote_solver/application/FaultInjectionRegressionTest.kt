@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.mock.MockSolverExecutionPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTracingPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.HandleId
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import kotlin.time.Duration
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class FaultInjectionRegressionTest {
    @Test
    fun duplicateEventShouldBeDeduplicatedByIdempotencyKey() {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val eventPort = InMemoryEventPort(clock, idGenerator)

        runSuspend {
            var delivered = 0
            eventPort.subscribe("fault-topic", "group-1") {
                delivered += 1
            }
            val first = eventPort.publish(
                topic = "fault-topic",
                key = "task-1",
                payload = "v1".toByteArray(),
                headers = mapOf("idempotencyKey" to "dispatch-1")
            )
            val second = eventPort.publish(
                topic = "fault-topic",
                key = "task-1",
                payload = "v2".toByteArray(),
                headers = mapOf("idempotencyKey" to "dispatch-1")
            )
            assertEquals(first.eventId, second.eventId)
            assertEquals(1, delivered)
        }
    }

    @Test
    fun nodeOfflineShouldRecoverRunningSlice() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 2000L,
                complexTaskQuantumMs = 2000L,
                maxSchedulingBatch = 8,
                nodeHeartbeatTimeoutMs = 1000L
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-fault-timeout",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/fault-timeout")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            val node = runtime.nodeStatePort.getNode("node-fault-timeout")
            assertNotNull(node)
            runtime.nodeStatePort.upsertNode(
                node.copy(
                    online = true,
                    availableUnits = 0,
                    lastHeartbeat = kotlin.time.Instant.fromEpochMilliseconds(node.lastHeartbeatEpochMs - 5000L)
                )
            )
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.RUNNING,
                    assignedNodeId = NodeId.of("node-fault-timeout")
                )
            )
            runtime.taskStatePort.appendSlice(
                SliceState(
                    sliceId = "slice-fault-timeout",
                    taskId = task.taskId.value,
                    dispatchId = "dispatch-fault-timeout",
                    status = SliceStatus.RUNNING,
                    nodeId = "node-fault-timeout",
                    quantum = 1000L,
                    startedAt = node.lastHeartbeatEpochMs - 4000L
                )
            )

            val recovered = runtime.service.reconcileNodeHealth()
            assertEquals(1, recovered)

            val recoveredTask = runtime.service.getTask(task.taskId)
            assertNotNull(recoveredTask)
            assertEquals(TaskStatus.QUEUED, recoveredTask.status)
            assertEquals(null, recoveredTask.assignedNodeId)
        }
    }

    @Test
    fun checkpointExportFailureShouldFailWithReasonCode() {
        val fixture = createFixture(
            solverFactory = { clock, idGenerator, _ ->
                CheckpointFailingSolverExecutionPort(clock, idGenerator)
            }
        )

        runSuspend {
            fixture.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-fault-checkpoint",
                    solverType = "test",
                    performanceScore = 1.0,
                    pricePerSecond = 0.1,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
        }

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                fixture.service.submitAndAwait(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/fault-checkpoint")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    maxRounds = UInt64(2),
                    throwIfFailed = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.CHECKPOINT_EXPORT_FAILED, error.code)

        runSuspend {
            val failed = fixture.taskStatePort
                .listTasks(setOf(TaskStatus.FAILED), 1)
                .firstOrNull()
            assertNotNull(failed)
            assertEquals(
                RemoteSolverErrorCode.CHECKPOINT_EXPORT_FAILED.name,
                failed.latestResult?.extension?.get("reasonCode")
            )
            val ledger = fixture.costLedgerPort.listByTask(failed.taskId.value)
            assertTrue(ledger.isEmpty())
        }
    }

    private data class Fixture(
        val service: RemoteSolverService,
        val taskStatePort: InMemoryTaskStatePort,
        val costLedgerPort: InMemoryCostLedgerPort
    )

    private fun createFixture(
        solverFactory: (ClockPort, UUIDIdGeneratorPort, InMemoryObjectStoragePort) -> SolverExecutionPort
    ): Fixture {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val taskStatePort = InMemoryTaskStatePort()
        val nodeStatePort = InMemoryNodeStatePort()
        val eventPort = InMemoryEventPort(clock, idGenerator)
        val checkpointPort = InMemoryCheckpointPort()
        val budgetPort = InMemoryBudgetPort()
        val costLedgerPort = InMemoryCostLedgerPort()
        val distributedLockPort = InMemoryDistributedLockPort(clock, idGenerator)
        val metricsPort = InMemoryMetricsPort()
        val tracingPort = InMemoryTracingPort()
        val service = RemoteSolverService(
            schedulerEngine = SchedulerEngine(clock),
            taskStatePort = taskStatePort,
            nodeStatePort = nodeStatePort,
            eventPort = eventPort,
            checkpointPort = checkpointPort,
            budgetPort = budgetPort,
            costLedgerPort = costLedgerPort,
            distributedLockPort = distributedLockPort,
            solverExecutionPort = solverFactory(clock, idGenerator, objectStoragePort),
            metricsPort = metricsPort,
            tracingPort = tracingPort,
            clock = clock,
            idGenerator = idGenerator
        )
        return Fixture(
            service = service,
            taskStatePort = taskStatePort,
            costLedgerPort = costLedgerPort
        )
    }

    private class CheckpointFailingSolverExecutionPort(
        private val clock: ClockPort,
        private val idGenerator: UUIDIdGeneratorPort
    ) : SolverExecutionPort {
        override suspend fun start(
            payload: SolvePayload,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle =
            ExecutionHandle(
                handleId = HandleId.of(idGenerator.newId("handle")),
                taskId = taskId,
                sliceId = sliceId,
                nodeId = nodeId,
                startedAt = clock.now()
            )

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle = start(payload, taskId, sliceId, nodeId, tenantId)

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult =
            SliceResult(
                sliceId = handle.sliceId.value,
                completed = false,
                feasible = true,
                objectiveValue = 1.0,
                gap = 0.5,
                elapsedMs = quantum.inWholeMilliseconds
            )

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? {
            throw IllegalStateException("simulated checkpoint export failure")
        }

        override suspend fun fetchFinalResult(handle: ExecutionHandle) = null

        override suspend fun stop(handle: ExecutionHandle): Boolean = true
    }
}
