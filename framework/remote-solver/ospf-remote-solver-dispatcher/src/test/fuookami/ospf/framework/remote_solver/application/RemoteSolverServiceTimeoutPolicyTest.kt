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
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import kotlin.time.Duration
import kotlin.time.Instant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class RemoteSolverServiceTimeoutPolicyTest {
    @Test
    fun taskShouldFailWhenHardTimeoutExceededBeforeDispatch() {
        val fixture = createFixture()

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-hard-timeout", supportsCheckpoint = true))
            val task = fixture.service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/hard-timeout"),
                    taskMeta = TaskMeta(timeLimitMs = 500L)
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            fixture.clock.advanceMs(1000L)

            val scheduled = fixture.service.scheduleOnce()
            assertNotNull(scheduled)
            assertEquals(TaskStatus.FAILED, scheduled.status)

            val stored = fixture.service.getTask(task.taskId)
            assertNotNull(stored)
            assertEquals(TaskStatus.FAILED, stored.status)
            assertTrue(fixture.service.getSlices(task.taskId).isEmpty())
        }
    }

    @Test
    fun taskShouldFailWhenSliceRuntimeExceedsQuantumAndGrace() {
        val fixture = createFixture(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 1000L,
                sliceTimeoutGraceMs = 0L
            ),
            solverFactory = { clock, idGenerator, _ ->
                SlowSliceSolverExecutionPort(
                    clock = clock,
                    idGenerator = idGenerator,
                    elapsedMs = 2500L
                )
            }
        )

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-slice-timeout", supportsCheckpoint = false))
            val task = fixture.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/slice-timeout")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )

            val scheduled = fixture.service.scheduleOnce()
            assertNotNull(scheduled)
            assertEquals(TaskStatus.FAILED, scheduled.status)

            val slices = fixture.service.getSlices(task.taskId)
            assertEquals(1, slices.size)
            assertEquals(SliceStatus.FAILED, slices.first().status)
            assertTrue(slices.first().error?.contains("Slice timeout exceeded") == true)
            assertEquals(
                RemoteSolverErrorCode.TASK_FAILED_SLICE_TIMEOUT.name,
                scheduled.latestResult?.extension?.get("reasonCode")
            )
        }
    }

    @Test
    fun taskShouldFailBeforePersistingInvalidStrictSliceResult() {
        val fixture = createFixture(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 1000L,
                sliceTimeoutGraceMs = 0L
            ),
            solverFactory = { clock, idGenerator, _ ->
                InvalidStrictSliceResultSolverExecutionPort(clock, idGenerator)
            }
        )

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-invalid-result", supportsCheckpoint = false))
            val task = fixture.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/invalid-result")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )

            val scheduled = fixture.service.scheduleOnce()

            assertNotNull(scheduled)
            assertEquals(TaskStatus.FAILED, scheduled.status)
            assertEquals(
                RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED.name,
                scheduled.latestResult?.extension?.get("reasonCode")
            )
            val slices = fixture.service.getSlices(task.taskId)
            assertEquals(1, slices.size)
            assertEquals(SliceStatus.FAILED, slices.single().status)
            assertTrue(slices.single().error?.contains("协议校验失败") == true)
        }
    }

    @Test
    fun submitAndAwaitShouldThrowHardTimeoutCodeWhenThrowIfFailedEnabled() {
        val fixture = createFixture()

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-hard-timeout-await", supportsCheckpoint = true))
            fixture.service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/hard-timeout-await"),
                    taskMeta = TaskMeta(timeLimitMs = 500L)
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                requestId = RequestId.of("req-hard-timeout-await")
            )
            fixture.clock.advanceMs(1000L)
        }

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                fixture.service.submitAndAwait(
                    payload = SolvePayload(
                        modelRef = ObjectRef.of(path = "model/hard-timeout-await"),
                        taskMeta = TaskMeta(timeLimitMs = 500L)
                    ),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    requestId = RequestId.of("req-hard-timeout-await"),
                    maxRounds = UInt64(2),
                    throwIfFailed = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT, error.code)
    }

    @Test
    fun submitAndAwaitShouldThrowSliceTimeoutCodeWhenThrowIfFailedEnabled() {
        val fixture = createFixture(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 1000L,
                sliceTimeoutGraceMs = 0L
            ),
            solverFactory = { clock, idGenerator, _ ->
                SlowSliceSolverExecutionPort(
                    clock = clock,
                    idGenerator = idGenerator,
                    elapsedMs = 2500L
                )
            }
        )

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-slice-timeout-await", supportsCheckpoint = false))
        }

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                fixture.service.submitAndAwait(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/slice-timeout-await")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    maxRounds = UInt64(2),
                    throwIfFailed = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.TASK_FAILED_SLICE_TIMEOUT, error.code)
    }

    @Test
    fun submitAndAwaitShouldThrowSolverExecutionFailedCodeWhenExecutionThrows() {
        val fixture = createFixture(
            solverFactory = { clock, idGenerator, _ ->
                ThrowingSolverExecutionPort(clock = clock, idGenerator = idGenerator)
            }
        )

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-execution-throws", supportsCheckpoint = true))
        }

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                fixture.service.submitAndAwait(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/execution-throws")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    maxRounds = UInt64(2),
                    throwIfFailed = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED, error.code)
    }

    @Test
    fun submitAndAwaitShouldThrowCheckpointExportFailedCodeWhenExportFails() {
        val fixture = createFixture(
            solverFactory = { clock, idGenerator, _ ->
                CheckpointFailingSolverExecutionPort(clock = clock, idGenerator = idGenerator)
            }
        )

        runSuspend {
            fixture.service.registerNode(defaultNodeProfile("node-checkpoint-export-fail", supportsCheckpoint = true))
        }

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                fixture.service.submitAndAwait(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/checkpoint-export-fail")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    maxRounds = UInt64(2),
                    throwIfFailed = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.CHECKPOINT_EXPORT_FAILED, error.code)
    }

    private data class Fixture(
        val service: RemoteSolverService,
        val clock: MutableClockPort
    )

    private fun createFixture(
        config: RemoteSolverConfig = RemoteSolverConfig(),
        solverFactory: (ClockPort, IdGeneratorPort, InMemoryObjectStoragePort) -> SolverExecutionPort = { clock, idGenerator, objectStorage ->
            MockSolverExecutionPort(clock, idGenerator, objectStorage)
        }
    ): Fixture {
        val clock = MutableClockPort(1_700_000_000_000L)
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
            idGenerator = idGenerator,
            config = config
        )
        return Fixture(service = service, clock = clock)
    }

    private fun defaultNodeProfile(nodeId: String, supportsCheckpoint: Boolean): NodeCapabilityProfile =
        NodeCapabilityProfile(
            nodeId = nodeId,
            solverType = "test",
            performanceScore = 1.0,
            pricePerSecond = 0.1,
            minBillingUnitSeconds = 1L,
            supportsInterrupt = true,
            supportsCheckpoint = supportsCheckpoint,
            supportsWarmStart = true,
            parallelUnits = 1
        )

    private class MutableClockPort(private var nowEpochMs: Long) : ClockPort {
        override fun now(): Instant = Instant.fromEpochMilliseconds(nowEpochMs)

        fun advanceMs(deltaMs: Long) {
            nowEpochMs += deltaMs
        }
    }

    private class SlowSliceSolverExecutionPort(
        private val clock: ClockPort,
        private val idGenerator: IdGeneratorPort,
        private val elapsedMs: Long
    ) : SolverExecutionPort {
        override suspend fun start(
            payload: SolvePayload,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle =
            ExecutionHandle(
                handleId = idGenerator.newId("handle"),
                taskId = taskId.value,
                sliceId = sliceId.value,
                nodeId = nodeId.value,
                startedAtEpochMs = clock.nowEpochMs()
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
                gap = 0.9,
                elapsedMs = elapsedMs
            )

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? = null

        override suspend fun fetchFinalResult(handle: ExecutionHandle) = null

        override suspend fun stop(handle: ExecutionHandle): Boolean = true
    }

    private class InvalidStrictSliceResultSolverExecutionPort(
        private val clock: ClockPort,
        private val idGenerator: IdGeneratorPort
    ) : SolverExecutionPort {
        override suspend fun start(
            payload: SolvePayload,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle = ExecutionHandle(
            handleId = idGenerator.newId("handle"),
            taskId = taskId.value,
            sliceId = sliceId.value,
            nodeId = nodeId.value,
            startedAtEpochMs = clock.nowEpochMs()
        )

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle = start(payload, taskId, sliceId, nodeId, tenantId)

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult = SliceResult(
            sliceId = SliceId.of(handle.sliceId.value),
            completed = true,
            feasible = true,
            objectiveValue = Flt64.one,
            gap = Flt64.zero,
            elapsed = quantum,
            schemaVersion = "3.0",
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT
        )

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? = null

        override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? = null

        override suspend fun stop(handle: ExecutionHandle): Boolean = true
    }

    private class ThrowingSolverExecutionPort(
        private val clock: ClockPort,
        private val idGenerator: IdGeneratorPort
    ) : SolverExecutionPort {
        override suspend fun start(
            payload: SolvePayload,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle =
            ExecutionHandle(
                handleId = idGenerator.newId("handle"),
                taskId = taskId.value,
                sliceId = sliceId.value,
                nodeId = nodeId.value,
                startedAtEpochMs = clock.nowEpochMs()
            )

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle = start(payload, taskId, sliceId, nodeId, tenantId)

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult {
            throw IllegalStateException("simulated solver execution failure")
        }

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? = null

        override suspend fun fetchFinalResult(handle: ExecutionHandle) = null

        override suspend fun stop(handle: ExecutionHandle): Boolean = true
    }

    private class CheckpointFailingSolverExecutionPort(
        private val clock: ClockPort,
        private val idGenerator: IdGeneratorPort
    ) : SolverExecutionPort {
        override suspend fun start(
            payload: SolvePayload,
            taskId: TaskId,
            sliceId: SliceId,
            nodeId: NodeId,
            tenantId: TenantId
        ): ExecutionHandle =
            ExecutionHandle(
                handleId = idGenerator.newId("handle"),
                taskId = taskId.value,
                sliceId = sliceId.value,
                nodeId = nodeId.value,
                startedAtEpochMs = clock.nowEpochMs()
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
                gap = 0.9,
                elapsedMs = quantum.inWholeMilliseconds
            )

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? {
            throw IllegalStateException("simulated checkpoint export failure")
        }

        override suspend fun fetchFinalResult(handle: ExecutionHandle) = null

        override suspend fun stop(handle: ExecutionHandle): Boolean = true
    }
}
