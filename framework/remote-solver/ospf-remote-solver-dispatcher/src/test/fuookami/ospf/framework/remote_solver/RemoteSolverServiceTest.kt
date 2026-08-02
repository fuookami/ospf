@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemorySchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTracingPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.mock.MockSolverExecutionPort
import fuookami.ospf.framework.remote_solver.application.RemoteSolverConfig
import fuookami.ospf.framework.remote_solver.application.RemoteSolverService
import fuookami.ospf.framework.remote_solver.application.SchedulerEngine
import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.EventTopics
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import kotlin.time.Instant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class RemoteSolverServiceTest {
    @Test
    fun submitTaskShouldInferComplexityFromTaskMeta() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                complexTaskVariableThreshold = 100,
                complexTaskConstraintThreshold = 200,
                complexTaskHistoricalRuntimeThresholdMs = 5000L
            )
        )

        runSuspend {
            val task = runtime.service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/infer-complex"),
                    taskMeta = TaskMeta(
                        estimatedVariableCount = 80,
                        estimatedConstraintCount = 180,
                        historicalRuntimeMs = 6000L
                    )
                )
            )

            assertEquals(TaskComplexity.COMPLEX, task.complexity)
            assertEquals(TimeSensitivity.NON_REALTIME, task.timeSensitivity)
        }
    }

    @Test
    fun submitTaskShouldInferSimpleComplexityWhenTaskMetaBelowThresholds() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                complexTaskVariableThreshold = 1000,
                complexTaskConstraintThreshold = 1000,
                complexTaskHistoricalRuntimeThresholdMs = 120000L
            )
        )

        runSuspend {
            val task = runtime.service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/infer-simple"),
                    taskMeta = TaskMeta(
                        estimatedVariableCount = 200,
                        estimatedConstraintCount = 300,
                        historicalRuntimeMs = 10000L
                    )
                )
            )

            assertEquals(TaskComplexity.SIMPLE, task.complexity)
            assertEquals(TimeSensitivity.NON_REALTIME, task.timeSensitivity)
        }
    }

    @Test
    fun complexRealtimeTaskShouldBeDowngradedToNonRealtime() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val controlEvents = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.SOLVING_CONTROL, "group-downgrade-test") { record ->
                controlEvents.add(record.payload.decodeToString())
            }

            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/downgrade")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.REALTIME,
                priority = 1
            )

            assertEquals(TimeSensitivity.NON_REALTIME, task.timeSensitivity)
            val stored = runtime.service.getTask(task.taskId)
            assertNotNull(stored)
            assertEquals(TimeSensitivity.NON_REALTIME, stored.timeSensitivity)
            assertTrue(
                controlEvents.any {
                    it.contains("\"taskId\":\"${task.taskId}\"") &&
                        it.contains("\"action\":\"downgrade\"") &&
                        it.contains("\"from\":\"REALTIME\"") &&
                        it.contains("\"to\":\"NON_REALTIME\"")
                }
            )
        }
    }

    @Test
    fun scheduleShouldPublishAcceptAndConfirmControlEvents() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val controlEvents = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.SOLVING_CONTROL, "group-control-test") { record ->
                controlEvents.add(record.payload.decodeToString())
            }

            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-control",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/control")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )

            runtime.service.scheduleOnce()

            assertTrue(
                controlEvents.any {
                    it.contains("\"taskId\":\"${task.taskId}\"") &&
                        it.contains("\"action\":\"accept\"") &&
                        it.contains("\"status\":\"accepted\"")
                }
            )
            assertTrue(
                controlEvents.any {
                    it.contains("\"taskId\":\"${task.taskId}\"") &&
                        it.contains("\"action\":\"confirm\"") &&
                        it.contains("\"status\":\"dispatching\"")
                }
            )
        }
    }

    @Test
    fun queuedTaskCanBeStoppedAndWillNotBeScheduled() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/stop-queued")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 3
            )

            val stopped = runtime.service.stopTask(task.taskId, reason = "manual-stop")
            assertNotNull(stopped)
            assertEquals(TaskStatus.STOPPED, stopped.status)

            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-stop-queued",
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

            val scheduled = runtime.service.scheduleOnce()
            assertEquals(null, scheduled)

            val finalTask = runtime.service.getTask(task.taskId)
            assertNotNull(finalTask)
            assertEquals(TaskStatus.STOPPED, finalTask.status)
            assertTrue(runtime.service.getSlices(task.taskId).isEmpty())
        }
    }

    @Test
    fun stoppedSimpleTaskCanResumeToQueued() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/resume-simple")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.STOPPED,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(System.currentTimeMillis())
                )
            )

            val resumed = runtime.service.resumeTask(task.taskId)
            assertNotNull(resumed)
            assertEquals(TaskStatus.QUEUED, resumed.status)
            assertEquals(null, resumed.assignedNodeId)
        }
    }

    @Test
    fun stoppedComplexTaskWithSnapshotCanResumeToSuspended() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/resume-complex")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.STOPPED,
                    latestSnapshotRef = ObjectRef.of(path = "checkpoints/resume-complex"),
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(System.currentTimeMillis())
                )
            )

            val resumed = runtime.service.resumeTask(task.taskId)
            assertNotNull(resumed)
            assertEquals(TaskStatus.SUSPENDED, resumed.status)
        }
    }

    @Test
    fun failedTaskResumeShouldThrowInvalidStateTransition() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/resume-invalid")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.FAILED,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(System.currentTimeMillis())
                )
            )

            val error = assertFailsWith<RemoteSolverException> {
                runtime.service.resumeTask(task.taskId)
            }
            assertEquals(RemoteSolverErrorCode.INVALID_TASK_STATE_TRANSITION, error.code)
        }
    }

    @Test
    fun resumeTaskShouldPublishResumeControlEvent() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val controlEvents = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.SOLVING_CONTROL, "group-resume-control-test") { record ->
                controlEvents.add(record.payload.decodeToString())
            }
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/resume-event")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.STOPPED,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(System.currentTimeMillis())
                )
            )

            val resumed = runtime.service.resumeTask(task.taskId)
            assertNotNull(resumed)
            assertEquals(TaskStatus.QUEUED, resumed.status)
            assertTrue(
                controlEvents.any {
                    it.contains("\"taskId\":\"${task.taskId}\"") &&
                        it.contains("\"action\":\"resume\"") &&
                        it.contains("\"status\":\"QUEUED\"")
                }
            )
        }
    }

    @Test
    fun runningTaskCanBeStoppedAndActiveSliceShouldFail() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-stop-running",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/stop-running")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 2
            )

            val node = runtime.nodeStatePort.getNode("node-stop-running")
            assertNotNull(node)
            runtime.nodeStatePort.upsertNode(node.copy(availableUnits = 0))
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.RUNNING,
                    assignedNodeId = NodeId.of("node-stop-running")
                )
            )
            runtime.taskStatePort.appendSlice(
                SliceState(
                    sliceId = "slice-stop-running",
                    taskId = task.taskId.value,
                    dispatchId = "dispatch-stop-running",
                    status = SliceStatus.RUNNING,
                    nodeId = "node-stop-running",
                    quantum = 1000L,
                    startedAt = System.currentTimeMillis()
                )
            )

            val stopped = runtime.service.stopTask(task.taskId, reason = "manual-stop")
            assertNotNull(stopped)
            assertEquals(TaskStatus.STOPPED, stopped.status)
            assertEquals(null, stopped.assignedNodeId)

            val slices = runtime.service.getSlices(task.taskId)
            assertEquals(1, slices.size)
            assertEquals(SliceStatus.FAILED, slices[0].status)
            assertTrue(slices[0].error?.contains("manual-stop") == true)

            val recoveredNode = runtime.nodeStatePort.getNode("node-stop-running")
            assertNotNull(recoveredNode)
            assertEquals(1, recoveredNode.availableUnits)
        }
    }

    @Test
    fun complexTaskCanCompleteAfterMultipleSlices() {
        // Create a custom runtime with MockSolverExecutionPort that requires multiple slices
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val eventPort = InMemoryEventPort(clock, idGenerator)
        val taskStatePort = InMemoryTaskStatePort()
        val nodeStatePort = InMemoryNodeStatePort()
        val budgetPort = InMemoryBudgetPort()
        val costLedgerPort = InMemoryCostLedgerPort()
        val distributedLockPort = InMemoryDistributedLockPort(clock, idGenerator)
        val metricsPort = InMemoryMetricsPort()
        val tracingPort = InMemoryTracingPort()
        val schedulerConfigAuditPort = InMemorySchedulerConfigAuditPort()
        val schedulerEngine = SchedulerEngine(clock)
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val checkpointPort = InMemoryCheckpointPort(0)

        // Use progressPerSlice = 0.33 to require ~3 slices to complete
        val solverExecutionPort = MockSolverExecutionPort(
            clock = clock,
            idGenerator = idGenerator,
            objectStoragePort = objectStoragePort,
            simulatedTotalRuntimeMs = 12000L,
            progressPerSlice = 0.33
        )

        val config = RemoteSolverConfig(
            simpleTaskQuantumMs = 1000L,
            complexTaskQuantumMs = 2500L,
            maxSchedulingBatch = 8
        )

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
            config = config,
            schedulerConfigAuditPort = schedulerConfigAuditPort
        )

        runSuspend {
            service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-a",
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

            val task = service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/demo")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 10,
                budgetLimit = Flt64(10.0)
            )

            repeat(10) {
                service.scheduleOnce()
            }

            val finalTask = service.getTask(task.taskId)
            val slices = service.getSlices(task.taskId)
            val costs = service.getTaskCostRecords(task.taskId)
            val summary = service.getTaskCostSummary(task.taskId)

            assertNotNull(finalTask)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)
            assertTrue(slices.size > 1)
            assertTrue(slices.any { it.checkpointRef != null })
            assertTrue(costs.isNotEmpty())
            assertEquals(finalTask.consumedCost.toDouble(), costs.sumOf { it.totalCost })
            assertEquals(costs.size, summary.recordCount)
            assertEquals(finalTask.consumedCost.toDouble(), summary.totalCost)
        }
    }

    @Test
    fun sliceLifecycleShouldPublishStartSuspendResumeAndEndActions() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                complexTaskQuantumMs = 4000L,
                complexTaskQuantumMinMs = 4000L,
                complexTaskQuantumMaxMs = 4000L,
                maxSchedulingBatch = 8
            )
        )

        runSuspend {
            val lifecyclePayloads = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.SLICE_LIFECYCLE, "group-slice-lifecycle-test") { record ->
                lifecyclePayloads.add(record.payload.decodeToString())
            }
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-lifecycle",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/lifecycle")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )

            repeat(5) {
                runtime.service.scheduleOnce()
            }

            val finalTask = runtime.service.getTask(task.taskId)
            assertNotNull(finalTask)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)
            assertTrue(lifecyclePayloads.any { it.contains("\"action\":\"slice-start\"") })
            assertTrue(lifecyclePayloads.any { it.contains("\"action\":\"slice-suspend\"") })
            assertTrue(lifecyclePayloads.any { it.contains("\"action\":\"slice-resume\"") })
            assertTrue(
                lifecyclePayloads.any {
                    it.contains("\"action\":\"slice-end\"") &&
                        it.contains("\"status\":\"COMPLETED\"")
                }
            )
        }
    }

    @Test
    fun budgetExceededTaskWillFail() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 5000L,
                complexTaskQuantumMs = 5000L,
                maxSchedulingBatch = 8
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-b",
                    solverType = "scip",
                    performanceScore = 1.0,
                    pricePerSecond = 100.0,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )

            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/high-cost")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1,
                budgetLimit = Flt64(1.0)
            )

            repeat(5) {
                runtime.service.scheduleOnce()
            }

            val finalTask = runtime.service.getTask(task.taskId)
            assertTrue(finalTask != null)
            assertEquals(TaskStatus.FAILED, finalTask.status)
        }
    }

    @Test
    fun staleNodeShouldBeMarkedOfflineAndRunningTaskShouldBeRecovered() {
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
                    nodeId = "node-timeout",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/timeout")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 5,
                budgetLimit = Flt64(10.0)
            )

            val node = runtime.nodeStatePort.getNode("node-timeout")
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
                    assignedNodeId = NodeId.of("node-timeout")
                )
            )
            runtime.taskStatePort.appendSlice(
                SliceState(
                    sliceId = "slice-timeout",
                    taskId = task.taskId.value,
                    dispatchId = "dispatch-timeout",
                    status = SliceStatus.RUNNING,
                    nodeId = "node-timeout",
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

            val slices = runtime.service.getSlices(task.taskId)
            assertEquals(1, slices.size)
            assertEquals(SliceStatus.FAILED, slices[0].status)
            assertTrue(slices[0].error?.contains("timeout") == true)

            val staleNode = runtime.nodeStatePort.getNode("node-timeout")
            assertNotNull(staleNode)
            assertFalse(staleNode.online)
            assertEquals(0, staleNode.availableUnits)
        }
    }

    @Test
    fun duplicateSubmitWithSameRequestIdShouldReturnExistingTask() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val first = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/idempotent")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1,
                budgetLimit = Flt64(5.0),
                requestId = RequestId.of("req-idempotent-1")
            )
            val second = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/idempotent-retry")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.REALTIME,
                priority = 9,
                budgetLimit = Flt64(999.0),
                requestId = RequestId.of("req-idempotent-1")
            )

            assertEquals(first.taskId, second.taskId)
            assertEquals(first.requestId, second.requestId)

            val queued = runtime.taskStatePort.listTasks(setOf(TaskStatus.QUEUED), limit = 100)
            assertEquals(1, queued.size)
            assertEquals(first.taskId, queued[0].taskId)
            assertEquals(TaskComplexity.SIMPLE, queued[0].complexity)
            assertEquals(5.0, queued[0].budgetLimit?.toDouble())
        }
    }

    @Test
    fun complexTaskQuantumMsShouldBeClampedByConfiguredRange() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                complexTaskQuantumMs = 3500L,
                complexTaskQuantumMinMs = 3000L,
                complexTaskQuantumMaxMs = 4000L,
                complexSolveEstimateMs = 20000L,
                complexCheckpointEstimateMs = 1000L,
                complexQuantumAlpha = 0.7,
                complexQuantumBeta = 0.5,
                complexQuantumPricePenalty = 0.1
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-quantum",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/quantum")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )

            runtime.service.scheduleOnce()
            val slices = runtime.service.getSlices(task.taskId)
            assertEquals(1, slices.size)
            assertEquals(4000L, slices.first().quantum.inWholeMilliseconds)
        }
    }

    @Test
    fun complexUrgentTaskShouldBeScheduledBeforeHigherPriorityNonUrgentTask() {
        // Create a custom runtime with MockSolverExecutionPort that requires multiple slices
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val eventPort = InMemoryEventPort(clock, idGenerator)
        val taskStatePort = InMemoryTaskStatePort()
        val nodeStatePort = InMemoryNodeStatePort()
        val budgetPort = InMemoryBudgetPort()
        val costLedgerPort = InMemoryCostLedgerPort()
        val distributedLockPort = InMemoryDistributedLockPort(clock, idGenerator)
        val metricsPort = InMemoryMetricsPort()
        val tracingPort = InMemoryTracingPort()
        val schedulerConfigAuditPort = InMemorySchedulerConfigAuditPort()
        val schedulerEngine = SchedulerEngine(clock)
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val checkpointPort = InMemoryCheckpointPort(0)

        // Use progressPerSlice = 0.5 to require 2 slices to complete
        val solverExecutionPort = MockSolverExecutionPort(
            clock = clock,
            idGenerator = idGenerator,
            objectStoragePort = objectStoragePort,
            simulatedTotalRuntimeMs = 2000L,
            progressPerSlice = 0.5
        )

        val config = RemoteSolverConfig(
            complexTaskQuantumMs = 1000L,
            complexTaskQuantumMinMs = 1000L,
            complexTaskQuantumMaxMs = 1000L
        )

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
            config = config,
            schedulerConfigAuditPort = schedulerConfigAuditPort
        )

        runSuspend {
            service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-rr",
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

            val nonUrgentTask = service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/non-urgent")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 10
            )
            val urgentTask = service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/urgent")),
                complexity = TaskComplexity.COMPLEX,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1,
                deadline = Instant.fromEpochMilliseconds(System.currentTimeMillis() - 1000L)
            )

            service.scheduleOnce()

            val urgentState = service.getTask(urgentTask.taskId)
            val nonUrgentState = service.getTask(nonUrgentTask.taskId)
            assertNotNull(urgentState)
            assertNotNull(nonUrgentState)
            assertEquals(TaskStatus.SUSPENDED, urgentState.status)
            assertEquals(TaskStatus.QUEUED, nonUrgentState.status)
        }
    }

    @Test
    fun taskWithRequiredSolverTypeShouldSelectCompatibleNode() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-solver-incompatible",
                    solverType = "scip",
                    performanceScore = 10.0,
                    pricePerSecond = 0.001,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-solver-compatible",
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
                    modelRef = ObjectRef.of(path = "model/solver-type"),
                    extension = mapOf("solverType" to "gurobi")
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )

            repeat(10) {
                runtime.service.scheduleOnce()
            }
            val updated = runtime.service.getTask(task.taskId)
            assertNotNull(updated)
            assertEquals(TaskStatus.COMPLETED, updated.status)
            val slices = runtime.service.getSlices(task.taskId)
            assertTrue(slices.isNotEmpty())
            assertTrue(slices.all { it.nodeId?.value == "node-solver-compatible" })
        }
    }

    @Test
    fun taskWithRequiredSolverTypeShouldStayQueuedWhenNoCompatibleNode() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-only-scip",
                    solverType = "scip",
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
                    modelRef = ObjectRef.of(path = "model/solver-type-queued"),
                    extension = mapOf("solverType" to "gurobi")
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )

            val scheduled = runtime.service.scheduleOnce()
            assertEquals(null, scheduled)
            val stored = runtime.service.getTask(task.taskId)
            assertNotNull(stored)
            assertEquals(TaskStatus.QUEUED, stored.status)
        }
    }

    @Test
    fun taskShouldFailBeforeDispatchWhenEstimatedSliceCostExceedsBudget() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 5000L
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-budget-estimate",
                    solverType = "gurobi",
                    performanceScore = 1.0,
                    pricePerSecond = 10.0,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/budget-estimate")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                budgetLimit = Flt64(1.0)
            )

            val scheduled = runtime.service.scheduleOnce()
            assertNotNull(scheduled)
            assertEquals(TaskStatus.FAILED, scheduled.status)
            assertTrue(runtime.service.getSlices(task.taskId).isEmpty())
        }
    }

    @Test
    fun taskMetaSolverTypeShouldDriveCompatibilityFilter() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-meta-scip",
                    solverType = "scip",
                    performanceScore = 10.0,
                    pricePerSecond = 0.001,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-meta-gurobi",
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
                    modelRef = ObjectRef.of(path = "model/meta-solver-type"),
                    taskMeta = TaskMeta(solverType = "gurobi")
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )

            repeat(10) {
                runtime.service.scheduleOnce()
            }
            val finalTask = runtime.service.getTask(task.taskId)
            assertNotNull(finalTask)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)
            val slices = runtime.service.getSlices(task.taskId)
            assertTrue(slices.isNotEmpty())
            assertTrue(slices.all { it.nodeId?.value == "node-meta-gurobi" })
        }
    }

    @Test
    fun submitAndAwaitShouldReturnCompletedTaskWhenNodeIsAvailable() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 4000L,
                maxSchedulingBatch = 8
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-submit-await",
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

            val task = runtime.service.submitAndAwait(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/submit-await-complete")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                maxRounds = UInt64(10)
            )

            assertEquals(TaskStatus.COMPLETED, task.status)
        }
    }

    @Test
    fun submitAndAwaitShouldReturnQueuedTaskWhenNoCompatibleNodeWithinMaxRounds() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val task = runtime.service.submitAndAwait(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "model/submit-await-queued"),
                    taskMeta = TaskMeta(solverType = "gurobi")
                ),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                maxRounds = UInt64(2)
            )

            assertEquals(TaskStatus.QUEUED, task.status)
        }
    }

    @Test
    fun submitAndAwaitShouldThrowWhenNotTerminalAndThrowIfNotTerminalEnabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                runtime.service.submitAndAwait(
                    payload = SolvePayload(
                        modelRef = ObjectRef.of(path = "model/submit-await-throw"),
                        taskMeta = TaskMeta(solverType = "gurobi")
                    ),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    maxRounds = UInt64(1),
                    throwIfNotTerminal = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.NO_COMPATIBLE_NODE_AVAILABLE, error.code)
        assertTrue(error.message.contains("no compatible online node"))
    }

    @Test
    fun submitAndAwaitShouldThrowBudgetFailedCodeWhenThrowIfFailedEnabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 5000L
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-await-budget-failed",
                    solverType = "gurobi",
                    performanceScore = 1.0,
                    pricePerSecond = 10.0,
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
                runtime.service.submitAndAwait(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "model/submit-await-budget-failed")),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    budgetLimit = Flt64(1.0),
                    maxRounds = UInt64(5),
                    throwIfFailed = true
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED, error.code)
    }

    @Test
    fun performanceScoreShouldLearnFromSliceFeedbackWhenEnabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 2000L,
                performanceLearningEnabled = true,
                performanceLearningRate = 0.2
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-learning-enabled",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/performance-learning-enabled")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )

            repeat(10) {
                runtime.service.scheduleOnce()
            }

            val finalTask = runtime.service.getTask(task.taskId)
            assertNotNull(finalTask)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)
            val updatedNode = runtime.nodeStatePort.getNode("node-learning-enabled")
            assertNotNull(updatedNode)
            assertTrue(updatedNode.profile.performanceScore.toDouble() > 1.0)
        }
    }

    @Test
    fun performanceScoreShouldStayUnchangedWhenLearningDisabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                simpleTaskQuantumMs = 2000L,
                performanceLearningEnabled = false
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-learning-disabled",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/performance-learning-disabled")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1
            )

            repeat(10) {
                runtime.service.scheduleOnce()
            }

            val finalTask = runtime.service.getTask(task.taskId)
            assertNotNull(finalTask)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)
            val updatedNode = runtime.nodeStatePort.getNode("node-learning-disabled")
            assertNotNull(updatedNode)
            assertEquals(1.0, updatedNode.profile.performanceScore.toDouble())
        }
    }

    @Test
    fun schedulerHotReloadShouldUpdateRuntimeConfigAndAppendAudit() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                schedulerConfigVersion = "v-base",
                schedulerHotReloadEnabled = true,
                simpleTaskQuantumMs = 2000L
            )
        )

        runSuspend {
            val before = runtime.service.schedulerRuntimeConfig()
            assertEquals(2000L, before.simpleTaskQuantumMs)
            assertEquals("v-base", runtime.service.schedulerConfigVersion())

            val audit = runtime.service.applySchedulerHotReload(
                changeSet = mapOf(
                    "scheduler.simple-task-quantum-ms" to "3500",
                    "scheduler.complex-urgency-weight" to "0.7"
                ),
                operator = "tester"
            )

            val after = runtime.service.schedulerRuntimeConfig()
            assertEquals(3500L, after.simpleTaskQuantumMs)
            assertEquals(0.7, after.complexUrgencyWeight)
            assertEquals(audit.version, runtime.service.schedulerConfigVersion())
            assertTrue(runtime.service.listSchedulerConfigAudits().isNotEmpty())
        }
    }

    @Test
    fun schedulerHotReloadShouldSupportRollback() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                schedulerConfigVersion = "v-base",
                schedulerHotReloadEnabled = true,
                simpleTaskQuantumMs = 2000L
            )
        )

        runSuspend {
            val before = runtime.service.schedulerRuntimeConfig()
            val update = runtime.service.applySchedulerHotReload(
                changeSet = mapOf("scheduler.simple-task-quantum-ms" to "3000"),
                operator = "tester"
            )
            assertEquals(3000L, runtime.service.schedulerRuntimeConfig().simpleTaskQuantumMs)

            runtime.service.rollbackSchedulerHotReload(
                targetVersion = "v-base",
                operator = "tester"
            )
            assertEquals(before.simpleTaskQuantumMs, runtime.service.schedulerRuntimeConfig().simpleTaskQuantumMs)
            assertTrue(runtime.service.schedulerConfigVersion() != update.version)
        }
    }

    @Test
    fun schedulerHotReloadShouldRejectUnsupportedKeys() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                schedulerConfigVersion = "v-base",
                schedulerHotReloadEnabled = true
            )
        )

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                runtime.service.applySchedulerHotReload(
                    changeSet = mapOf("scheduler.unknown-key" to "1"),
                    operator = "tester"
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.INVALID_ARGUMENT, error.code)
    }

    @Test
    fun schedulerHotReloadShouldFailWhenDisabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                schedulerConfigVersion = "v-base",
                schedulerHotReloadEnabled = false
            )
        )

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                runtime.service.applySchedulerHotReload(
                    changeSet = mapOf("scheduler.simple-task-quantum-ms" to "2100"),
                    operator = "tester"
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.INVALID_ARGUMENT, error.code)
    }

    @Test
    fun budgetExceededShouldTryCheaperNodeBeforeFail() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            // Register expensive node (faster due to higher performance)
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-expensive",
                    solverType = "gurobi",
                    performanceScore = 10.0,  // Very fast - eta ~2s, satisfies tight deadline
                    pricePerSecond = 1.0,    // Expensive: $60 per slice (minBilling 60s)
                    minBillingUnitSeconds = 60L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )

            // Register cheap node (very slow, won't satisfy tight deadline)
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-cheap",
                    solverType = "gurobi",
                    performanceScore = 0.1,  // Very slow - eta ~200s, can't satisfy 10s deadline
                    pricePerSecond = 0.1,    // Cheap: $6 per slice (60s * $0.1)
                    minBillingUnitSeconds = 60L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )

            // Submit task with tight deadline - only expensive node can satisfy
            // Budget = $10, passes checkBudget but expensive node costs $60 -> wouldExceedBudget
            val now = System.currentTimeMillis()
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/budget-degrade")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.REALTIME,
                priority = 5,
                budgetLimit = Flt64(10.0),  // Not enough for expensive node ($60)
                deadline = Instant.fromEpochMilliseconds(now + 10_000)  // 10 seconds - only fast node (eta 2s) can satisfy
            )

            // Schedule - scheduler must pick expensive node due to deadline constraint
            // Then wouldExceedBudget fails, should degrade to cheap node (ignoring deadline)
            val scheduled = runtime.service.scheduleOnce()
            assertNotNull(scheduled)
            // After budget degradation, should be assigned to cheap node, not failed
            assertEquals("node-cheap", scheduled.assignedNodeId?.value, "Should degrade to cheaper node instead of failing")
            assertFalse(scheduled.status == TaskStatus.FAILED, "Task should not fail when cheaper node available")
        }
    }

    @Test
    fun budgetExceededShouldWaitForBudgetWhenNoCheaperNodeAvailable() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            // Register only expensive node
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-only-expensive",
                    solverType = "gurobi",
                    performanceScore = 1.0,
                    pricePerSecond = 1.0,    // $60 per slice (minBilling 60s)
                    minBillingUnitSeconds = 60L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )

            // Submit task with budgetLimit higher than node cost, but consumed causes shortage
            // budgetLimit = 100.0 > $60 (node cost)
            // After consumedCost = 50.0, remaining = 50.0 < $60 -> wouldExceedBudget
            // Budget refresh (refund) could help -> should wait
            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/budget-wait")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 5,
                budgetLimit = Flt64(100.0)  // Higher than node cost $60
            )

            // Consume budget to cause shortage
            runtime.taskStatePort.upsertTask(
                task.copy(consumedCost = Flt64(50.0))  // remaining = 50 < $60
            )

            // Schedule - should wait for budget refund instead of failing
            runtime.service.scheduleOnce()

            // Task should be in WAITING_FOR_BUDGET status, not FAILED
            val finalTask = runtime.service.getTask(task.taskId)
            assertNotNull(finalTask)
            assertTrue(
                finalTask.status == TaskStatus.WAITING_FOR_BUDGET || finalTask.status == TaskStatus.QUEUED,
                "Task should be waiting for budget or still queued, not failed. Status: ${finalTask.status}"
            )
            assertFalse(finalTask.status == TaskStatus.FAILED, "Task should not fail when budget could refresh via refund")
        }
    }

    @Test
    fun waitingForBudgetTaskShouldRequeueBeforeDispatch() {
        val runtime = InMemoryRemoteSolverBootstrap.create()

        runSuspend {
            val controlEvents = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.SOLVING_CONTROL, "group-budget-requeue-test") { record ->
                controlEvents.add(record.payload.decodeToString())
            }
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-budget-requeue",
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
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/simple-budget-requeue")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME,
                priority = 1,
                budgetLimit = Flt64(10.0)
            )
            runtime.taskStatePort.upsertTask(
                task.copy(
                    status = TaskStatus.WAITING_FOR_BUDGET,
                    updatedAt = Instant.fromEpochMilliseconds(System.currentTimeMillis())
                )
            )

            val scheduled = runtime.service.scheduleOnce()

            assertNotNull(scheduled)
            assertEquals(TaskStatus.COMPLETED, scheduled.status)
            assertTrue(
                controlEvents.any {
                    it.contains("\"taskId\":\"${task.taskId}\"") &&
                        it.contains("\"action\":\"budget_requeue\"") &&
                        it.contains("\"status\":\"QUEUED\"")
                }
            )
        }
    }
}
