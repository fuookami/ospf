@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 远程求解器服务
 *
 * 本模块提供远程求解器的核心业务逻辑，
 * 负责任务调度、节点管理、成本计算等核心功能。
 * 是整个远程求解器系统的核心服务层。
 *
 * Remote Solver Service
 *
 * This module provides the core business logic for the remote solver,
 * responsible for task scheduling, node management, cost calculation, and other core functions.
 * It is the core service layer of the entire remote solver system.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.BudgetSnapshot
import fuookami.ospf.framework.remote_solver.domain.BudgetReservation
import fuookami.ospf.framework.remote_solver.domain.LockLease
import fuookami.ospf.framework.remote_solver.domain.CostRecord
import fuookami.ospf.framework.remote_solver.domain.CostSummary
import fuookami.ospf.framework.remote_solver.domain.EventEnvelopeHeaders
import fuookami.ospf.framework.remote_solver.domain.EventTopics
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.domain.SliceLifecyclePayload
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.domain.TaskDispatchPayload
import fuookami.ospf.framework.remote_solver.domain.TaskResultPayload
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.domain.TaskSummaryPayload
import fuookami.ospf.framework.remote_solver.domain.CostUpdatePayload
import fuookami.ospf.framework.remote_solver.domain.HeartbeatPayload
import fuookami.ospf.framework.remote_solver.domain.SolvingControlPayload
import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.DispatchId
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorMapper
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteResultValidator
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SchedulingDecision
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceOutcome
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.framework.remote_solver.port.BudgetPort
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort
import fuookami.ospf.framework.remote_solver.port.EventPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import fuookami.ospf.framework.remote_solver.port.NodeStatePort
import fuookami.ospf.framework.remote_solver.port.SchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import fuookami.ospf.framework.remote_solver.port.TracingPort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.min
import kotlin.time.DurationUnit
import kotlin.time.toDuration

/**
 * 远程求解器服务
 *
 * 远程求解器的核心业务服务，负责：
 * - 任务提交、调度和生命周期管理
 * - 节点注册、心跳和健康检查
 * - 预算管理和成本计算
 * - 调度器配置热更新
 * - 性能学习和优化
 *
 * Remote solver service.
 *
 * Core business service for the remote solver, responsible for:
 * - Task submission, scheduling, and lifecycle management
 * - Node registration, heartbeat, and health check
 * - Budget management and cost calculation
 * - Scheduler configuration hot reload
 * - Performance learning and optimization
 *
 * @param schedulerEngine 调度器引擎，用于节点选择决策
 *                         Scheduler engine for node selection decisions
 * @param taskStatePort 任务状态端口，用于任务持久化
 *                       Task state port for task persistence
 * @param nodeStatePort 节点状态端口，用于节点状态管理
 *                       Node state port for node state management
 * @param eventPort 事件端口，用于事件发布
 *                   Event port for event publishing
 * @param checkpointPort 检查点端口，用于检查点存储
 *                        Checkpoint port for checkpoint storage
 * @param budgetPort 预算端口，用于预算管理
 *                    Budget port for budget management
 * @param costLedgerPort 成本账本端口，用于成本记录
 *                         Cost ledger port for cost recording
 * @param distributedLockPort 分布式锁端口，用于并发控制
 *                              Distributed lock port for concurrency control
 * @param solverExecutionPort 求解器执行端口，用于实际求解
 *                              Solver execution port for actual solving
 * @param metricsPort 指标端口，用于指标记录
 *                     Metrics port for metrics recording
 * @param tracingPort 链路追踪端口，用于分布式追踪
 *                     Tracing port for distributed tracing
 * @param clock 时钟端口，用于时间获取
 *               Clock port for time retrieval
 * @param idGenerator ID 生成器端口，用于 ID 生成
 *                     ID generator port for ID generation
 * @param config 远程求解器配置
 *                Remote solver configuration
 * @param schedulerConfigAuditPort 调度器配置审计端口（可选）
 *                                   Scheduler configuration audit port (optional)
 */
class RemoteSolverService(
    private val schedulerEngine: SchedulerEngine,
    private val taskStatePort: TaskStatePort,
    private val nodeStatePort: NodeStatePort,
    private val eventPort: EventPort,
    private val checkpointPort: CheckpointPort,
    private val budgetPort: BudgetPort,
    private val costLedgerPort: CostLedgerPort,
    private val distributedLockPort: DistributedLockPort,
    private val solverExecutionPort: SolverExecutionPort,
    private val metricsPort: MetricsPort,
    private val tracingPort: TracingPort,
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val config: RemoteSolverConfig = RemoteSolverConfig(),
    private val schedulerConfigAuditPort: SchedulerConfigAuditPort? = null,
    private val objectStoragePort: ObjectStoragePort? = null
) {
    companion object {
        private val TERMINAL_STATUSES = setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)
    }

    /** Raised when a late worker result no longer owns the dispatch fence. */
    private class StaleExecutionException : Exception()

    /** Mutable state shared by the lease, admission, and dispatch helpers. */
    private class DispatchAttempt(
        val leaseRef: AtomicReference<LockLease>,
        val leaseLost: AtomicReference<Boolean>,
        val leaseRenewalJob: Job
    ) {
        var reservedSliceId: SliceId? = null
        var slotAcquired: Boolean = false
        var occupiedNodeId: NodeId? = null
        var ownershipTransferred: Boolean = false
    }

    private data class DispatchSelection(
        val task: TaskState,
        val availableNodes: List<NodeState>,
        val selectedNode: NodeState,
        val selectionDecision: NodeSelectionDecision
    )

    private sealed class DispatchNodeChoice {
        data class Selected(val selection: DispatchSelection) : DispatchNodeChoice()
        data class Finished(val task: TaskState) : DispatchNodeChoice()
        object Skip : DispatchNodeChoice()
    }

    private data class DispatchBudgetPlan(
        val selectedNode: NodeState,
        val selectionDecision: NodeSelectionDecision,
        val quantumDecision: QuantumDecision,
        val quantumMs: Long
    )

    private sealed class DispatchBudgetChoice {
        data class Proceed(val plan: DispatchBudgetPlan) : DispatchBudgetChoice()
        data class Finished(val task: TaskState) : DispatchBudgetChoice()
        object Skip : DispatchBudgetChoice()
    }

    private val runningHandleByTaskId = ConcurrentHashMap<String, ExecutionHandle>()
    /** Current dispatch generation for each task (the slice id is the fence). */
    private val runningSliceByTaskId = ConcurrentHashMap<String, String>()
    private val latestGapByTaskId = ConcurrentHashMap<String, Double>()
    private val progressSignalByTaskId = ConcurrentHashMap<String, ProgressSignal>()
    private val lastDeadlineUrgencyByTaskId = ConcurrentHashMap<String, Double>()
    /** Last dispatched task per fairness cohort; used to rotate equal-priority work. */
    private val taskRoundRobinCursorByClass = ConcurrentHashMap<String, String>()
    /** Last dispatch time used by the starvation guard across priority cohorts. */
    private val lastDispatchAtByTaskId = ConcurrentHashMap<String, Long>()
    /** Dispatch-scoped budget reservations held until actual slice reconciliation. */
    private val budgetReservationBySliceId = ConcurrentHashMap<String, BudgetReservation>()
    private val dispatcherId = idGenerator.newId("dispatcher")
    private val runtimeSchedulerConfigRef = AtomicReference(SchedulerRuntimeConfig.from(config))
    private val schedulerConfigVersionRef = AtomicReference(config.schedulerConfigVersion.ifBlank { "v0" })
    private val schedulerHotReloadSeq = AtomicInteger(0)
    private val schedulerSnapshots = ConcurrentHashMap<String, SchedulerRuntimeConfig>()
    private val schedulerHotReloadAudits = CopyOnWriteArrayList<SchedulerHotReloadAuditRecord>()
    private val schedulerConfigMutationMutex = Mutex()

    init {
        schedulerSnapshots[schedulerConfigVersionRef.get()] = runtimeSchedulerConfigRef.get()
        if (schedulerConfigAuditPort != null) {
            runBlocking {
                val persisted = schedulerConfigAuditPort.list(Int.MAX_VALUE)
                if (persisted.isNotEmpty()) {
                    persisted.forEach { record ->
                        schedulerHotReloadAudits.add(record)
                        schedulerConfigAuditPort.getSnapshot(record.version)?.let { snapshot ->
                            schedulerSnapshots[record.version] = snapshot
                        }
                    }
                    // Audit ports return records in successful append order.
                    // Effective timestamps are caller supplied and may move
                    // backwards, so they cannot identify the latest version.
                    val latest = persisted.lastOrNull()
                    if (latest != null) {
                        schedulerSnapshots[latest.version]?.let { snapshot ->
                            runtimeSchedulerConfigRef.set(snapshot)
                            schedulerConfigVersionRef.set(latest.version)
                        }
                    }
                }
            }
        }
        schedulerEngine.updateWeights(SchedulerWeights.from(runtimeSchedulerConfigRef.get()))
    }

    fun schedulerConfigVersion(): String = schedulerConfigVersionRef.get()

    fun schedulerRuntimeConfig(): SchedulerRuntimeConfig = runtimeSchedulerConfigRef.get()

    fun taskStatePort(): TaskStatePort = taskStatePort

    fun nodeStatePort(): NodeStatePort = nodeStatePort

    fun checkpointPort(): CheckpointPort = checkpointPort

    fun costLedgerPort(): CostLedgerPort = costLedgerPort

    fun eventPort(): EventPort = eventPort

    fun clockPort(): ClockPort = clock

    fun nodeHeartbeatTimeoutMs(): Long = config.nodeHeartbeatTimeoutMs

    fun listSchedulerConfigAudits(limit: Int = 100): List<SchedulerHotReloadAuditRecord> {
        val safeLimit = limit.coerceAtLeast(1)
        val all = schedulerHotReloadAudits.toList()
        if (all.size <= safeLimit) {
            return all
        }
        return all.takeLast(safeLimit)
    }

    suspend fun applySchedulerHotReload(
        changeSet: Map<String, String>,
        operator: String,
        effectiveAtEpochMs: Long = clock.nowEpochMs(),
        requestedVersion: String? = null
    ): SchedulerHotReloadAuditRecord {
        ensureHotReloadEnabled()
        return schedulerConfigMutationMutex.withLock {
            val normalizedOperator = operator.trim().ifEmpty { "unknown" }
            val previousVersion = schedulerConfigVersionRef.get()
            val previous = runtimeSchedulerConfigRef.get()
            val next = applySchedulerChangeSet(previous, changeSet)
            val newVersion = requestedVersion?.trim()?.takeIf { it.isNotEmpty() }
                ?: nextSchedulerVersion("hot", effectiveAtEpochMs)
            requireSchedulerVersionAvailable(newVersion, previousVersion)
            val audit = SchedulerHotReloadAuditRecord(
                version = newVersion,
                previousVersion = previousVersion,
                operator = normalizedOperator,
                effectiveAtEpochMs = effectiveAtEpochMs,
                changeSet = changeSet.toMap(),
                rollbackFromVersion = null
            )

            // Persist the complete record before publishing it to workers.
            // A failed audit write therefore leaves the old runtime active.
            schedulerConfigAuditPort?.let {
                it.saveSnapshot(newVersion, next)
                it.append(audit)
            }
            publishSchedulerConfig(previousVersion, previous, newVersion, next, audit)
            audit
        }
    }

    suspend fun rollbackSchedulerHotReload(
        targetVersion: String,
        operator: String,
        effectiveAtEpochMs: Long = clock.nowEpochMs(),
        requestedVersion: String? = null
    ): SchedulerHotReloadAuditRecord {
        ensureHotReloadEnabled()
        return schedulerConfigMutationMutex.withLock {
            val normalizedTargetVersion = targetVersion.trim()
            require(normalizedTargetVersion.isNotEmpty()) { "targetVersion must not be blank" }
            val snapshot = schedulerSnapshots[normalizedTargetVersion]
                ?: throw RemoteSolverException(
                    code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                    message = "Unknown scheduler config version '$normalizedTargetVersion'"
                )
            val previousVersion = schedulerConfigVersionRef.get()
            val previous = runtimeSchedulerConfigRef.get()
            val newVersion = requestedVersion?.trim()?.takeIf { it.isNotEmpty() }
                ?: nextSchedulerVersion("rollback-$normalizedTargetVersion", effectiveAtEpochMs)
            requireSchedulerVersionAvailable(newVersion, previousVersion)
            val audit = SchedulerHotReloadAuditRecord(
                version = newVersion,
                previousVersion = previousVersion,
                operator = operator.trim().ifEmpty { "unknown" },
                effectiveAtEpochMs = effectiveAtEpochMs,
                changeSet = snapshot.toChangeSet(),
                rollbackFromVersion = normalizedTargetVersion
            )
            schedulerConfigAuditPort?.let {
                it.saveSnapshot(newVersion, snapshot)
                it.append(audit)
            }
            publishSchedulerConfig(previousVersion, previous, newVersion, snapshot, audit)
            audit
        }
    }

    private fun nextSchedulerVersion(prefix: String, effectiveAtEpochMs: Long): String {
        var candidate: String
        do {
            candidate = "$prefix-$effectiveAtEpochMs-${schedulerHotReloadSeq.incrementAndGet()}"
        } while (schedulerSnapshots.containsKey(candidate) || schedulerHotReloadAudits.any { it.version == candidate })
        return candidate
    }

    private fun requireSchedulerVersionAvailable(newVersion: String, previousVersion: String) {
        require(newVersion.matches(Regex("[A-Za-z0-9_.-]+"))) {
            "scheduler version contains illegal characters"
        }
        require(newVersion != previousVersion) {
            "requestedVersion must be different from current version '$previousVersion'"
        }
        require(!schedulerSnapshots.containsKey(newVersion) && schedulerHotReloadAudits.none { it.version == newVersion }) {
            "scheduler version '$newVersion' already exists"
        }
    }

    private fun publishSchedulerConfig(
        previousVersion: String,
        previous: SchedulerRuntimeConfig,
        newVersion: String,
        next: SchedulerRuntimeConfig,
        audit: SchedulerHotReloadAuditRecord
    ) {
        check(runtimeSchedulerConfigRef.compareAndSet(previous, next)) {
            "scheduler runtime changed while publishing version '$newVersion'"
        }
        if (!schedulerConfigVersionRef.compareAndSet(previousVersion, newVersion)) {
            runtimeSchedulerConfigRef.compareAndSet(next, previous)
            error("scheduler version changed while publishing version '$newVersion'")
        }
        try {
            schedulerEngine.updateWeights(SchedulerWeights.from(next))
        } catch (error: Throwable) {
            schedulerConfigVersionRef.compareAndSet(newVersion, previousVersion)
            runtimeSchedulerConfigRef.compareAndSet(next, previous)
            throw error
        }
        schedulerSnapshots[newVersion] = next
        schedulerHotReloadAudits.add(audit)
    }

    suspend fun registerNode(profile: NodeCapabilityProfile) {
        val now = clock.now()
        nodeStatePort.upsertNode(
            NodeState(
                nodeId = profile.nodeId,
                profile = profile,
                availableUnits = profile.parallelUnits,
                lastHeartbeat = now,
                online = true
            )
        )
    }

    suspend fun submitTask(
        payload: SolvePayload,
        complexity: TaskComplexity? = null,
        timeSensitivity: TimeSensitivity? = null,
        priority: Int = 0,
        deadline: kotlin.time.Instant? = null,
        budgetLimit: Flt64? = null,
        budgetScope: BudgetScopeId? = null,
        tenantId: TenantId? = null,
        requestId: RequestId? = null
    ): TaskState {
        val now = clock.now()
        val stableRequestId = requestId ?: RequestId.of(idGenerator.newId("req"))
        val resolvedTenantId = resolveTenantId(tenantId, payload)

        val scheduling = payload.scheduling
        val resolvedComplexity = scheduling?.complexity ?: complexity ?: inferComplexity(payload)
        val resolvedTimeSensitivity = scheduling?.timeSensitivity ?: timeSensitivity ?: TimeSensitivity.NON_REALTIME
        val (normalizedTimeSensitivity, downgradedRealtimeComplexTask) =
            normalizeSubmission(resolvedComplexity, resolvedTimeSensitivity)
        val resolvedPriority = scheduling?.priority ?: priority
        val resolvedDeadline = scheduling?.deadline ?: deadline
        val resolvedBudgetLimit = scheduling?.budgetLimit ?: budgetLimit
        val resolvedBudgetScope = scheduling?.budgetScope ?: budgetScope
        val taskId = TaskId.of(idGenerator.newId("task"))
        val scope = normalizeBudgetScope(resolvedBudgetScope, resolvedTenantId, taskId)
        val task = TaskState(
            taskId = taskId,
            requestId = stableRequestId,
            tenantId = resolvedTenantId,
            status = TaskStatus.QUEUED,
            complexity = resolvedComplexity,
            timeSensitivity = normalizedTimeSensitivity,
            priority = resolvedPriority,
            deadline = resolvedDeadline,
            payload = payload,
            createdAt = now,
            updatedAt = now,
            budgetScope = scope,
            budgetLimit = resolvedBudgetLimit,
            latestSnapshotRef = scheduling?.checkpointRef
        )
        // The identity check and insert must be one persistence operation.  A
        // get-then-upsert sequence allows two dispatchers to publish duplicate
        // submission events and overwrite the winner's task state.
        val insertResult = taskStatePort.insertIfAbsent(task)
        if (!insertResult.inserted) {
            return insertResult.task
        }
        if (resolvedBudgetLimit != null) {
            budgetPort.configureBudget(scope.value, resolvedBudgetLimit.toDouble())
        }
        publishEvent(
            topic = EventTopics.SOLVING_REQUEST,
            key = task.taskId,
            payload = task.summaryPayload(),
            tenantId = task.tenantId,
            idempotencyKey = "request:${task.requestId}:solving_request"
        )
        if (downgradedRealtimeComplexTask) {
            publishEvent(
                topic = EventTopics.SOLVING_CONTROL,
                key = task.taskId,
                payload = SolvingControlPayload(
                    taskId = task.taskId.value,
                    action = "downgrade",
                    from = TimeSensitivity.REALTIME.name,
                    to = TimeSensitivity.NON_REALTIME.name,
                    reason = "complex_realtime_policy"
                ).toByteArray(),
                tenantId = task.tenantId,
                idempotencyKey = "task:${task.taskId}:downgrade"
            )
        }
        return task
    }

    suspend fun submitAndAwait(
        payload: SolvePayload,
        complexity: TaskComplexity? = null,
        timeSensitivity: TimeSensitivity? = null,
        priority: Int = 0,
        deadline: kotlin.time.Instant? = null,
        budgetLimit: Flt64? = null,
        budgetScope: BudgetScopeId? = null,
        tenantId: TenantId? = null,
        requestId: RequestId? = null,
        maxRounds: UInt64 = UInt64(100),
        throwIfNotTerminal: Boolean = false,
        throwIfFailed: Boolean = false
    ): TaskState {
        require(maxRounds > UInt64.zero) { "maxRounds must be positive." }
        val task = submitTask(
            payload = payload,
            complexity = complexity,
            timeSensitivity = timeSensitivity,
            priority = priority,
            deadline = deadline,
            budgetLimit = budgetLimit,
            budgetScope = budgetScope,
            tenantId = tenantId,
            requestId = requestId
        )
        var latest = taskStatePort.getTask(task.taskId) ?: task
        var rounds = UInt64.zero
        while (rounds < maxRounds && latest.status !in TERMINAL_STATUSES) {
            scheduleOnce()
            latest = taskStatePort.getTask(task.taskId) ?: latest
            rounds += UInt64.one
        }
        if (throwIfFailed && latest.status == TaskStatus.FAILED) {
            throw resolveAwaitException(task = latest, maxRounds = maxRounds)
        }
        if (throwIfNotTerminal && latest.status !in TERMINAL_STATUSES) {
            throw resolveAwaitException(task = latest, maxRounds = maxRounds)
        }
        return latest
    }

    suspend fun scheduleOnce(): TaskState? =
        tracingPort.inSpan("schedule_once") {
            reconcileNodeHealth()
            val candidates = taskStatePort.listTasks(
                statuses = setOf(TaskStatus.QUEUED, TaskStatus.SUSPENDED, TaskStatus.ACCEPTED, TaskStatus.WAITING_FOR_BUDGET),
                limit = config.maxSchedulingBatch
            )
            if (candidates.isEmpty()) {
                return@inSpan null
            }
            for (task in rankCandidates(candidates)) {
                val hardTimeoutReason = hardTimeoutReason(task)
                if (hardTimeoutReason != null) {
                    metricsPort.increment("task.failed", tags = mapOf("reason" to "hard_timeout"))
                    return@inSpan markFailed(task, hardTimeoutReason, RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT)
                }
                val execution = tryScheduleTask(task)
                if (execution != null) {
                    return@inSpan execution
                }
            }
            null
        }

    suspend fun runUntilIdle(maxRounds: Int = 100): Int {
        var rounds = 0
        while (rounds < maxRounds) {
            val updated = scheduleOnce() ?: break
            rounds += 1
            if (updated.status == TaskStatus.COMPLETED || updated.status == TaskStatus.FAILED) {
                continue
            }
        }
        return rounds
    }

    suspend fun getTask(taskId: String): TaskState? = taskStatePort.getTask(taskId)

    suspend fun getTask(taskId: TaskId): TaskState? = taskStatePort.getTask(taskId)

    suspend fun getSlices(taskId: String): List<SliceState> = taskStatePort.getSlices(taskId)

    suspend fun getSlices(taskId: TaskId): List<SliceState> = taskStatePort.getSlices(taskId)

    suspend fun stopTask(
        taskId: String,
        reason: String = "Stopped by user",
        operator: String? = null,
        source: String? = null
    ): TaskState? = stopTask(TaskId.of(taskId), reason, operator, source)

    suspend fun stopTask(
        taskId: TaskId,
        reason: String = "Stopped by user",
        operator: String? = null,
        source: String? = null
    ): TaskState? =
        tracingPort.inSpan("stop_task") {
            val task = taskStatePort.getTask(taskId) ?: return@inSpan null
            if (task.status in setOf(TaskStatus.STOPPED, TaskStatus.COMPLETED, TaskStatus.FAILED)) {
                return@inSpan task
            }

            val now = clock.now()
            val activeSlices = taskStatePort.getSlices(task.taskId)
                .filter { it.status in setOf(SliceStatus.PLANNED, SliceStatus.RUNNING, SliceStatus.CHECKPOINTING) }
            val runningGeneration = runningSliceByTaskId[task.taskId.value]
            val slotNodeId = task.assignedNodeId
            val ownsExecutionSlot = slotNodeId != null && (
                task.status in setOf(TaskStatus.DISPATCHING, TaskStatus.RUNNING) ||
                    activeSlices.any { it.sliceId.value == runningGeneration }
                )
            publishEvent(
                topic = EventTopics.SOLVING_CONTROL,
                key = task.taskId,
                payload = SolvingControlPayload(
                    taskId = task.taskId.value,
                    action = "stop",
                    status = "stopping",
                    operator = operator,
                    source = source
                ).toByteArray(),
                tenantId = task.tenantId,
                idempotencyKey = "task:${task.taskId}:stop:request"
            )

            if (task.status == TaskStatus.RUNNING) {
                val hardInterrupt = shouldUseHardInterrupt(task)
                taskStatePort.upsertTask(
                    task.copy(
                        status = TaskStatus.STOPPING,
                        updatedAt = now
                    )
                )
                if (hardInterrupt) {
                    runningHandleByTaskId[task.taskId.value]?.let { handle ->
                        try {
                            solverExecutionPort.stop(handle)
                        } catch (_: Exception) {
                        }
                    }
                }
                runningHandleByTaskId.remove(task.taskId.value)
            }

            runningSliceByTaskId.remove(task.taskId.value)
            activeSlices.forEach { slice ->
                val stoppedSlice = slice.copy(
                    status = SliceStatus.FAILED,
                    finishedAt = now,
                    error = reason
                )
                taskStatePort.updateSlice(stoppedSlice)
                publishSliceLifecycle(
                    slice = stoppedSlice,
                    action = "slice-stop",
                    taskStatus = TaskStatus.STOPPED,
                    tenantId = task.tenantId,
                    reason = reason,
                    idempotencySuffix = "stop"
                )
            }

            val stopped = task.copy(
                status = TaskStatus.STOPPED,
                assignedNodeId = null,
                latestResult = SolveResult(
                    feasible = task.latestResult?.feasible ?: false,
                    optimal = false,
                    objectiveValue = task.latestResult?.objectiveValue,
                    objectiveValueInt64 = task.latestResult?.objectiveValueInt64,
                    gap = task.latestResult?.gap,
                    elapsed = (task.latestResult?.elapsedMs ?: 0L).toDuration(DurationUnit.MILLISECONDS),
                    checkpointRef = task.effectiveCheckpointRef(),
                    resultRef = null,
                    message = reason,
                    extension = (task.latestResult?.extension ?: emptyMap()) + ("reasonCode" to "CANCELLED"),
                    schemaVersion = task.latestResult?.schemaVersion
                        ?.takeIf { it.substringBefore('.').toIntOrNull() == 2 }
                        ?: "2.0",
                    problemStatus = if (task.latestResult?.feasible == true) {
                        RemoteProblemStatus.FEASIBLE
                    } else {
                        RemoteProblemStatus.UNKNOWN
                    },
                    terminationReason = RemoteTerminationReason.CANCELLED,
                    solutionPresence = if (task.latestResult?.feasible == true) {
                        RemoteSolutionPresence.INCUMBENT
                    } else {
                        RemoteSolutionPresence.NONE
                    },
                    proofStatus = RemoteProofStatus.NONE,
                    provenance = task.latestResult?.provenance ?: emptyMap(),
                    fingerprints = task.latestResult?.fingerprints ?: emptyMap(),
                    fingerprintSchemas = task.latestResult?.fingerprintSchemas ?: emptyMap(),
                    statistics = task.latestResult?.statistics ?: emptyMap(),
                    diagnostics = task.latestResult?.diagnostics ?: emptyMap(),
                    runId = task.latestResult?.runId,
                    attemptId = task.latestResult?.attemptId,
                    artifactDigest = null,
                    incumbentRef = task.latestResult?.incumbentRef,
                    modelFingerprint = task.latestResult?.modelFingerprint,
                    scheduling = task.latestResult?.scheduling?.copy(
                        outcome = SliceOutcome.CANCELLED,
                        reason = reason
                    ),
                    outcome = SliceOutcome.CANCELLED,
                    cancellationChain = task.latestResult?.cancellationChain ?: emptyList()
                ),
                updatedAt = now
            )
            taskStatePort.upsertTask(stopped)
            clearLearningState(task.taskId.value)
            if (ownsExecutionSlot) {
                slotNodeId?.let { nodeId ->
                    nodeStatePort.releaseUnit(nodeId.value)
                }
            }
            metricsPort.increment("task.stopped")
            publishEvent(
                topic = EventTopics.TASK_RESULT,
                key = stopped.taskId,
                payload = TaskResultPayload(
                    taskId = stopped.taskId.value,
                    status = stopped.status.name,
                    message = reason
                ).toByteArray(),
                tenantId = stopped.tenantId,
                idempotencyKey = "task:${stopped.taskId}:stopped"
            )
            publishEvent(
                topic = EventTopics.SOLVING_CONTROL,
                key = stopped.taskId,
                payload = SolvingControlPayload(
                    taskId = stopped.taskId.value,
                    action = "stop",
                    status = "stopped",
                    operator = operator,
                    source = source
                ).toByteArray(),
                tenantId = stopped.tenantId,
                idempotencyKey = "task:${stopped.taskId}:stop:confirm"
            )
            stopped
        }

    suspend fun resumeTask(
        taskId: String,
        operator: String? = null,
        source: String? = null,
        reason: String? = null
    ): TaskState? = resumeTask(TaskId.of(taskId), operator, source, reason)

    suspend fun resumeTask(
        taskId: TaskId,
        operator: String? = null,
        source: String? = null,
        reason: String? = null
    ): TaskState? =
        tracingPort.inSpan("resume_task") {
            val task = taskStatePort.getTask(taskId) ?: return@inSpan null
            val now = clock.now()
            when (task.status) {
                TaskStatus.STOPPED -> {
                    val resumedStatus = if (task.effectiveCheckpointRef() != null || task.effectiveComplexity() == TaskComplexity.COMPLEX) {
                        TaskStatus.SUSPENDED
                    } else {
                        TaskStatus.QUEUED
                    }
                    val resumed = preserveResumeSourceIdentity(task.copy(
                        status = resumedStatus,
                        assignedNodeId = null,
                        updatedAt = now
                    ))
                    taskStatePort.upsertTask(resumed)
                    publishEvent(
                        topic = EventTopics.SOLVING_CONTROL,
                        key = resumed.taskId,
                        payload = SolvingControlPayload(
                    taskId = resumed.taskId.value,
                            action = "resume",
                            status = resumedStatus.name,
                            operator = operator,
                            source = source,
                            reason = reason
                        ).toByteArray(),
                        tenantId = resumed.tenantId,
                        idempotencyKey = "task:${resumed.taskId}:resume"
                    )
                    resumed
                }

                TaskStatus.SUSPENDED,
                TaskStatus.QUEUED,
                TaskStatus.ACCEPTED -> preserveResumeSourceIdentity(task)

                else -> throw RemoteSolverException(
                    code = RemoteSolverErrorCode.INVALID_TASK_STATE_TRANSITION,
                    message = "Task $taskId cannot be resumed from status ${task.status}.",
                    metadata = mapOf(
                        "taskId" to task.taskId.value,
                        "status" to task.status.name
                    )
                )
            }
        }

    /**
     * Preserve the identity accepted by a resume action. The checkpoint's
     * source slice is the action attempt; a future child slice is created only
     * by the later dispatch path and must never be fabricated here.
     */
    private suspend fun preserveResumeSourceIdentity(task: TaskState): TaskState {
        val checkpoint = checkpointPort.latest(task.taskId) ?: return task
        val previous = task.latestResult
        val fingerprints = (previous?.fingerprints ?: emptyMap()).toMutableMap()
        checkpoint.modelFingerprint?.let { fingerprints.putIfAbsent("model", it) }
        checkpoint.configurationFingerprint?.let { fingerprints.putIfAbsent("configuration", it) }
        checkpoint.solverFingerprint?.let { fingerprints.putIfAbsent("solver", it) }
        val resumedResult = previous?.copy(
            checkpointRef = checkpoint.ref,
            schemaVersion = previous.schemaVersion
                .takeIf { it.substringBefore('.').toIntOrNull() == 2 }
                ?: "2.0",
            runId = task.taskId.value,
            attemptId = checkpoint.sliceId.value,
            modelFingerprint = checkpoint.modelFingerprint ?: previous.modelFingerprint,
            fingerprints = fingerprints,
            provenance = previous.provenance.ifEmpty { checkpoint.provenance },
            cancellationChain = previous.cancellationChain.ifEmpty { checkpoint.cancellationChain }
        ) ?: SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsed = 0L.toDuration(DurationUnit.MILLISECONDS),
            checkpointRef = checkpoint.ref,
            message = "Resume accepted from source checkpoint",
            schemaVersion = "2.0",
            provenance = checkpoint.provenance,
            fingerprintSchemas = mapOf(
                "model" to "1.0",
                "configuration" to "1.0",
                "solver" to "1.0"
            ).filterKeys { fingerprints.containsKey(it) },
            runId = task.taskId.value,
            attemptId = checkpoint.sliceId.value,
            modelFingerprint = checkpoint.modelFingerprint,
            fingerprints = fingerprints,
            outcome = SliceOutcome.RESUMABLE,
            cancellationChain = checkpoint.cancellationChain
        )
        val enriched = task.copy(
            latestResult = resumedResult,
            latestSnapshotRef = checkpoint.ref
        )
        if (enriched != task) {
            taskStatePort.upsertTask(enriched)
        }
        return enriched
    }

    suspend fun heartbeat(nodeId: String) = heartbeat(NodeId.of(nodeId))

    suspend fun heartbeat(nodeId: NodeId) {
        val now = clock.now()
        nodeStatePort.heartbeat(nodeId, now)
        publishEvent(
            topic = EventTopics.SOLVER_HEARTBEAT,
            key = nodeId,
            payload = HeartbeatPayload(
                nodeId = nodeId.value,
                heartbeatAt = now.toEpochMilliseconds()
            ).toByteArray(),
            tenantId = "system",
            idempotencyKey = "heartbeat:$nodeId:${now.toEpochMilliseconds()}"
        )
    }

    suspend fun reconcileNodeHealth(): Int {
        val now = clock.nowEpochMs()
        val staleNodes = nodeStatePort.listNodes(onlineOnly = false)
            .filter { it.online && now - it.lastHeartbeatEpochMs > config.nodeHeartbeatTimeoutMs }
        if (staleNodes.isEmpty()) {
            return 0
        }

        var recoveredTaskCount = 0
        for (node in staleNodes) {
            nodeStatePort.upsertNode(
                node.copy(
                    online = false,
                    availableUnits = 0
                )
            )
            recoveredTaskCount += recoverTimedOutNodeTasks(node.nodeId.value, now)
            publishEvent(
                topic = EventTopics.COST_AND_CAPABILITY_UPDATE,
                key = node.nodeId.value,
                payload = CostUpdatePayload(
                    nodeId = node.nodeId.value,
                    online = false
                ).toByteArray(),
                tenantId = "system",
                idempotencyKey = "node:${node.nodeId.value}:offline"
            )
        }
        return recoveredTaskCount
    }

    suspend fun getTaskCostRecords(taskId: String): List<CostRecord> =
        costLedgerPort.listByTask(taskId)

    suspend fun getTaskCostRecords(taskId: TaskId): List<CostRecord> =
        costLedgerPort.listByTask(taskId.value)

    suspend fun getTaskCostSummary(taskId: String): CostSummary =
        costLedgerPort.summarizeByTask(taskId)

    suspend fun getTaskCostSummary(taskId: TaskId): CostSummary =
        costLedgerPort.summarizeByTask(taskId.value)

    suspend fun getBudgetScopeCostSummary(scope: String): CostSummary =
        costLedgerPort.summarizeByBudgetScope(scope)

    private suspend fun tryScheduleTask(task: TaskState): TaskState? {
        val lease = distributedLockPort.acquire(
            key = "task:${task.taskId}:dispatch",
            owner = dispatcherId,
            ttlMs = config.dispatchLockTtlMs
        ) ?: return null
        val leaseRef = AtomicReference(lease)
        val leaseLost = AtomicReference(false)
        val leaseRenewalJob = startLeaseRenewal(task, leaseRef, leaseLost)
        val attempt = DispatchAttempt(leaseRef, leaseLost, leaseRenewalJob)
        return try {
            scheduleWithLease(task, attempt)
        } finally {
            cleanupDispatchAttempt(task, attempt)
        }
    }

    private suspend fun startLeaseRenewal(
        task: TaskState,
        leaseRef: AtomicReference<LockLease>,
        leaseLost: AtomicReference<Boolean>
    ): Job = CoroutineScope(currentCoroutineContext()).launch {
        val intervalMs = max(1L, config.dispatchLockTtlMs / 3L)
        while (isActive) {
            delay(intervalMs)
            val renewed = distributedLockPort.renew(leaseRef.get(), config.dispatchLockTtlMs)
            if (renewed == null) {
                leaseLost.set(true)
                // A lost dispatch lease fences this generation. Ask the worker
                // to stop promptly; execution will then reject stale results.
                val handle = runningHandleByTaskId[task.taskId.value]
                if (handle != null) {
                    try {
                        solverExecutionPort.stop(handle)
                    } catch (_: Exception) {
                    }
                }
                break
            }
            leaseRef.set(renewed)
        }
    }

    private suspend fun scheduleWithLease(task: TaskState, attempt: DispatchAttempt): TaskState? {
        if (attempt.leaseLost.get()) {
            return null
        }
        val latestTask = taskStatePort.getTask(task.taskId) ?: return null
        if (latestTask.status !in setOf(
                TaskStatus.QUEUED,
                TaskStatus.SUSPENDED,
                TaskStatus.ACCEPTED,
                TaskStatus.WAITING_FOR_BUDGET
            )) {
            return null
        }
        val hardTimeoutReason = hardTimeoutReason(latestTask)
        if (hardTimeoutReason != null) {
            metricsPort.increment("task.failed", tags = mapOf("reason" to "hard_timeout"))
            return markFailed(latestTask, hardTimeoutReason, RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT)
        }

        return when (val choice = chooseDispatchNode(latestTask, attempt)) {
            is DispatchNodeChoice.Selected -> dispatchSelectedTask(choice.selection, attempt)
            is DispatchNodeChoice.Finished -> choice.task
            DispatchNodeChoice.Skip -> null
        }
    }

    private suspend fun chooseDispatchNode(
        task: TaskState,
        attempt: DispatchAttempt
    ): DispatchNodeChoice {
        if (attempt.leaseLost.get()) {
            return DispatchNodeChoice.Skip
        }
        // Admission is evaluated before budget and scoring. Keeping the
        // reasons local distinguishes missing capability from a full node.
        val onlineNodes = nodeStatePort.listNodes(onlineOnly = true)
        val admission = classifyAdmission(task, onlineNodes)
        metricsPort.increment(
            "scheduler.admission",
            tags = mapOf(
                "class" to admission.taskClass.name,
                "reason" to admission.reason.name
            )
        )
        val availableNodes = onlineNodes
            .asSequence()
            .filter { it.availableUnits > 0 }
            .filter { isNodeExecutionCapable(task, it) }
            .toList()
        if (availableNodes.isEmpty() || attempt.leaseLost.get()) {
            return DispatchNodeChoice.Skip
        }

        val budgetEligibleNodes = if (task.effectiveBudgetLimit() == null) {
            availableNodes
        } else {
            availableNodes.filter {
                !wouldExceedBudget(task, it, computeQuantumMs(task, it))
            }
        }
        var selectedNode: NodeState? = null
        var selectionDecision: NodeSelectionDecision? = null
        if (!checkBudget(task) || budgetEligibleNodes.isEmpty()) {
            when (val degradationResult = tryBudgetDegradation(task, availableNodes)) {
                is BudgetDegradationResult.CanProceedWithNode -> {
                    selectedNode = degradationResult.node
                    selectionDecision = budgetSelectionDecision(task, degradationResult.node, availableNodes)
                }
                BudgetDegradationResult.WaitForBudget -> {
                    markSelectionBudgetWait(task)
                    return DispatchNodeChoice.Skip
                }
                BudgetDegradationResult.FailAfterDegradation -> {
                    val failed = markFailed(
                        task,
                        "Budget exceeded after degradation",
                        RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED
                    )
                    metricsPort.increment("task.failed", tags = mapOf("reason" to "budget"))
                    return DispatchNodeChoice.Finished(failed)
                }
            }
        }

        if (selectedNode == null) {
            if (budgetEligibleNodes.isEmpty()) {
                return DispatchNodeChoice.Skip
            }
            val schedulerConfig = runtimeSchedulerConfigRef.get()
            selectionDecision = schedulerEngine.explainNodeSelection(
                task = task,
                candidates = budgetEligibleNodes,
                preferredNodeId = task.assignedNodeId,
                migrationHysteresisRatio = schedulerConfig.schedulerMigrationHysteresisRatio,
                minSlicesBeforeMigration = schedulerConfig.schedulerMinSlicesBeforeMigration,
                migrationCostWeight = schedulerConfig.schedulerMigrationCostWeight,
                slicesExecuted = taskStatePort.getSlices(task.taskId).size,
                progressSignal = taskProgressSignal(task)
            )
            selectedNode = selectionDecision?.selectedNode
        }
        val node = selectedNode ?: return DispatchNodeChoice.Skip
        return DispatchNodeChoice.Selected(
            DispatchSelection(
                task = task,
                availableNodes = availableNodes,
                selectedNode = node,
                selectionDecision = selectionDecision ?: budgetSelectionDecision(task, node, availableNodes)
            )
        )
    }

    private suspend fun markSelectionBudgetWait(task: TaskState) {
        if (task.status == TaskStatus.WAITING_FOR_BUDGET) {
            return
        }
        val waitingTask = task.copy(
            status = TaskStatus.WAITING_FOR_BUDGET,
            updatedAt = clock.now()
        )
        taskStatePort.upsertTask(waitingTask)
        publishEvent(
            topic = EventTopics.SOLVING_CONTROL,
            key = waitingTask.taskId,
            payload = SolvingControlPayload(
                taskId = waitingTask.taskId.value,
                action = "budget_wait",
                status = "waiting",
                reason = "Budget exhausted, waiting for release"
            ).toByteArray(),
            tenantId = waitingTask.tenantId,
            idempotencyKey = "task:${waitingTask.taskId}:budget_wait"
        )
        metricsPort.increment("task.budget_wait")
    }

    private suspend fun dispatchSelectedTask(
        selection: DispatchSelection,
        attempt: DispatchAttempt
    ): TaskState? {
        val dispatchableTask = requeueWaitingForBudget(selection.task) ?: return null
        val selectedNode = selection.selectedNode
        if (!nodeStatePort.occupyUnit(selectedNode.nodeId)) {
            return null
        }
        attempt.slotAcquired = true
        attempt.occupiedNodeId = selectedNode.nodeId
        val acceptedTask = acceptForDispatch(dispatchableTask)
        if (acceptedTask == null) {
            nodeStatePort.releaseUnit(selectedNode.nodeId)
            attempt.slotAcquired = false
            return null
        }
        return prepareDispatch(
            acceptedTask = acceptedTask,
            initialSelection = selection,
            initialNode = selectedNode,
            attempt = attempt
        )
    }

    private suspend fun prepareDispatch(
        acceptedTask: TaskState,
        initialSelection: DispatchSelection,
        initialNode: NodeState,
        attempt: DispatchAttempt
    ): TaskState? {
        val now = clock.now()
        val dispatchId = DispatchId.of(idGenerator.newId("dispatch"))
        val sliceId = SliceId.of(idGenerator.newId("slice"))
        attempt.reservedSliceId = sliceId
        return when (val budgetChoice = prepareDispatchBudget(
            acceptedTask = acceptedTask,
            initialSelection = initialSelection,
            initialNode = initialNode,
            now = now,
            attempt = attempt
        )) {
            is DispatchBudgetChoice.Finished -> budgetChoice.task
            DispatchBudgetChoice.Skip -> null
            is DispatchBudgetChoice.Proceed -> persistAndExecuteDispatch(
                acceptedTask = acceptedTask,
                dispatchId = dispatchId,
                sliceId = sliceId,
                plan = budgetChoice.plan,
                attempt = attempt,
                now = now
            )
        }
    }

    private suspend fun prepareDispatchBudget(
        acceptedTask: TaskState,
        initialSelection: DispatchSelection,
        initialNode: NodeState,
        now: kotlin.time.Instant,
        attempt: DispatchAttempt
    ): DispatchBudgetChoice {
        var selectedNode = initialNode
        var selectionDecision = initialSelection.selectionDecision
        var quantumDecision = computeQuantumDecision(acceptedTask, selectedNode)
        var quantumMs = effectiveQuantumMs(acceptedTask, selectedNode, quantumDecision)
        if (wouldExceedBudget(acceptedTask, selectedNode, quantumMs)) {
            nodeStatePort.releaseUnit(selectedNode.nodeId)
            attempt.slotAcquired = false
            when (val degradationResult = tryBudgetDegradation(acceptedTask, initialSelection.availableNodes)) {
                is BudgetDegradationResult.CanProceedWithNode -> {
                    val cheaperNode = degradationResult.node
                    if (!nodeStatePort.occupyUnit(cheaperNode.nodeId)) {
                        return DispatchBudgetChoice.Skip
                    }
                    attempt.slotAcquired = true
                    attempt.occupiedNodeId = cheaperNode.nodeId
                    selectedNode = cheaperNode
                    selectionDecision = budgetSelectionDecision(
                        acceptedTask,
                        cheaperNode,
                        initialSelection.availableNodes
                    )
                    quantumDecision = computeQuantumDecision(acceptedTask, cheaperNode)
                    quantumMs = effectiveQuantumMs(acceptedTask, cheaperNode, quantumDecision)
                    if (wouldExceedBudget(acceptedTask, cheaperNode, quantumMs)) {
                        nodeStatePort.releaseUnit(cheaperNode.nodeId)
                        attempt.slotAcquired = false
                        if (acceptedTask.status != TaskStatus.WAITING_FOR_BUDGET) {
                            val waitingTask = acceptedTask.copy(
                                status = TaskStatus.WAITING_FOR_BUDGET,
                                updatedAt = now
                            )
                            taskStatePort.upsertTask(waitingTask)
                            publishEvent(
                                topic = EventTopics.SOLVING_CONTROL,
                                key = waitingTask.taskId,
                                payload = SolvingControlPayload(
                                    taskId = waitingTask.taskId.value,
                                    action = "budget_wait",
                                    status = "waiting",
                                    reason = "Budget exhausted after degradation"
                                ).toByteArray(),
                                tenantId = waitingTask.tenantId,
                                idempotencyKey = "task:${waitingTask.taskId}:budget_wait"
                            )
                            metricsPort.increment("task.budget_wait")
                        }
                        return DispatchBudgetChoice.Skip
                    }
                }
                BudgetDegradationResult.WaitForBudget -> {
                    if (acceptedTask.status != TaskStatus.WAITING_FOR_BUDGET) {
                        val waitingTask = acceptedTask.copy(
                            status = TaskStatus.WAITING_FOR_BUDGET,
                            updatedAt = now
                        )
                        taskStatePort.upsertTask(waitingTask)
                        publishEvent(
                            topic = EventTopics.SOLVING_CONTROL,
                            key = waitingTask.taskId,
                            payload = SolvingControlPayload(
                                taskId = waitingTask.taskId.value,
                                action = "budget_wait",
                                status = "waiting",
                                reason = "Budget exhausted, waiting for release"
                            ).toByteArray(),
                            tenantId = waitingTask.tenantId,
                            idempotencyKey = "task:${waitingTask.taskId}:budget_wait"
                        )
                        metricsPort.increment("task.budget_wait")
                    }
                    return DispatchBudgetChoice.Skip
                }
                BudgetDegradationResult.FailAfterDegradation -> {
                    val failed = markFailed(
                        acceptedTask,
                        "Budget exceeded after degradation",
                        RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED
                    )
                    metricsPort.increment("task.failed", tags = mapOf("reason" to "budget"))
                    return DispatchBudgetChoice.Finished(failed)
                }
            }
        }
        return DispatchBudgetChoice.Proceed(
            DispatchBudgetPlan(
                selectedNode = selectedNode,
                selectionDecision = selectionDecision,
                quantumDecision = quantumDecision,
                quantumMs = quantumMs
            )
        )
    }

    private suspend fun persistAndExecuteDispatch(
        acceptedTask: TaskState,
        dispatchId: DispatchId,
        sliceId: SliceId,
        plan: DispatchBudgetPlan,
        attempt: DispatchAttempt,
        now: kotlin.time.Instant
    ): TaskState? {
        val selectedNode = plan.selectedNode
        val budgetReservation = if (acceptedTask.effectiveBudgetLimit() != null) {
            reserveBudgetForDispatch(
                task = acceptedTask,
                dispatchId = dispatchId,
                sliceId = sliceId,
                node = selectedNode,
                quantumMs = plan.quantumMs
            )
        } else {
            null
        }
        if (acceptedTask.effectiveBudgetLimit() != null && budgetReservation == null) {
            nodeStatePort.releaseUnit(selectedNode.nodeId)
            attempt.slotAcquired = false
            return markWaitingForBudget(
                task = acceptedTask,
                reason = "Budget reservation failed; waiting for budget release"
            )
        }

        val currentTask = acceptedTask.copy(
            status = TaskStatus.DISPATCHING,
            assignedNodeId = selectedNode.nodeId,
            updatedAt = now
        )
        taskStatePort.upsertTask(currentTask)
        publishEvent(
            topic = EventTopics.SOLVING_CONTROL,
            key = currentTask.taskId,
            payload = SolvingControlPayload(
                taskId = currentTask.taskId.value,
                action = "confirm",
                status = "dispatching",
                dispatchId = dispatchId.value,
                nodeId = selectedNode.nodeId.value,
                dispatcherId = dispatcherId
            ).toByteArray(),
            tenantId = acceptedTask.tenantId,
            idempotencyKey = "dispatch:$dispatchId:confirm"
        )
        publishEvent(
            topic = EventTopics.TASK_DISPATCH,
            key = acceptedTask.taskId,
            payload = TaskDispatchPayload(
                dispatchId = dispatchId.value,
                taskId = acceptedTask.taskId.value,
                sliceId = sliceId.value,
                nodeId = selectedNode.nodeId.value,
                quantumMs = plan.quantumMs,
                checkpointInRef = acceptedTask.effectiveCheckpointRef()?.path?.value
            ).toByteArray(),
            tenantId = acceptedTask.tenantId,
            idempotencyKey = "dispatch:$dispatchId"
        )

        val currentSlice = SliceState(
            sliceId = sliceId,
            taskId = acceptedTask.taskId,
            dispatchId = dispatchId,
            status = SliceStatus.PLANNED,
            nodeId = selectedNode.nodeId,
            quantum = plan.quantumMs.toDuration(DurationUnit.MILLISECONDS)
        )
        taskStatePort.appendSlice(currentSlice)
        if (attempt.leaseLost.get()) {
            return null
        }
        // Install the slice generation before starting the backend. A late
        // worker from an older generation is then fenced by its slice id.
        runningSliceByTaskId[acceptedTask.taskId.value] = sliceId.value
        rememberTaskDispatch(acceptedTask)
        val decision = plan.selectionDecision
        val sliceAudit = SliceAuditRecord(
            admissionClass = TaskAdmissionClass.of(acceptedTask),
            selectionReason = decision.reason,
            selectionScore = decision.selectedScore,
            quantumReason = plan.quantumDecision.reason,
            migrationDecision = decision.migrationDecision,
            eligibleNodeIds = decision.candidates.filter { it.eligible }.map { it.nodeId }
        )
        attempt.ownershipTransferred = true
        return executeDispatchedSlice(
            acceptedTask = acceptedTask,
            initialTask = currentTask,
            initialSlice = currentSlice,
            selectedNode = selectedNode,
            sliceId = sliceId,
            quantumMs = plan.quantumMs,
            sliceAudit = sliceAudit,
            budgetReservation = budgetReservation,
            leaseLost = attempt.leaseLost
        )
    }

    private suspend fun cleanupDispatchAttempt(task: TaskState, attempt: DispatchAttempt) {
        attempt.leaseRenewalJob.cancel()
        try {
            attempt.leaseRenewalJob.join()
        } catch (_: CancellationException) {
        }
        if (attempt.leaseLost.get() && attempt.ownershipTransferred) {
            recoverLostDispatch(
                taskId = task.taskId,
                sliceId = attempt.reservedSliceId,
                nodeId = attempt.occupiedNodeId
            )
        }
        attempt.reservedSliceId?.let { sliceId ->
            budgetReservationBySliceId.remove(sliceId.value)?.let { reservation ->
                try {
                    releaseBudgetReservation(reservation)
                } catch (_: Exception) {
                }
            }
        }
        if (!attempt.ownershipTransferred && attempt.slotAcquired) {
            try {
                attempt.occupiedNodeId?.let { nodeStatePort.releaseUnit(it) }
            } catch (_: Exception) {
            }
            // A failure before execution ownership transfer must not leave a
            // task stuck in DISPATCHING/ACCEPTED.
            try {
                val current = taskStatePort.getTask(task.taskId)
                if (current != null && current.status in setOf(TaskStatus.ACCEPTED, TaskStatus.DISPATCHING)) {
                    taskStatePort.upsertTask(
                        current.copy(
                            status = TaskStatus.QUEUED,
                            assignedNodeId = null,
                            updatedAt = clock.now()
                        )
                    )
                }
            } catch (_: Exception) {
            }
        }
        distributedLockPort.release(attempt.leaseRef.get())
    }

    private suspend fun executeDispatchedSlice(
        acceptedTask: TaskState,
        initialTask: TaskState,
        initialSlice: SliceState,
        selectedNode: NodeState,
        sliceId: SliceId,
        quantumMs: Long,
        sliceAudit: SliceAuditRecord,
        budgetReservation: BudgetReservation?,
        leaseLost: AtomicReference<Boolean>
    ): TaskState {
        var currentTask = initialTask
        var currentSlice = initialSlice
        var pendingBudgetReservation = budgetReservation
        var activeHandle: ExecutionHandle? = null
        var handleStopped = false
        return try {
            val checkpointForResume = acceptedTask.effectiveCheckpointRef()
            val resumeCapabilityFailure = checkpointResumeAdmissionReason(acceptedTask, selectedNode)
            if (resumeCapabilityFailure != null) {
                throw RemoteSolverException(
                    code = when (resumeCapabilityFailure) {
                        AdmissionReasonCode.CHECKPOINT_UNSUPPORTED,
                        AdmissionReasonCode.WARM_START_UNSUPPORTED ->
                            RemoteSolverErrorCode.NO_COMPATIBLE_NODE_AVAILABLE
                        else -> RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED
                    },
                    message = "Checkpoint cannot be resumed on node ${selectedNode.nodeId.value}: $resumeCapabilityFailure"
                )
            }
            val resumedFromCheckpoint = checkpointForResume != null
            val payload = acceptedTask.payload
            val handle = if (checkpointForResume != null) {
                solverExecutionPort.resume(
                    payload,
                    checkpointForResume,
                    acceptedTask.taskId,
                    sliceId,
                    selectedNode.nodeId,
                    acceptedTask.tenantId
                )
            } else {
                solverExecutionPort.start(payload, acceptedTask.taskId, sliceId, selectedNode.nodeId, acceptedTask.tenantId)
            }
            activeHandle = handle
            if (leaseLost.get() || runningSliceByTaskId[acceptedTask.taskId.value] != sliceId.value) {
                try {
                    solverExecutionPort.stop(handle)
                } catch (_: Exception) {
                }
                handleStopped = true
                throw StaleExecutionException()
            }
            runningHandleByTaskId[acceptedTask.taskId.value] = handle

            if (leaseLost.get() || !isExecutionOwner(acceptedTask.taskId, sliceId, selectedNode.nodeId)) {
                throw StaleExecutionException()
            }

            currentTask = currentTask.copy(status = TaskStatus.RUNNING, updatedAt = clock.now())
            taskStatePort.upsertTask(currentTask)

            currentSlice = currentSlice.copy(status = SliceStatus.RUNNING, startedAt = clock.now())
            taskStatePort.updateSlice(currentSlice)
            publishSliceLifecycle(
                slice = currentSlice,
                action = if (resumedFromCheckpoint) "slice-resume" else "slice-start",
                taskStatus = currentTask.status,
                tenantId = acceptedTask.tenantId,
                audit = sliceAudit,
                idempotencySuffix = if (resumedFromCheckpoint) "resume" else "start"
            )

            currentCoroutineContext().ensureActive()
            var sliceResult = solverExecutionPort.awaitSliceEnd(handle, quantumMs)
            validateSliceResult(sliceResult, acceptedTask.taskId, sliceId)
            if (leaseLost.get() || !isExecutionOwner(acceptedTask.taskId, sliceId, selectedNode.nodeId)) {
                throw StaleExecutionException()
            }
            var sliceTimeoutReason = sliceTimeoutReason(sliceResult, quantumMs)
            var totalElapsedMs = sliceResult.elapsedMs.coerceAtLeast(0L)
            var performanceResult = sliceResult

            // A simple/non-preemptible task must not be converted into a resumable
            // checkpoint merely because a worker returned early. Continue on the
            // same live handle until the worker reaches a terminal result. The
            // task hard timeout and coroutine cancellation are the only bounds;
            // runtime estimates must not turn a valid long solve into a failure.
            // 简单任务/不可抢占任务不能因为 worker 提前返回就伪装成可恢复 checkpoint。
            // 在同一活动句柄上持续等待，只有任务硬超时或协程取消才会中止。
            if (requiresRunToCompletion(acceptedTask) && sliceTimeoutReason == null) {
                while (!isTerminalSliceResult(sliceResult)) {
                    currentCoroutineContext().ensureActive()
                    if (leaseLost.get() || !isExecutionOwner(acceptedTask.taskId, sliceId, selectedNode.nodeId)) {
                        throw StaleExecutionException()
                    }
                    if (hardTimeoutReason(acceptedTask) != null) {
                        break
                    }
                    val nextResult = solverExecutionPort.awaitSliceEnd(handle, quantumMs)
                    validateSliceResult(nextResult, acceptedTask.taskId, sliceId)
                    if (leaseLost.get() || !isExecutionOwner(acceptedTask.taskId, sliceId, selectedNode.nodeId)) {
                        throw StaleExecutionException()
                    }
                    val nextTimeoutReason = sliceTimeoutReason(nextResult, quantumMs)
                    if (nextTimeoutReason != null) {
                        sliceTimeoutReason = nextTimeoutReason
                        totalElapsedMs = saturatingAdd(totalElapsedMs, nextResult.elapsedMs.coerceAtLeast(0L))
                        sliceResult = nextResult.copy(
                            elapsed = totalElapsedMs.toDuration(DurationUnit.MILLISECONDS)
                        )
                        break
                    }
                    performanceResult = nextResult
                    totalElapsedMs = saturatingAdd(totalElapsedMs, nextResult.elapsedMs.coerceAtLeast(0L))
                    sliceResult = nextResult.copy(
                        elapsed = totalElapsedMs.toDuration(DurationUnit.MILLISECONDS)
                    )
                }
            }

            val terminalResult = isTerminalSliceResult(sliceResult)
            if (leaseLost.get() || !isExecutionOwner(acceptedTask.taskId, sliceId, selectedNode.nodeId)) {
                throw StaleExecutionException()
            }
            val checkpointRef = if (!requiresRunToCompletion(acceptedTask) &&
                !terminalResult &&
                selectedNode.profile.supportsCheckpoint
            ) {
                currentSlice = currentSlice.copy(status = SliceStatus.CHECKPOINTING)
                taskStatePort.updateSlice(currentSlice)
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-checkpointing",
                    taskStatus = currentTask.status,
                    tenantId = acceptedTask.tenantId,
                    audit = sliceAudit,
                    idempotencySuffix = "checkpointing"
                )
                try {
                    solverExecutionPort.exportCheckpoint(handle)
                } catch (e: Exception) {
                    throw RemoteSolverException(
                        code = RemoteSolverErrorCode.CHECKPOINT_EXPORT_FAILED,
                        message = e.message ?: "Checkpoint export failed",
                        cause = e
                    )
                }
            } else {
                null
            }

            // A normal quantum return is a completed worker lifecycle. Stop the
            // handle before the node is released and the task is re-queued.
            // 普通 quantum 返回代表本次 worker 生命周期结束；在释放节点并重新排队前停止句柄。
            if (!terminalResult) {
                if (!solverExecutionPort.stop(handle)) {
                    throw RemoteSolverException(
                        code = RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED,
                        message = "Failed to stop solver execution after quantum return"
                    )
                }
                handleStopped = true
            }

            if (checkpointRef != null) {
                val checkpointEnvelope = objectStoragePort
                    ?.get(checkpointRef)
                    ?.decodeToString()
                    ?.let(PortableCheckpointCodec::decodeCompatibleOrNull)
                checkpointPort.save(
                    CheckpointMetadata(
                        taskId = acceptedTask.taskId,
                        sliceId = sliceId,
                        ref = checkpointRef,
                        createdAt = clock.now(),
                        schemaVersion = checkpointEnvelope?.schemaVersion ?: if (
                            acceptedTask.payload.modelData.modelType == NormalizedModelType.CP &&
                            checkpointRef.path.value.contains("/checkpoint/")
                        ) {
                            "2.0"
                        } else {
                            "1.0"
                        },
                        modelFingerprint = checkpointEnvelope?.modelFingerprint
                            ?: acceptedTask.payload.extension["modelFingerprint"],
                        configurationFingerprint = checkpointEnvelope?.configurationFingerprint
                            ?: acceptedTask.payload.extension["configurationFingerprint"],
                        solverFingerprint = checkpointEnvelope?.solverFingerprint
                            ?: acceptedTask.payload.extension["solverFingerprint"],
                        integritySha256 = checkpointEnvelope?.integritySha256
                            ?: acceptedTask.payload.extension["checkpointIntegritySha256"],
                        provenance = checkpointEnvelope?.provenance
                            ?: sliceResult.provenance,
                        cancellationChain = checkpointEnvelope?.cancellationChain
                            ?: sliceResult.cancellationChain
                    )
                )
            }

            val sliceOutcome = resolveSliceOutcome(
                result = sliceResult,
                checkpointRef = checkpointRef,
                timeoutReason = sliceTimeoutReason
            )
            val schedulingDecision = schedulingDecision(
                task = acceptedTask,
                slice = initialSlice,
                node = selectedNode,
                quantumMs = quantumMs,
                audit = sliceAudit,
                checkpointRef = checkpointRef,
                incumbentRef = sliceResult.incumbentRef,
                outcome = sliceOutcome,
                reason = sliceResult.message ?: sliceAudit.selectionReason
            )
            sliceResult = sliceResult.copy(
                scheduling = mergeSchedulingDecision(sliceResult.scheduling, schedulingDecision),
                outcome = sliceOutcome
            )

            val costRecord = computeCost(acceptedTask, sliceId.value, selectedNode, sliceResult.elapsedMs)
            pendingBudgetReservation?.let { reservation ->
                if (!reconcileBudgetReservation(reservation, costRecord.totalCost)) {
                    throw RemoteSolverException(
                        code = RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED,
                        message = "Budget reconciliation failed for slice ${sliceId.value}"
                    )
                }
                pendingBudgetReservation = null
                budgetReservationBySliceId.remove(sliceId.value, reservation)
            }
            costLedgerPort.append(costRecord)
            metricsPort.timing("slice.runtime.ms", sliceResult.elapsedMs)
            metricsPort.gauge("slice.cost", costRecord.totalCost, tags = mapOf("nodeId" to selectedNode.nodeId.value))
            publishEvent(
                EventTopics.COST_AND_CAPABILITY_UPDATE,
                key = acceptedTask.taskId,
                payload = costRecord.summaryPayload(),
                tenantId = acceptedTask.tenantId,
                idempotencyKey = "slice:$sliceId:cost"
            )
            updateNodePerformanceScore(
                taskId = acceptedTask.taskId.value,
                nodeId = selectedNode.nodeId.value,
                quantumMs = currentSlice.quantum.inWholeMilliseconds,
                sliceResult = performanceResult
            )
            val consumedCostAfterSlice = Flt64(currentTask.consumedCost.toDouble() + costRecord.totalCost)
            val latestSnapshotAfterSlice = checkpointRef ?: currentTask.effectiveCheckpointRef()

            if (sliceTimeoutReason != null) {
                currentSlice = currentSlice.copy(
                    status = SliceStatus.FAILED,
                    checkpointRef = checkpointRef,
                    finishedAt = clock.now(),
                    error = sliceTimeoutReason
                )
                taskStatePort.updateSlice(currentSlice)
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-end",
                    taskStatus = TaskStatus.FAILED,
                    tenantId = acceptedTask.tenantId,
                    reason = sliceTimeoutReason,
                    audit = sliceAudit,
                    idempotencySuffix = "end-failed"
                )
                metricsPort.increment("task.failed", tags = mapOf("reason" to "slice_timeout"))
                return markFailed(
                    currentTask.copy(
                        latestSnapshotRef = latestSnapshotAfterSlice,
                        consumedCost = consumedCostAfterSlice,
                        updatedAt = clock.now()
                    ),
                    sliceTimeoutReason,
                    RemoteSolverErrorCode.TASK_FAILED_SLICE_TIMEOUT
                )
            }

            val timeoutAfterSliceReason = hardTimeoutReason(
                currentTask.copy(
                    latestSnapshotRef = latestSnapshotAfterSlice,
                    consumedCost = consumedCostAfterSlice
                )
            )
            if (timeoutAfterSliceReason != null) {
                currentSlice = currentSlice.copy(
                    status = SliceStatus.FAILED,
                    checkpointRef = checkpointRef,
                    finishedAt = clock.now(),
                    error = timeoutAfterSliceReason
                )
                taskStatePort.updateSlice(currentSlice)
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-end",
                    taskStatus = TaskStatus.FAILED,
                    tenantId = acceptedTask.tenantId,
                    reason = timeoutAfterSliceReason,
                    audit = sliceAudit,
                    idempotencySuffix = "end-failed"
                )
                metricsPort.increment("task.failed", tags = mapOf("reason" to "hard_timeout"))
                return markFailed(
                    currentTask.copy(
                        latestSnapshotRef = latestSnapshotAfterSlice,
                        consumedCost = consumedCostAfterSlice,
                        updatedAt = clock.now()
                    ),
                    timeoutAfterSliceReason,
                    RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT
                )
            }

            val terminal = sliceResult.completed || isTerminalTermination(sliceResult.terminationReason)
            val terminalStatus = when (sliceResult.terminationReason) {
                RemoteTerminationReason.CANCELLED -> TaskStatus.STOPPED
                RemoteTerminationReason.BACKEND_FAILURE,
                RemoteTerminationReason.NUMERICAL_FAILURE,
                RemoteTerminationReason.INTERRUPTED -> TaskStatus.FAILED
                else -> TaskStatus.COMPLETED
            }
            val updatedTask = if (terminal) {
                val fetchedFinalResult = solverExecutionPort.fetchFinalResult(handle)
                fetchedFinalResult?.let { result ->
                    RemoteResultValidator.validateSolveResult(
                        result = result,
                        expectedTaskId = acceptedTask.taskId,
                        expectedAttemptId = sliceId
                    )?.let { message ->
                        throw RemoteSolverException(
                            code = RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED,
                            message = "远程最终结果协议校验失败：$message / Remote final result protocol validation failed: $message"
                        )
                    }
                }
                val finalOutcome = when (terminalStatus) {
                    TaskStatus.COMPLETED -> SliceOutcome.COMPLETED
                    TaskStatus.STOPPED -> SliceOutcome.CANCELLED
                    else -> SliceOutcome.FAILED
                }
                val finalDecision = schedulingDecision(
                    task = acceptedTask,
                    slice = initialSlice,
                    node = selectedNode,
                    quantumMs = quantumMs,
                    audit = sliceAudit,
                    checkpointRef = checkpointRef,
                    incumbentRef = sliceResult.incumbentRef,
                    outcome = finalOutcome,
                    reason = sliceResult.message ?: sliceAudit.selectionReason
                )
                val finalResult = fetchedFinalResult?.copy(
                    incumbentRef = fetchedFinalResult.incumbentRef
                        ?: sliceResult.incumbentRef
                        ?: finalDecision.incumbentRef,
                    modelFingerprint = fetchedFinalResult.modelFingerprint
                        ?: sliceResult.modelFingerprint
                        ?: finalDecision.modelFingerprint,
                    scheduling = mergeSchedulingDecision(fetchedFinalResult.scheduling, finalDecision),
                    outcome = finalOutcome
                ) ?: SolveResult(
                    feasible = sliceResult.feasible,
                    optimal = sliceResult.solutionPresence == RemoteSolutionPresence.OPTIMAL &&
                        sliceResult.proofStatus == RemoteProofStatus.VERIFIED,
                    objectiveValue = sliceResult.objectiveValue,
                    objectiveValueInt64 = sliceResult.objectiveValueInt64,
                    gap = sliceResult.gap,
                    elapsed = sliceResult.elapsed,
                    checkpointRef = checkpointRef,
                    problemStatus = sliceResult.problemStatus,
                    terminationReason = sliceResult.terminationReason,
                    solutionPresence = sliceResult.solutionPresence,
                    proofStatus = sliceResult.proofStatus,
                    schemaVersion = sliceResult.schemaVersion,
                    resultRef = sliceResult.resultRef,
                    provenance = sliceResult.provenance,
                    fingerprints = sliceResult.fingerprints,
                    fingerprintSchemas = sliceResult.fingerprintSchemas,
                    statistics = sliceResult.statistics,
                    diagnostics = sliceResult.diagnostics,
                    runId = sliceResult.runId,
                    attemptId = sliceResult.attemptId,
                    artifactDigest = sliceResult.artifactDigest,
                    incumbentRef = sliceResult.incumbentRef ?: finalDecision.incumbentRef,
                    modelFingerprint = sliceResult.modelFingerprint ?: finalDecision.modelFingerprint,
                    scheduling = finalDecision,
                    outcome = finalOutcome,
                    cancellationChain = sliceResult.cancellationChain
                )
                currentSlice = currentSlice.copy(
                    status = if (terminalStatus == TaskStatus.COMPLETED) {
                        SliceStatus.COMPLETED
                    } else {
                        SliceStatus.FAILED
                    },
                    checkpointRef = checkpointRef,
                    finishedAt = clock.now()
                )
                currentTask.copy(
                    status = terminalStatus,
                    latestResult = finalResult,
                    latestSnapshotRef = checkpointRef ?: currentTask.effectiveCheckpointRef(),
                    consumedCost = consumedCostAfterSlice,
                    updatedAt = clock.now()
                )
            } else {
                val incumbent = if (sliceResult.feasible &&
                    sliceResult.solutionPresence != RemoteSolutionPresence.NONE
                ) {
                    val incumbentDecision = schedulingDecision(
                        task = acceptedTask,
                        slice = initialSlice,
                        node = selectedNode,
                        quantumMs = quantumMs,
                        audit = sliceAudit,
                        checkpointRef = checkpointRef,
                        incumbentRef = sliceResult.incumbentRef,
                        outcome = sliceOutcome,
                        reason = sliceResult.message ?: sliceAudit.selectionReason
                    )
                    SolveResult(
                        feasible = true,
                        optimal = false,
                        objectiveValue = sliceResult.objectiveValue,
                        objectiveValueInt64 = sliceResult.objectiveValueInt64,
                        gap = sliceResult.gap,
                        elapsed = sliceResult.elapsed,
                        checkpointRef = checkpointRef,
                        resultRef = sliceResult.resultRef,
                        message = sliceResult.message,
                        schemaVersion = sliceResult.schemaVersion,
                        problemStatus = RemoteProblemStatus.FEASIBLE,
                        terminationReason = sliceResult.terminationReason,
                        solutionPresence = RemoteSolutionPresence.INCUMBENT,
                        proofStatus = RemoteProofStatus.NONE,
                        provenance = sliceResult.provenance,
                        fingerprints = sliceResult.fingerprints,
                        fingerprintSchemas = sliceResult.fingerprintSchemas,
                        statistics = sliceResult.statistics,
                        diagnostics = sliceResult.diagnostics,
                        runId = sliceResult.runId,
                        attemptId = sliceResult.attemptId,
                        artifactDigest = sliceResult.artifactDigest,
                        incumbentRef = sliceResult.incumbentRef,
                        modelFingerprint = sliceResult.modelFingerprint ?: incumbentDecision.modelFingerprint,
                        scheduling = incumbentDecision,
                        outcome = sliceOutcome,
                        cancellationChain = sliceResult.cancellationChain
                    )
                } else {
                    currentTask.latestResult
                }
                currentSlice = currentSlice.copy(
                    status = SliceStatus.SUSPENDED,
                    checkpointRef = checkpointRef,
                    finishedAt = clock.now()
                )
                currentTask.copy(
                    status = TaskStatus.SUSPENDED,
                    latestResult = incumbent,
                    latestSnapshotRef = checkpointRef ?: currentTask.effectiveCheckpointRef(),
                    consumedCost = consumedCostAfterSlice,
                    updatedAt = clock.now()
                )
            }
            taskStatePort.updateSlice(currentSlice)
            taskStatePort.upsertTask(updatedTask)
            if (updatedTask.status in setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)) {
                clearLearningState(updatedTask.taskId.value)
            }
            if (updatedTask.status == TaskStatus.COMPLETED) {
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-end",
                    taskStatus = updatedTask.status,
                    tenantId = acceptedTask.tenantId,
                    audit = sliceAudit,
                    idempotencySuffix = "end-completed"
                )
            } else if (updatedTask.status in setOf(TaskStatus.FAILED, TaskStatus.STOPPED)) {
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-end",
                    taskStatus = updatedTask.status,
                    tenantId = acceptedTask.tenantId,
                    reason = sliceResult.message,
                    audit = sliceAudit,
                    idempotencySuffix = "end-terminal"
                )
            } else {
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-suspend",
                    taskStatus = updatedTask.status,
                    tenantId = acceptedTask.tenantId,
                    audit = sliceAudit,
                    idempotencySuffix = "suspend"
                )
            }
            publishEvent(
                topic = if (updatedTask.status in setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)) {
                    EventTopics.TASK_RESULT
                } else EventTopics.SLICE_LIFECYCLE,
                key = updatedTask.taskId,
                payload = updatedTask.summaryPayload(),
                tenantId = acceptedTask.tenantId,
                idempotencyKey = if (updatedTask.status in setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)) {
                    "task:${updatedTask.taskId}:terminal"
                } else {
                    "slice:${currentSlice.sliceId}:suspended"
                }
            )
            updatedTask
        } catch (e: CancellationException) {
            throw e
        } catch (_: StaleExecutionException) {
            if (leaseLost.get()) {
                recoverLostDispatch(
                    taskId = acceptedTask.taskId,
                    sliceId = sliceId,
                    nodeId = selectedNode.nodeId
                )
            }
            taskStatePort.getTask(acceptedTask.taskId) ?: currentTask
        } catch (e: Exception) {
            currentSlice = currentSlice.copy(
                status = SliceStatus.FAILED,
                finishedAt = clock.now(),
                error = e.message
            )
            taskStatePort.updateSlice(currentSlice)
            publishSliceLifecycle(
                slice = currentSlice,
                action = "slice-end",
                taskStatus = TaskStatus.FAILED,
                tenantId = acceptedTask.tenantId,
                reason = e.message ?: "Unknown failure",
                audit = sliceAudit,
                idempotencySuffix = "end-failed"
            )
            val reasonCode = if (e is RemoteSolverException) {
                e.code
            } else {
                RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED
            }
            val failedTask = markFailed(
                currentTask,
                e.message ?: "Unknown failure",
                reasonCode
            )
            failedTask.latestResult?.let { result ->
                failedTask.copy(
                    latestResult = result.copy(
                        scheduling = mergeSchedulingDecision(
                            result.scheduling,
                            schedulingDecision(
                                task = acceptedTask,
                                slice = initialSlice,
                                node = selectedNode,
                                quantumMs = quantumMs,
                                audit = sliceAudit,
                                checkpointRef = currentSlice.checkpointRef,
                                incumbentRef = result.incumbentRef,
                                outcome = SliceOutcome.FAILED,
                                reason = e.message ?: "Unknown failure"
                            )
                        ),
                        outcome = SliceOutcome.FAILED
                    )
                )
                    .also { persisted ->
                        taskStatePort.upsertTask(persisted)
                    }
            } ?: failedTask
        } finally {
            pendingBudgetReservation?.let { reservation ->
                try {
                    releaseBudgetReservation(reservation)
                } catch (_: Exception) {
                }
                budgetReservationBySliceId.remove(sliceId.value, reservation)
            }
            activeHandle?.let { handle ->
                if (!handleStopped) {
                    try {
                        solverExecutionPort.stop(handle)
                    } catch (_: Exception) {
                    }
                }
            }
            activeHandle?.let { handle ->
                runningHandleByTaskId.remove(acceptedTask.taskId.value, handle)
            }
            if (runningSliceByTaskId.remove(acceptedTask.taskId.value, sliceId.value)) {
                nodeStatePort.releaseUnit(selectedNode.nodeId)
            }
        }
    }

    /**
     * 判断切片终止原因是否代表求解已到达不可继续切片的终态。
     * Determines whether a termination reason represents a non-resumable terminal state.
     *
     * @param reason 切片终止原因 / Slice termination reason
     * @return 是否为非可恢复终态 / Whether the reason is non-resumable and terminal
     */
    private fun isTerminalTermination(reason: RemoteTerminationReason): Boolean {
        return reason in setOf(
            RemoteTerminationReason.SOLUTION_LIMIT,
            RemoteTerminationReason.OBJECTIVE_LIMIT,
            RemoteTerminationReason.CANCELLED,
            RemoteTerminationReason.INTERRUPTED,
            RemoteTerminationReason.NUMERICAL_FAILURE,
            RemoteTerminationReason.BACKEND_FAILURE
        )
    }

    private suspend fun acceptForDispatch(task: TaskState): TaskState? {
        if (task.status == TaskStatus.ACCEPTED) {
            return task
        }
        if (task.status !in setOf(TaskStatus.QUEUED, TaskStatus.SUSPENDED)) {
            return null
        }
        val accepted = taskStatePort.compareAndSet(
            taskId = task.taskId,
            from = setOf(TaskStatus.QUEUED, TaskStatus.SUSPENDED),
            to = TaskStatus.ACCEPTED,
            updatedAt = clock.now()
        )
        if (!accepted) {
            return null
        }
        val acceptedTask = taskStatePort.getTask(task.taskId) ?: return null
        publishEvent(
            topic = EventTopics.SOLVING_CONTROL,
            key = acceptedTask.taskId,
            payload = SolvingControlPayload(
                taskId = acceptedTask.taskId.value,
                action = "accept",
                status = "accepted",
                dispatcherId = dispatcherId
            ).toByteArray(),
            tenantId = acceptedTask.tenantId,
            idempotencyKey = "task:${acceptedTask.taskId}:accept"
        )
        return acceptedTask
    }

    private suspend fun requeueWaitingForBudget(task: TaskState): TaskState? {
        if (task.status != TaskStatus.WAITING_FOR_BUDGET) {
            return task
        }
        val queuedAt = clock.now()
        val requeued = taskStatePort.compareAndSet(
            taskId = task.taskId,
            from = setOf(TaskStatus.WAITING_FOR_BUDGET),
            to = TaskStatus.QUEUED,
            updatedAt = queuedAt
        )
        if (!requeued) {
            return null
        }
        val queuedTask = taskStatePort.getTask(task.taskId) ?: return null
        publishEvent(
            topic = EventTopics.SOLVING_CONTROL,
            key = queuedTask.taskId,
            payload = SolvingControlPayload(
                taskId = queuedTask.taskId.value,
                action = "budget_requeue",
                status = TaskStatus.QUEUED.name,
                reason = "Budget available, task returned to queue"
            ).toByteArray(),
            tenantId = queuedTask.tenantId,
            idempotencyKey = "task:${queuedTask.taskId}:budget_requeue"
        )
        return queuedTask
    }

    private suspend fun checkBudget(task: TaskState): Boolean {
        val limit = task.effectiveBudgetLimit() ?: return true
        if (task.consumedCost.toDouble() >= limit.toDouble()) {
            return false
        }
        val snapshot = budgetPort.snapshot(task.effectiveBudgetScope().value) ?: return true
        return snapshot.remaining > 0
    }

    private suspend fun markWaitingForBudget(task: TaskState, reason: String): TaskState {
        if (task.status == TaskStatus.WAITING_FOR_BUDGET) {
            return task
        }
        val waitingTask = task.copy(
            status = TaskStatus.WAITING_FOR_BUDGET,
            assignedNodeId = null,
            updatedAt = clock.now()
        )
        taskStatePort.upsertTask(waitingTask)
        publishEvent(
            topic = EventTopics.SOLVING_CONTROL,
            key = waitingTask.taskId,
            payload = SolvingControlPayload(
                taskId = waitingTask.taskId.value,
                action = "budget_wait",
                status = "waiting",
                reason = reason
            ).toByteArray(),
            tenantId = waitingTask.tenantId,
            idempotencyKey = "task:${waitingTask.taskId}:budget_wait"
        )
        metricsPort.increment("task.budget_wait")
        return waitingTask
    }

    private suspend fun reserveBudgetForDispatch(
        task: TaskState,
        dispatchId: DispatchId,
        sliceId: SliceId,
        node: NodeState,
        quantumMs: Long
    ): BudgetReservation? {
        val amount = estimateSliceCost(node, quantumMs)
        if (!amount.isFinite() || amount < 0.0) {
            return null
        }
        val scope = task.effectiveBudgetScope().value
        // Keep this identity stable for the complete dispatch lifecycle.  The
        // budget adapter uses it to make retries and settlement idempotent.
        val reservation = BudgetReservation(
            reservationId = "dispatch:${dispatchId.value}:slice:${sliceId.value}",
            scope = scope,
            amount = amount
        )
        if (!budgetPort.reserve(reservation)) {
            return null
        }
        budgetReservationBySliceId[sliceId.value] = reservation
        return reservation
    }

    /**
     * Releases a reservation without changing the net consumed amount.
     * 在不改变净消费金额的前提下释放预算预留。
     */
    private suspend fun releaseBudgetReservation(reservation: BudgetReservation): Boolean {
        // Settling at zero atomically clears the reservation without charging
        // the budget.  This is also idempotent for a retrying cleanup path.
        return budgetPort.settle(reservation, 0.0)
    }

    private suspend fun reconcileBudgetReservation(
        reservation: BudgetReservation,
        actualCost: Double
    ): Boolean {
        if (!actualCost.isFinite() || actualCost < 0.0) {
            return false
        }
        // The port performs the below/above-estimate cases in one atomic
        // transition and records the reservation id for idempotent retries.
        return budgetPort.settle(reservation, actualCost)
    }

    private sealed class BudgetDegradationResult {
        data class CanProceedWithNode(val node: NodeState) : BudgetDegradationResult()
        object WaitForBudget : BudgetDegradationResult()
        object FailAfterDegradation : BudgetDegradationResult()
    }

    private suspend fun tryBudgetDegradation(
        task: TaskState,
        compatibleNodes: List<NodeState>
    ): BudgetDegradationResult {
        val snapshot = budgetPort.snapshot(task.effectiveBudgetScope().value)
        val budgetLimit = task.effectiveBudgetLimit()?.toDouble() ?: Double.MAX_VALUE
        val remaining = snapshot?.remaining ?: (budgetLimit - task.consumedCost.toDouble())

        // 1. Try cheapest node
        val sortedNodes = compatibleNodes.sortedBy { it.profile.pricePerSecond }
        for (node in sortedNodes) {
            val quantumMs = computeQuantumMs(task, node)
            val cheapCost = estimateSliceCost(node, quantumMs)
            if (remaining >= cheapCost) {
                return BudgetDegradationResult.CanProceedWithNode(node)
            }
        }

        // 2. Check if budget refresh could help
        // If budgetLimit is too low for any node, even refresh won't help -> fail
        if (sortedNodes.isNotEmpty()) {
            val cheapestNode = sortedNodes.first()
            val cheapestQuantumMs = computeQuantumMs(task, cheapestNode)
            val cheapestCost = estimateSliceCost(cheapestNode, cheapestQuantumMs)
            // If the budget limit itself is below the cheapest node cost, fail immediately
            // (budget refresh won't increase budgetLimit)
            if (budgetLimit < cheapestCost) {
                return BudgetDegradationResult.FailAfterDegradation
            }
            // Otherwise, wait for budget refresh
            return BudgetDegradationResult.WaitForBudget
        }

        // 3. No compatible nodes
        return BudgetDegradationResult.FailAfterDegradation
    }

    private fun rankCandidates(tasks: List<TaskState>): List<TaskState> {
        val now = clock.nowEpochMs()
        val complexAges = tasks
            .asSequence()
            .filter { it.effectiveComplexity() == TaskComplexity.COMPLEX }
            .map { max(0L, now - it.createdAt.toEpochMilliseconds()).toDouble() }
            .toList()
        val maxAge = complexAges.maxOrNull() ?: 1.0
        val starvationAgeMs = runtimeSchedulerConfigRef.get().schedulerStarvationAgeMs.coerceAtLeast(1L)

        fun waitingAgeMs(task: TaskState): Long = max(
            0L,
            now - (lastDispatchAtByTaskId[task.taskId.value]
                ?: task.createdAt.toEpochMilliseconds())
        )

        return tasks.sortedWith(Comparator { left, right ->
            val leftWaitingAge = waitingAgeMs(left)
            val rightWaitingAge = waitingAgeMs(right)
            val leftStarved = leftWaitingAge >= starvationAgeMs
            val rightStarved = rightWaitingAge >= starvationAgeMs

            // A starved task first enters the protected set. Once both tasks
            // are protected, the oldest waiting task wins across priority
            // cohorts; otherwise a high-priority stream can keep both tasks
            // starved while still monopolizing every dispatch.
            val starvationComparison = when {
                leftStarved != rightStarved -> if (leftStarved) -1 else 1
                leftStarved && rightStarved &&
                    left.effectiveDeadline() == null && right.effectiveDeadline() == null ->
                    rightWaitingAge.compareTo(leftWaitingAge)
                else -> 0
            }
            if (starvationComparison != 0) {
                starvationComparison
            } else {
                val leftUrgency = if (left.effectiveDeadline() != null) estimateUrgency(left, now) else 0.0
                val rightUrgency = if (right.effectiveDeadline() != null) estimateUrgency(right, now) else 0.0
                val urgencyComparison = rightUrgency.compareTo(leftUrgency)
                if (urgencyComparison != 0) {
                    urgencyComparison
                } else {
                    // Preserve hard priority for tasks outside the starvation
                    // guard, and rotate equal-priority work within its cohort.
                    val priorityComparison = right.effectivePriority().compareTo(left.effectivePriority())
                    if (priorityComparison != 0) {
                        priorityComparison
                    } else {
                        val roundRobinComparison = taskRoundRobinOrder(left, tasks)
                            .compareTo(taskRoundRobinOrder(right, tasks))
                        if (roundRobinComparison != 0) {
                            roundRobinComparison
                        } else {
                            val leftScore = if (left.effectiveComplexity() == TaskComplexity.COMPLEX) {
                                complexRoundRobinPriority(left, now, maxAge)
                            } else {
                                simplePriorityScore(left)
                            }
                            val rightScore = if (right.effectiveComplexity() == TaskComplexity.COMPLEX) {
                                complexRoundRobinPriority(right, now, maxAge)
                            } else {
                                simplePriorityScore(right)
                            }
                            val scoreComparison = rightScore.compareTo(leftScore)
                            if (scoreComparison != 0) {
                                scoreComparison
                            } else {
                                val createdComparison = left.createdAt.compareTo(right.createdAt)
                                if (createdComparison != 0) {
                                    createdComparison
                                } else {
                                    left.taskId.value.compareTo(right.taskId.value)
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    private fun fairnessCohortKey(task: TaskState): String? {
        // Deadline work is ordered by urgency and must not be rotated ahead of
        // a task with a materially tighter deadline.
        if (task.effectiveDeadline() != null) {
            return null
        }
        return listOf(
            task.effectiveComplexity().name,
            task.effectiveTimeSensitivity().name,
            task.effectivePriority().toString()
        ).joinToString("|")
    }

    private fun taskRoundRobinOrder(task: TaskState, tasks: List<TaskState>): Int {
        val key = fairnessCohortKey(task) ?: return 0
        val cohort = tasks
            .filter { fairnessCohortKey(it) == key }
            .sortedWith(compareBy<TaskState> { it.createdAt }.thenBy { it.taskId.value })
        if (cohort.size <= 1) {
            return 0
        }
        val taskIndex = cohort.indexOfFirst { it.taskId == task.taskId }
        if (taskIndex < 0) {
            return 0
        }
        val cursorIndex = taskRoundRobinCursorByClass[key]
            ?.let { cursorId -> cohort.indexOfFirst { it.taskId.value == cursorId } }
            ?.takeIf { it >= 0 }
            ?: -1
        val start = (cursorIndex + 1 + cohort.size) % cohort.size
        return (taskIndex - start + cohort.size) % cohort.size
    }

    private fun rememberTaskDispatch(task: TaskState) {
        lastDispatchAtByTaskId[task.taskId.value] = clock.nowEpochMs()
        fairnessCohortKey(task)?.let { key ->
            taskRoundRobinCursorByClass[key] = task.taskId.value
        }
    }

    private fun simplePriorityScore(task: TaskState): Double {
        val realtimeBonus = if (task.effectiveTimeSensitivity() == TimeSensitivity.REALTIME) 1.0 else 0.0
        return task.effectivePriority().toDouble() * 10.0 + realtimeBonus
    }

    private fun complexRoundRobinPriority(task: TaskState, now: Long, maxAge: Double): Double {
        val runtime = runtimeSchedulerConfigRef.get()
        val urgency = estimateUrgency(task, now)
        val waitingAge = max(0L, now - task.createdAt.toEpochMilliseconds()).toDouble() / max(1.0, maxAge)
        val progressNeed = estimateProgressNeed(task)
        val costSensitivity = estimateCostSensitivity(task)
        val priorityBoost = task.effectivePriority().toDouble() / 100.0
        return runtime.complexUrgencyWeight * urgency +
            runtime.complexWaitingAgeWeight * waitingAge +
            runtime.complexProgressNeedWeight * progressNeed -
            runtime.complexCostSensitivityWeight * costSensitivity +
            priorityBoost
    }

    private fun estimateUrgency(task: TaskState, now: Long): Double {
        val deadlineInstant = task.effectiveDeadline() ?: return 0.0
        val deadlineEpochMs = deadlineInstant.toEpochMilliseconds()
        val createdAtEpochMs = task.createdAt.toEpochMilliseconds()
        if (deadlineEpochMs <= now) {
            return 1.0
        }
        val window = max(1L, deadlineEpochMs - createdAtEpochMs).toDouble()
        val consumed = max(0L, now - createdAtEpochMs).toDouble()
        return (consumed / window).coerceIn(0.0, 1.0)
    }

    private fun estimateProgressNeed(task: TaskState): Double {
        val gapBased = task.latestResult?.gap?.toDouble()?.coerceIn(0.0, 1.0)
        if (gapBased != null) {
            return gapBased
        }
        return if (task.effectiveCheckpointRef() == null) {
            1.0
        } else {
            0.5
        }
    }

    private fun estimateCostSensitivity(task: TaskState): Double {
        val budgetLimit = task.effectiveBudgetLimit() ?: return 0.0
        val limit = budgetLimit.toDouble()
        if (limit <= 0.0) {
            return 1.0
        }
        return (task.consumedCost.toDouble() / limit).coerceIn(0.0, 1.0)
    }

    private fun resolveHardTimeoutEpochMs(task: TaskState): Long? =
        (task.payload.config?.timeLimitMs ?: task.payload.taskMeta.timeLimitMs)
            ?.takeIf { it > 0L }
            ?.let { task.createdAt.plus(it.toDuration(DurationUnit.MILLISECONDS)).toEpochMilliseconds() }

    private fun hardTimeoutReason(task: TaskState, nowEpochMs: Long = clock.nowEpochMs()): String? {
        val timeoutEpochMs = resolveHardTimeoutEpochMs(task)
        if (timeoutEpochMs != null && nowEpochMs > timeoutEpochMs) {
            return "Hard timeout exceeded: now=$nowEpochMs, timeoutAt=$timeoutEpochMs"
        }
        val deadlineEpochMs = task.effectiveDeadline()?.toEpochMilliseconds()
        if (deadlineEpochMs != null && nowEpochMs >= deadlineEpochMs) {
            return "Deadline exceeded: now=$nowEpochMs, deadlineAt=$deadlineEpochMs"
        }
        return null
    }

    private fun sliceTimeoutReason(sliceResult: SliceResult, quantumMs: Long): String? {
        val allowedMs = max(0L, quantumMs) + max(0L, config.sliceTimeoutGraceMs)
        if (sliceResult.elapsedMs <= allowedMs) {
            return null
        }
        return "Slice timeout exceeded: elapsedMs=${sliceResult.elapsedMs}, allowedMs=$allowedMs"
    }

    private fun validateSliceResult(
        result: SliceResult,
        expectedTaskId: TaskId,
        expectedSliceId: SliceId
    ) {
        RemoteResultValidator.validateSliceResult(
            result = result,
            expectedTaskId = expectedTaskId,
            expectedSliceId = expectedSliceId
        )?.let { message ->
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED,
                message = "远程切片结果协议校验失败：$message / Remote slice result protocol validation failed: $message"
            )
        }
    }

    /**
     * Checks both the process-local generation fence and persisted ownership.
     * The persisted check is what protects a restarted dispatcher or a node
     * recovery from accepting a late result from an old slice.
     */
    private suspend fun isExecutionOwner(
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId
    ): Boolean {
        if (runningSliceByTaskId[taskId.value] != sliceId.value) {
            return false
        }
        val task = taskStatePort.getTask(taskId) ?: return false
        if (task.assignedNodeId != nodeId ||
            task.status !in setOf(TaskStatus.DISPATCHING, TaskStatus.RUNNING)
        ) {
            return false
        }
        val slice = taskStatePort.getSlices(taskId).firstOrNull { it.sliceId == sliceId }
            ?: return false
        return slice.nodeId == nodeId &&
            slice.status in setOf(SliceStatus.PLANNED, SliceStatus.RUNNING, SliceStatus.CHECKPOINTING)
    }

    /**
     * Return a lease-fenced dispatch to a retryable state without touching a
     * newer generation.  The generation check is deliberately performed before
     * every persisted repair so a late worker cannot overwrite a replacement.
     *
     * 租约丢失后将当前分发恢复为可重试状态，并且在每次持久化修复前检查代次，
     * 防止迟到 worker 覆盖新的分发代次。
     */
    private suspend fun recoverLostDispatch(
        taskId: TaskId,
        sliceId: SliceId?,
        nodeId: NodeId?
    ) {
        val generation = sliceId?.value ?: return
        if (runningSliceByTaskId[taskId.value] != generation) {
            return
        }
        val task = taskStatePort.getTask(taskId) ?: return
        if (nodeId == null || task.assignedNodeId != nodeId ||
            task.status !in setOf(TaskStatus.DISPATCHING, TaskStatus.RUNNING)
        ) {
            return
        }
        val activeSlice = taskStatePort.getSlices(taskId)
            .firstOrNull { it.sliceId.value == generation }
        if (activeSlice != null && activeSlice.status in setOf(
                SliceStatus.PLANNED,
                SliceStatus.RUNNING,
                SliceStatus.CHECKPOINTING
            )) {
            val failedSlice = activeSlice.copy(
                status = SliceStatus.FAILED,
                finishedAt = clock.now(),
                error = "Dispatch lease lost"
            )
            if (runningSliceByTaskId[taskId.value] != generation) {
                return
            }
            taskStatePort.updateSlice(failedSlice)
            publishSliceLifecycle(
                slice = failedSlice,
                action = "slice-end",
                taskStatus = TaskStatus.QUEUED,
                tenantId = task.tenantId,
                reason = "Dispatch lease lost",
                idempotencySuffix = "lease-lost"
            )
        }
        val retryStatus = if (task.effectiveCheckpointRef() != null) {
            TaskStatus.SUSPENDED
        } else {
            TaskStatus.QUEUED
        }
        if (runningSliceByTaskId[taskId.value] != generation) {
            return
        }
        if (taskStatePort.compareAndSet(
                taskId = taskId,
                from = setOf(TaskStatus.DISPATCHING, TaskStatus.RUNNING),
                to = retryStatus,
                updatedAt = clock.now()
            )
        ) {
            val current = taskStatePort.getTask(taskId)
            if (current != null && current.assignedNodeId == nodeId &&
                current.status == retryStatus && runningSliceByTaskId[taskId.value] == generation
            ) {
                taskStatePort.upsertTask(current.copy(assignedNodeId = null, updatedAt = clock.now()))
            }
        }
    }

    private fun taskProgressSignal(task: TaskState): ProgressSignal? {
        val taskId = task.taskId.value
        val urgency = if (task.effectiveDeadline() != null) {
            estimateUrgency(task, clock.nowEpochMs())
        } else {
            lastDeadlineUrgencyByTaskId.remove(taskId)
            0.0
        }
        val previousUrgency = if (task.effectiveDeadline() != null) {
            lastDeadlineUrgencyByTaskId.put(taskId, urgency)
        } else {
            null
        }
        val deadlineRiskIncreasing = previousUrgency != null &&
            urgency > previousUrgency + 1e-9
        val observed = progressSignalByTaskId[taskId]
        if (observed == null && !deadlineRiskIncreasing) {
            return null
        }
        return (observed ?: ProgressSignal()).copy(
            currentGap = task.latestResult?.gap?.toDouble()?.coerceIn(0.0, 1.0)
                ?: observed?.currentGap,
            deadlineRiskIncreasing = deadlineRiskIncreasing
        )
    }

    private fun saturatingAdd(left: Long, right: Long): Long {
        if (right <= 0L) {
            return left
        }
        return if (Long.MAX_VALUE - left < right) Long.MAX_VALUE else left + right
    }

    private fun computeQuantumMs(task: TaskState, node: NodeState): Long {
        val decision = computeQuantumDecision(task, node)
        return effectiveQuantumMs(task, node, decision)
    }

    private fun effectiveQuantumMs(
        task: TaskState,
        node: NodeState,
        decision: QuantumDecision
    ): Long {
        val runtime = runtimeSchedulerConfigRef.get()
        var effective = if (!requiresRunToCompletion(task)) {
            decision.quantumMs.coerceAtLeast(1L)
        } else {
            val estimatedRuntimeMs = task.effectiveRuntimeEstimateMs()
                ?: if (task.effectiveComplexity() == TaskComplexity.SIMPLE) {
                    runtime.simpleTaskQuantumMs
                } else {
                    (runtime.complexSolveEstimateMs.toDouble() /
                        max(0.1, node.profile.performanceScore.toDouble())).toLong()
                }
            val candidate = max(decision.quantumMs, estimatedRuntimeMs.coerceAtLeast(1L))
            val taskLimitMs = task.payload.config?.timeLimitMs ?: task.payload.taskMeta.timeLimitMs
            if (taskLimitMs != null && taskLimitMs > 0L) {
                min(candidate, taskLimitMs).coerceAtLeast(1L)
            } else {
                candidate
            }
        }
        val deadlineEpochMs = task.effectiveDeadline()?.toEpochMilliseconds()
        if (deadlineEpochMs != null) {
            val now = clock.nowEpochMs()
            val remaining = if (deadlineEpochMs <= now) {
                0L
            } else {
                deadlineEpochMs - now
            }
            if (remaining <= 0L) {
                return 0L
            }
            val twiceQuantum = if (effective >= Long.MAX_VALUE / 2L) {
                Long.MAX_VALUE
            } else {
                effective * 2L
            }
            if (remaining < twiceQuantum) {
                effective = min(
                    runtime.complexTaskQuantumMaxMs.coerceAtLeast(1L),
                    remaining
                ).coerceAtLeast(1L)
            }
        }
        return effective
    }

    private fun requiresRunToCompletion(task: TaskState): Boolean =
        task.effectiveComplexity() == TaskComplexity.SIMPLE ||
            task.payload.scheduling?.preemptionMode ==
            fuookami.ospf.framework.remote_solver.protocol.domain.PreemptionMode.NON_PREEMPTIBLE

    private fun isTerminalSliceResult(result: SliceResult): Boolean =
        result.completed || isTerminalTermination(result.terminationReason)

    private fun computeQuantumDecision(task: TaskState, node: NodeState): QuantumDecision {
        val runtime = runtimeSchedulerConfigRef.get()
        if (task.effectiveComplexity() != TaskComplexity.COMPLEX) {
            return QuantumDecision(
                quantumMs = runtime.simpleTaskQuantumMs.coerceAtLeast(1L),
                reason = "simple_fixed_quantum",
                complexity = task.effectiveComplexity(),
                performanceScore = node.profile.performanceScore.toDouble(),
                checkpointEstimateMs = 0L
            )
        }

        val performance = max(0.1, node.profile.performanceScore.toDouble())
        val solveEstimateMs = (task.effectiveRuntimeEstimateMs()?.toDouble()
            ?: runtime.complexSolveEstimateMs.toDouble()) / performance
        val configuredCheckpointEstimateMs = task.effectiveCheckpointEstimateMs()
            ?: runtime.complexCheckpointEstimateMs
        val checkpointEstimateMs = if (node.profile.supportsCheckpoint) {
            configuredCheckpointEstimateMs.toDouble()
        } else {
            0.0
        }
        // Checkpoint time is an amortizable per-slice cost: a more expensive
        // checkpoint warrants a longer quantum, rather than shrinking it.
        // 检查点成本可通过更长时间片摊销，因此成本越高，量子应越大。
        val baselineQuantum = runtime.complexQuantumAlpha * solveEstimateMs +
            runtime.complexQuantumBeta * checkpointEstimateMs
        val priceFactor = 1.0 + max(0.0, node.profile.pricePerSecond.toDouble()) * runtime.complexQuantumPricePenalty
        val adjustedQuantum = baselineQuantum / priceFactor
        val fallbackQuantum = runtime.complexTaskQuantumMs.toDouble()
        val candidate = if (adjustedQuantum.isFinite() && adjustedQuantum > 0.0) {
            adjustedQuantum
        } else {
            fallbackQuantum
        }
        val clamped = min(
            runtime.complexTaskQuantumMaxMs.toDouble(),
            max(runtime.complexTaskQuantumMinMs.toDouble(), candidate)
        )
        return QuantumDecision(
            quantumMs = clamped.toLong().coerceAtLeast(1L),
            reason = "complex_dynamic_clamped",
            complexity = task.effectiveComplexity(),
            performanceScore = performance,
            checkpointEstimateMs = configuredCheckpointEstimateMs
        )
    }

    private fun ensureHotReloadEnabled() {
        if (!config.schedulerHotReloadEnabled) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "scheduler.hot-reload.enabled=false"
            )
        }
    }

    private fun applySchedulerChangeSet(
        current: SchedulerRuntimeConfig,
        changeSet: Map<String, String>
    ): SchedulerRuntimeConfig {
        if (changeSet.isEmpty()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "changeSet must not be empty"
            )
        }
        var updated = current
        changeSet.forEach { (key, rawValue) ->
            updated = when (key) {
                "scheduler.simple-task-quantum-ms" ->
                    updated.copy(simpleTaskQuantumMs = parseLongConfigValue(key, rawValue, minValue = 1L))
                "scheduler.complex-task-quantum-ms" ->
                    updated.copy(complexTaskQuantumMs = parseLongConfigValue(key, rawValue, minValue = 1L))
                "scheduler.complex-task-quantum-min-ms" ->
                    updated.copy(complexTaskQuantumMinMs = parseLongConfigValue(key, rawValue, minValue = 1L))
                "scheduler.complex-task-quantum-max-ms" ->
                    updated.copy(complexTaskQuantumMaxMs = parseLongConfigValue(key, rawValue, minValue = 1L))
                "scheduler.complex-solve-estimate-ms" ->
                    updated.copy(complexSolveEstimateMs = parseLongConfigValue(key, rawValue, minValue = 1L))
                "scheduler.complex-checkpoint-estimate-ms" ->
                    updated.copy(complexCheckpointEstimateMs = parseLongConfigValue(key, rawValue, minValue = 0L))
                "scheduler.complex-quantum-alpha" ->
                    updated.copy(complexQuantumAlpha = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.complex-quantum-beta" ->
                    updated.copy(complexQuantumBeta = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.complex-quantum-price-penalty" ->
                    updated.copy(complexQuantumPricePenalty = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.complex-urgency-weight" ->
                    updated.copy(complexUrgencyWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.complex-waiting-age-weight" ->
                    updated.copy(complexWaitingAgeWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.complex-progress-need-weight" ->
                    updated.copy(complexProgressNeedWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.complex-cost-sensitivity-weight" ->
                    updated.copy(complexCostSensitivityWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.starvation-age-ms" ->
                    updated.copy(schedulerStarvationAgeMs = parseLongConfigValue(key, rawValue, minValue = 1L))
                "scheduler.migration-hysteresis-ratio" ->
                    updated.copy(
                        schedulerMigrationHysteresisRatio = parseDoubleConfigValueInRange(
                            key = key,
                            raw = rawValue,
                            minValue = 0.0,
                            maxValue = 1.0
                        )
                    )
                "scheduler.min-slices-before-migration" ->
                    updated.copy(
                        schedulerMinSlicesBeforeMigration = parseIntConfigValue(
                            key = key,
                            raw = rawValue,
                            minValue = 0
                        )
                    )
                "scheduler.migration-cost-weight" ->
                    updated.copy(schedulerMigrationCostWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.cost-weight" ->
                    updated.copy(schedulerCostWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.deadline-risk-weight" ->
                    updated.copy(schedulerDeadlineRiskWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.queue-delay-weight" ->
                    updated.copy(schedulerQueueDelayWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.round-robin-score-tolerance" ->
                    updated.copy(
                        schedulerRoundRobinScoreTolerance = parseDoubleConfigValueInRange(
                            key = key,
                            raw = rawValue,
                            minValue = 0.0,
                            maxValue = 1.0
                        )
                    )
                "scheduler.weighted-round-robin.enabled" ->
                    updated.copy(schedulerWeightedRoundRobinEnabled = parseBooleanConfigValue(key, rawValue))
                "scheduler.progress-weight" ->
                    updated.copy(schedulerProgressWeight = parseDoubleConfigValue(key, rawValue, minValue = 0.0))
                "scheduler.progress.fast-gap-threshold" ->
                    updated.copy(
                        schedulerProgressFastGapThreshold = parseDoubleConfigValueInRange(
                            key = key,
                            raw = rawValue,
                            minValue = 0.0,
                            maxValue = 1.0
                        )
                    )
                "scheduler.progress.cheap-gap-threshold" ->
                    updated.copy(
                        schedulerProgressCheapGapThreshold = parseDoubleConfigValueInRange(
                            key = key,
                            raw = rawValue,
                            minValue = 0.0,
                            maxValue = 1.0
                        )
                    )
                "scheduler.progress.min-improvement" ->
                    updated.copy(
                        schedulerProgressMinImprovement = parseDoubleConfigValueInRange(
                            key = key,
                            raw = rawValue,
                            minValue = 0.0,
                            maxValue = 1.0
                        )
                    )
                "scheduler.progress.no-improvement-slices" ->
                    updated.copy(
                        schedulerProgressNoImprovementSlices = parseIntConfigValue(
                            key = key,
                            raw = rawValue,
                            minValue = 1
                        )
                    )
                else ->
                    throw RemoteSolverException(
                        code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                        message = "Unsupported hot-reload key '$key'"
                    )
            }
        }
        if (updated.complexTaskQuantumMaxMs < updated.complexTaskQuantumMinMs) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "scheduler.complex-task-quantum-max-ms must be >= scheduler.complex-task-quantum-min-ms"
            )
        }
        if (updated.schedulerProgressCheapGapThreshold > updated.schedulerProgressFastGapThreshold) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "scheduler.progress.cheap-gap-threshold must be <= scheduler.progress.fast-gap-threshold"
            )
        }
        return updated
    }

    private fun parseLongConfigValue(key: String, raw: String, minValue: Long): Long {
        val parsed = raw.trim().toLongOrNull()
            ?: throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Invalid long value for '$key': '$raw'"
            )
        return parsed.coerceAtLeast(minValue)
    }

    private fun parseIntConfigValue(key: String, raw: String, minValue: Int): Int {
        val parsed = raw.trim().toIntOrNull()
            ?: throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Invalid int value for '$key': '$raw'"
            )
        return parsed.coerceAtLeast(minValue)
    }

    private fun parseDoubleConfigValue(key: String, raw: String, minValue: Double): Double {
        val parsed = raw.trim().toDoubleOrNull()
            ?: throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Invalid double value for '$key': '$raw'"
            )
        if (!parsed.isFinite()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Invalid double value for '$key': '$raw'"
            )
        }
        return parsed.coerceAtLeast(minValue)
    }

    private fun parseDoubleConfigValueInRange(
        key: String,
        raw: String,
        minValue: Double,
        maxValue: Double
    ): Double {
        val parsed = parseDoubleConfigValue(key, raw, minValue)
        if (parsed > maxValue) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Invalid double value for '$key': '$raw'. Expected <= $maxValue"
            )
        }
        return parsed
    }

    private fun parseBooleanConfigValue(key: String, raw: String): Boolean = when (raw.trim().lowercase()) {
        "true", "1", "yes", "y", "on" -> true
        "false", "0", "no", "n", "off" -> false
        else -> throw RemoteSolverException(
            code = RemoteSolverErrorCode.INVALID_ARGUMENT,
            message = "Invalid boolean value for '$key': '$raw'"
        )
    }

    private suspend fun markFailed(
        task: TaskState,
        reason: String,
        reasonCode: RemoteSolverErrorCode = RemoteSolverErrorCode.TASK_FAILED
    ): TaskState {
        val normalizedReasonCode = RemoteSolverErrorMapper.reasonCodeOf(reasonCode)
        val previousResult = task.latestResult
        val preservedFeasible = previousResult?.feasible ?: false
        val failed = task.copy(
            status = TaskStatus.FAILED,
            latestResult = SolveResult(
                feasible = preservedFeasible,
                optimal = false,
                objectiveValue = previousResult?.objectiveValue,
                objectiveValueInt64 = previousResult?.objectiveValueInt64,
                gap = previousResult?.gap,
                elapsed = (previousResult?.elapsedMs ?: 0L).toDuration(DurationUnit.MILLISECONDS),
                checkpointRef = task.effectiveCheckpointRef(),
                resultRef = null,
                message = reason,
                extension = mapOf("reasonCode" to normalizedReasonCode),
                schemaVersion = previousResult?.schemaVersion
                    ?.takeIf { it.substringBefore('.').toIntOrNull() == 2 }
                    ?: "2.0",
                problemStatus = if (preservedFeasible) {
                    RemoteProblemStatus.FEASIBLE
                } else {
                    RemoteProblemStatus.UNKNOWN
                },
                terminationReason = when (reasonCode) {
                    RemoteSolverErrorCode.TASK_FAILED_SLICE_TIMEOUT,
                    RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT -> RemoteTerminationReason.TIME_LIMIT
                    else -> RemoteTerminationReason.BACKEND_FAILURE
                },
                solutionPresence = if (preservedFeasible) {
                    RemoteSolutionPresence.INCUMBENT
                } else {
                    RemoteSolutionPresence.NONE
                },
                proofStatus = RemoteProofStatus.NONE,
                provenance = previousResult?.provenance ?: emptyMap(),
                fingerprints = previousResult?.fingerprints ?: emptyMap(),
                fingerprintSchemas = previousResult?.fingerprintSchemas ?: emptyMap(),
                statistics = previousResult?.statistics ?: emptyMap(),
                diagnostics = previousResult?.diagnostics ?: emptyMap(),
                runId = previousResult?.runId,
                attemptId = previousResult?.attemptId,
                artifactDigest = null,
                incumbentRef = previousResult?.incumbentRef,
                modelFingerprint = previousResult?.modelFingerprint,
                scheduling = previousResult?.scheduling,
                outcome = SliceOutcome.FAILED,
                cancellationChain = previousResult?.cancellationChain ?: emptyList()
            ),
            updatedAt = clock.now()
        )
        taskStatePort.upsertTask(failed)
        clearLearningState(failed.taskId.value)
        val resultPayload = TaskResultPayload(
            taskId = failed.taskId.value,
            status = failed.status.name,
            reasonCode = normalizedReasonCode,
            message = reason
        )
        publishEvent(
            topic = EventTopics.TASK_RESULT,
            key = failed.taskId,
            payload = resultPayload.toByteArray(),
            tenantId = failed.tenantId,
            idempotencyKey = "task:${failed.taskId}:failed"
        )
        return failed
    }

    private suspend fun recoverTimedOutNodeTasks(nodeId: String, now: Long): Int {
        val impactedTasks = taskStatePort.listTasks(
            statuses = setOf(TaskStatus.DISPATCHING, TaskStatus.RUNNING),
            limit = Int.MAX_VALUE
        ).filter { it.assignedNodeId?.value == nodeId }
        if (impactedTasks.isEmpty()) {
            return 0
        }

        var recovered = 0
        for (task in impactedTasks) {
            val slices = taskStatePort.getSlices(task.taskId)
            val failedSlices = mutableListOf<SliceState>()
            for (slice in slices) {
                if (slice.nodeId?.value != nodeId) {
                    continue
                }
                if (slice.status !in setOf(SliceStatus.PLANNED, SliceStatus.RUNNING, SliceStatus.CHECKPOINTING)) {
                    continue
                }
                val failedSlice = slice.copy(
                    status = SliceStatus.FAILED,
                    finishedAt = kotlin.time.Instant.fromEpochMilliseconds(now),
                    error = "Node heartbeat timeout"
                )
                taskStatePort.updateSlice(failedSlice)
                failedSlices += failedSlice
            }

            val nextStatus = if (task.effectiveCheckpointRef() != null) TaskStatus.SUSPENDED else TaskStatus.QUEUED
            taskStatePort.upsertTask(
                task.copy(
                    status = nextStatus,
                    assignedNodeId = null,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(now)
                )
            )
            metricsPort.increment("task.recovered", tags = mapOf("reason" to "node_timeout"))
            failedSlices.forEach { failedSlice ->
                publishSliceLifecycle(
                    slice = failedSlice,
                    action = "slice-end",
                    taskStatus = nextStatus,
                    tenantId = task.tenantId,
                    reason = "node_timeout",
                    idempotencySuffix = "recovered:node_timeout"
                )
            }
            recovered += 1
        }
        return recovered
    }

    private fun computeCost(task: TaskState, sliceId: String, node: NodeState, runtimeMs: Long): CostRecord {
        val billedSeconds = maxOf(
            node.profile.minBillingUnitSeconds,
            ceil(runtimeMs.toDouble() / 1000.0).toLong()
        )
        val usage = billedSeconds * node.profile.pricePerSecond.toDouble()
        val total = usage + node.profile.licenseCostPerSlice.toDouble()
        return CostRecord(
            taskId = task.taskId.value,
            budgetScope = task.effectiveBudgetScope().value,
            sliceId = sliceId,
            nodeId = node.nodeId.value,
            runtimeMs = runtimeMs,
            billedSeconds = billedSeconds,
            pricePerSecond = node.profile.pricePerSecond.toDouble(),
            licenseCost = node.profile.licenseCostPerSlice.toDouble(),
            totalCost = total,
            createdAtEpochMs = clock.nowEpochMs()
        )
    }

    private fun wouldExceedBudget(task: TaskState, node: NodeState, quantumMs: Long): Boolean {
        val budgetLimit = task.effectiveBudgetLimit() ?: return false
        val estimatedCost = estimateSliceCost(node, quantumMs)
        return task.consumedCost.toDouble() + estimatedCost > budgetLimit.toDouble()
    }

    private fun estimateSliceCost(node: NodeState, runtimeMs: Long): Double {
        val billedSeconds = maxOf(
            node.profile.minBillingUnitSeconds,
            ceil(max(0L, runtimeMs).toDouble() / 1000.0).toLong()
        )
        return billedSeconds * node.profile.pricePerSecond.toDouble() + node.profile.licenseCostPerSlice.toDouble()
    }

    private suspend fun updateNodePerformanceScore(
        taskId: String,
        nodeId: String,
        quantumMs: Long,
        sliceResult: SliceResult
    ) {
        if (!config.performanceLearningEnabled) {
            return
        }
        val node = nodeStatePort.getNode(nodeId) ?: return
        val minScoreBound = max(0.1, config.performanceScoreMin)
        val maxScoreBound = max(minScoreBound, config.performanceScoreMax)
        val currentScore = max(minScoreBound, node.profile.performanceScore.toDouble())
        val observedScore = observePerformanceSignal(taskId, quantumMs, sliceResult)
        val learningRate = config.performanceLearningRate.coerceIn(0.0, 1.0)
        val rawUpdatedScore = if (learningRate <= 0.0) {
            currentScore
        } else {
            currentScore * (1.0 - learningRate) + observedScore * learningRate
        }
        val updatedScore = rawUpdatedScore.coerceIn(minScoreBound, maxScoreBound)
        if (abs(updatedScore - node.profile.performanceScore.toDouble()) <= 1e-9) {
            return
        }
        nodeStatePort.upsertNode(
            node.copy(
                profile = node.profile.copy(performanceScore = Flt64(updatedScore))
            )
        )
        metricsPort.gauge(
            "node.performance.score",
            updatedScore,
            tags = mapOf("nodeId" to nodeId)
        )
        publishEvent(
            topic = EventTopics.COST_AND_CAPABILITY_UPDATE,
            key = nodeId,
            payload = CostUpdatePayload(
                nodeId = nodeId,
                performanceScore = updatedScore
            ).toByteArray(),
            tenantId = "system",
            idempotencyKey = "node:$nodeId:performance:${sliceResult.sliceId}"
        )
    }

    private fun observePerformanceSignal(taskId: String, quantumMs: Long, sliceResult: SliceResult): Double {
        val elapsedMs = max(1L, sliceResult.elapsedMs).toDouble()
        val effectiveQuantumMs = max(1L, quantumMs).toDouble()
        val runtimeSignal = (effectiveQuantumMs / elapsedMs).coerceAtLeast(0.1)
        val gapSignal = gapSignal(taskId, sliceResult.gap?.toDouble())
        val completionSignal = if (sliceResult.completed) 1.1 else 1.0
        val feasibleSignal = if (sliceResult.feasible) 1.05 else 1.0
        return (runtimeSignal * gapSignal * completionSignal * feasibleSignal).coerceAtLeast(0.1)
    }

    private fun gapSignal(taskId: String, gap: Double?): Double {
        val normalizedGap = gap?.coerceIn(0.0, 1.0) ?: return 1.0
        val previous = latestGapByTaskId.put(taskId, normalizedGap)
        if (previous == null) {
            progressSignalByTaskId[taskId] = ProgressSignal(currentGap = normalizedGap)
            return (1.0 + (1.0 - normalizedGap) * 0.2).coerceIn(0.5, 2.0)
        }
        val rawImprovement = previous - normalizedGap
        val previousSignal = progressSignalByTaskId[taskId]
        val noImprovementSlices = if (rawImprovement >= 0.01) {
            0
        } else {
            (previousSignal?.noImprovementSlices ?: 0) + 1
        }
        progressSignalByTaskId[taskId] = ProgressSignal(
            previousGap = previous,
            currentGap = normalizedGap,
            improvement = rawImprovement,
            noImprovementSlices = noImprovementSlices
        )
        val improvement = (previous - normalizedGap).coerceAtLeast(0.0)
        val regression = (normalizedGap - previous).coerceAtLeast(0.0)
        return (1.0 + improvement * 2.0 - regression).coerceIn(0.5, 2.0)
    }

    private fun clearLearningState(taskId: String) {
        latestGapByTaskId.remove(taskId)
        progressSignalByTaskId.remove(taskId)
        lastDeadlineUrgencyByTaskId.remove(taskId)
        lastDispatchAtByTaskId.remove(taskId)
    }

    private fun TaskState.summaryPayload(): ByteArray =
        TaskSummaryPayload(
            taskId = taskId.value,
            status = status.name,
            priority = priority
        ).toByteArray()

    private fun CostRecord.summaryPayload(): ByteArray =
        CostUpdatePayload(
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            totalCost = totalCost,
            runtimeMs = runtimeMs
        ).toByteArray()

    private suspend fun publishEvent(
        topic: String,
        key: Any,
        payload: ByteArray,
        tenantId: Any,
        idempotencyKey: String? = null
    ) {
        val tenantIdValue = tenantId.toString()
        val traceId = tracingPort.currentTraceId()
        val spanId = tracingPort.currentSpanId()
        val base = mutableMapOf(EventEnvelopeHeaders.TENANT_ID to tenantIdValue)
        traceId?.let { base[EventEnvelopeHeaders.TRACE_ID] = it }
        spanId?.let { base[EventEnvelopeHeaders.SPAN_ID] = it }
        val headers = if (idempotencyKey.isNullOrBlank()) {
            base.toMap()
        } else {
            base["idempotencyKey"] = idempotencyKey
            base.toMap()
        }
        eventPort.publish(topic = topic, key = key.toString(), payload = payload, headers = headers)
    }

    private fun resolveSliceOutcome(
        result: SliceResult,
        checkpointRef: ObjectRef?,
        timeoutReason: String?
    ): SliceOutcome = when {
        timeoutReason != null -> SliceOutcome.FAILED
        result.terminationReason == RemoteTerminationReason.CANCELLED -> SliceOutcome.CANCELLED
        result.terminationReason in setOf(
            RemoteTerminationReason.BACKEND_FAILURE,
            RemoteTerminationReason.NUMERICAL_FAILURE,
            RemoteTerminationReason.INTERRUPTED
        ) -> SliceOutcome.FAILED
        result.completed || isTerminalTermination(result.terminationReason) -> SliceOutcome.COMPLETED
        checkpointRef != null -> SliceOutcome.CHECKPOINTED
        result.feasible && result.solutionPresence != RemoteSolutionPresence.NONE -> SliceOutcome.RESUMABLE
        else -> SliceOutcome.PREEMPTED
    }

    private fun schedulingDecision(
        task: TaskState,
        slice: SliceState,
        node: NodeState,
        quantumMs: Long,
        audit: SliceAuditRecord,
        checkpointRef: ObjectRef?,
        incumbentRef: ObjectRef?,
        outcome: SliceOutcome,
        reason: String?
    ): SchedulingDecision {
        val request = task.payload.scheduling
        val metadata = request?.metadata.orEmpty() + mapOf(
            "admissionClass" to audit.admissionClass.name,
            "selectionReason" to audit.selectionReason,
            "quantumReason" to audit.quantumReason,
            "migrationDecision" to audit.migrationDecision,
            "eligibleNodeIds" to audit.eligibleNodeIds.joinToString(",")
        )
        return SchedulingDecision(
            dispatchId = slice.dispatchId,
            taskId = task.taskId,
            sliceId = slice.sliceId,
            nodeId = node.nodeId,
            priority = task.effectivePriority(),
            deadline = task.effectiveDeadline(),
            budgetScope = task.effectiveBudgetScope(),
            budgetLimit = task.effectiveBudgetLimit(),
            quantum = quantumMs.toDuration(DurationUnit.MILLISECONDS),
            queueWait = task.effectiveQueueWaitEstimateMs()?.toDuration(DurationUnit.MILLISECONDS),
            estimate = request?.estimate,
            modelFingerprint = request?.modelFingerprint ?: task.payload.extension["modelFingerprint"],
            modelFingerprintSchema = request?.modelFingerprintSchema,
            checkpointRef = checkpointRef ?: task.effectiveCheckpointRef(),
            incumbentRef = incumbentRef ?: request?.incumbentRef,
            qualityTarget = request?.qualityTarget,
            preemptionMode = request?.preemptionMode,
            resumeMode = request?.resumeMode,
            outcome = outcome,
            reason = reason,
            metadata = metadata
        )
    }

    private fun mergeSchedulingDecision(
        existing: SchedulingDecision?,
        effective: SchedulingDecision
    ): SchedulingDecision {
        if (existing == null) {
            return effective
        }
        return existing.copy(
            dispatchId = effective.dispatchId ?: existing.dispatchId,
            taskId = effective.taskId ?: existing.taskId,
            sliceId = effective.sliceId ?: existing.sliceId,
            nodeId = effective.nodeId ?: existing.nodeId,
            priority = effective.priority ?: existing.priority,
            deadline = effective.deadline ?: existing.deadline,
            budgetScope = effective.budgetScope ?: existing.budgetScope,
            budgetLimit = effective.budgetLimit ?: existing.budgetLimit,
            quantum = effective.quantum ?: existing.quantum,
            queueWait = effective.queueWait ?: existing.queueWait,
            estimate = existing.estimate ?: effective.estimate,
            modelFingerprint = effective.modelFingerprint ?: existing.modelFingerprint,
            modelFingerprintSchema = effective.modelFingerprintSchema ?: existing.modelFingerprintSchema,
            checkpointRef = effective.checkpointRef ?: existing.checkpointRef,
            incumbentRef = effective.incumbentRef ?: existing.incumbentRef,
            qualityTarget = existing.qualityTarget ?: effective.qualityTarget,
            preemptionMode = effective.preemptionMode ?: existing.preemptionMode,
            resumeMode = effective.resumeMode ?: existing.resumeMode,
            outcome = effective.outcome ?: existing.outcome,
            reason = effective.reason ?: existing.reason,
            metadata = existing.metadata + effective.metadata
        )
    }

    private suspend fun publishSliceLifecycle(
        slice: SliceState,
        action: String,
        taskStatus: TaskStatus,
        tenantId: Any,
        reason: String? = null,
        audit: SliceAuditRecord? = null,
        idempotencySuffix: String
    ) {
        val payload = SliceLifecyclePayload(
            taskId = slice.taskId.value,
            sliceId = slice.sliceId.value,
            dispatchId = slice.dispatchId.value,
            action = action,
            status = slice.status.name,
            taskStatus = taskStatus.name,
            nodeId = slice.nodeId?.value,
            quantumMs = slice.quantum.inWholeMilliseconds,
            reason = reason,
            admissionClass = audit?.admissionClass?.name,
            selectionReason = audit?.selectionReason,
            selectionScore = audit?.selectionScore,
            quantumReason = audit?.quantumReason,
            migrationDecision = audit?.migrationDecision,
            eligibleNodeIds = audit?.eligibleNodeIds ?: emptyList()
        )
        publishEvent(
            topic = EventTopics.SLICE_LIFECYCLE,
            key = slice.sliceId,
            tenantId = tenantId,
            payload = payload.toByteArray(),
            idempotencyKey = "slice:${slice.sliceId}:$idempotencySuffix"
        )
    }

    private suspend fun shouldUseHardInterrupt(task: TaskState): Boolean {
        val nodeId = task.assignedNodeId ?: return true
        val node = nodeStatePort.getNode(nodeId.value) ?: return true
        return node.profile.supportsInterrupt
    }

    private fun normalizeSubmission(
        complexity: TaskComplexity,
        timeSensitivity: TimeSensitivity
    ): Pair<TimeSensitivity, Boolean> {
        if (complexity == TaskComplexity.COMPLEX && timeSensitivity == TimeSensitivity.REALTIME) {
            return TimeSensitivity.NON_REALTIME to true
        }
        return timeSensitivity to false
    }

    private fun inferComplexity(payload: SolvePayload): TaskComplexity {
        val meta = payload.taskMeta
        return inferComplexity(meta)
    }

    private fun inferComplexity(meta: TaskMeta): TaskComplexity {
        val variableCount = meta.estimatedVariableCount
        if (variableCount != null && variableCount >= config.complexTaskVariableThreshold) {
            return TaskComplexity.COMPLEX
        }
        val constraintCount = meta.estimatedConstraintCount
        if (constraintCount != null && constraintCount >= config.complexTaskConstraintThreshold) {
            return TaskComplexity.COMPLEX
        }
        val historicalRuntimeMs = meta.historicalRuntimeMs
        if (historicalRuntimeMs != null && historicalRuntimeMs >= config.complexTaskHistoricalRuntimeThresholdMs) {
            return TaskComplexity.COMPLEX
        }
        return TaskComplexity.SIMPLE
    }

    private fun resolveTenantId(tenantId: TenantId?, payload: SolvePayload): TenantId {
        val fromArg = tenantId?.value
        val fromExtension = payload.extension["tenantId"]?.trim()?.takeIf { it.isNotEmpty() }
        val resolved = fromArg ?: fromExtension ?: "default"
        require(Regex("^[A-Za-z0-9._-]+$").matches(resolved)) {
            "tenantId contains illegal characters: '$resolved'"
        }
        return TenantId.of(resolved)
    }

    private fun normalizeBudgetScope(
        budgetScope: BudgetScopeId?,
        tenantId: TenantId,
        taskId: TaskId
    ): BudgetScopeId {
        val rawScope = budgetScope?.value ?: taskId.value
        if (rawScope.startsWith("${tenantId.value}:")) {
            return BudgetScopeId.of(rawScope)
        }
        return BudgetScopeId.of("${tenantId.value}:$rawScope")
    }

    private fun classifyAdmission(task: TaskState, nodes: List<NodeState>): AdmissionDecision {
        val decisions = nodes.map { node ->
            val reasons = buildList {
                if (!node.online) add(AdmissionReasonCode.OFFLINE)
                if (node.availableUnits <= 0) add(AdmissionReasonCode.NO_AVAILABLE_SLOT)
                if (!isNodeCompatible(task, node)) {
                    val modelType = resolvedModelType(task)
                    val requiredSolverType = task.payload.taskMeta.solverType?.value
                        ?: task.payload.extension["solverType"]
                        ?: task.payload.taskMeta.metadata["solverType"]
                     if (modelType !in node.profile.supportedModelTypes) {
                        add(AdmissionReasonCode.MODEL_UNSUPPORTED)
                    }
                    if (requiredSolverType != null &&
                        !node.profile.solverType.value.equals(requiredSolverType, ignoreCase = true)
                    ) {
                        add(AdmissionReasonCode.SOLVER_UNSUPPORTED)
                    }
                }
                if (requiresCapability(task, "requiresCheckpoint") && !node.profile.supportsCheckpoint) {
                    add(AdmissionReasonCode.CHECKPOINT_UNSUPPORTED)
                }
                if (requiresCapability(task, "requiresWarmStart") && !node.profile.supportsWarmStart) {
                    add(AdmissionReasonCode.WARM_START_UNSUPPORTED)
                }
                if (requiresCapability(task, "requiresInterrupt") && !node.profile.supportsInterrupt) {
                    add(AdmissionReasonCode.INTERRUPT_UNSUPPORTED)
                }
                addAll(task.schedulingModeAdmissionReasons(node))
                checkpointResumeAdmissionReason(task, node)?.let(::add)
            }
            NodeAdmissionDecision(
                nodeId = node.nodeId.value,
                eligible = reasons.isEmpty(),
                reasons = reasons
            )
        }
        val eligible = decisions.any { it.eligible }
        val reason = when {
            eligible -> AdmissionReasonCode.ACCEPTED
            decisions.isEmpty() -> AdmissionReasonCode.NO_COMPATIBLE_NODE
            decisions.all { AdmissionReasonCode.NO_AVAILABLE_SLOT in it.reasons } ->
                AdmissionReasonCode.NO_AVAILABLE_SLOT
            else -> decisions.firstOrNull { it.reasons.isNotEmpty() }?.reasons?.first()
                ?: AdmissionReasonCode.NO_COMPATIBLE_NODE
        }
        return AdmissionDecision(
            taskId = task.taskId.value,
            taskClass = TaskAdmissionClass.of(task),
            admitted = eligible,
            reason = reason,
            nodes = decisions
        )
    }

    private fun isNodeExecutionCapable(task: TaskState, node: NodeState): Boolean =
        node.online &&
            node.availableUnits > 0 &&
            isNodeCompatible(task, node) &&
            (!requiresCapability(task, "requiresCheckpoint") || node.profile.supportsCheckpoint) &&
            (!requiresCapability(task, "requiresWarmStart") || node.profile.supportsWarmStart) &&
            (!requiresCapability(task, "requiresInterrupt") || node.profile.supportsInterrupt) &&
            task.schedulingModeAdmissionReasons(node).isEmpty() &&
            checkpointResumeAdmissionReason(task, node) == null

    private fun requiresCapability(task: TaskState, key: String): Boolean =
        task.payload.extension[key]?.trim()?.equals("true", ignoreCase = true) == true ||
            task.payload.taskMeta.metadata[key]?.trim()?.equals("true", ignoreCase = true) == true

    /** Capability required to consume an existing checkpoint, if any. */
    private fun checkpointResumeAdmissionReason(
        task: TaskState,
        node: NodeState
    ): AdmissionReasonCode? {
        if (task.effectiveCheckpointRef() == null) {
            return null
        }
        return when (task.payload.scheduling?.resumeMode) {
            fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.NATIVE_CHECKPOINT ->
                if (node.profile.supportsNativeCheckpoint) null else AdmissionReasonCode.CHECKPOINT_UNSUPPORTED

            fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.BASIS ->
                AdmissionReasonCode.WARM_START_UNSUPPORTED

            else ->
                if (node.profile.supportsWarmStart) null else AdmissionReasonCode.WARM_START_UNSUPPORTED
        }
    }

    private fun budgetSelectionDecision(
        task: TaskState,
        selectedNode: NodeState,
        candidates: List<NodeState>
    ): NodeSelectionDecision = NodeSelectionDecision(
        selectedNode = selectedNode,
        reason = "budget_degraded",
        candidates = candidates.map { node ->
            NodeSelectionCandidate(
                nodeId = node.nodeId.value,
                eligible = node.nodeId == selectedNode.nodeId || isNodeExecutionCapable(task, node),
                reasons = if (node.nodeId == selectedNode.nodeId) {
                    listOf(AdmissionReasonCode.ACCEPTED)
                } else {
                    emptyList()
                }
            )
        },
        migrationDecision = if (task.assignedNodeId == selectedNode.nodeId) "held" else "migrated"
    )

    private fun isNodeCompatible(task: TaskState, node: NodeState): Boolean {
        val modelType = resolvedModelType(task) ?: return false
        // UNKNOWN is an explicit capability value, never a wildcard. Opaque
        // references may run only on nodes that advertise UNKNOWN support.
        if (modelType !in node.profile.supportedModelTypes) {
            return false
        }
        val requiredSolverType = task.payload.taskMeta.solverType?.value
            ?: task.payload.extension["solverType"]
            ?: task.payload.taskMeta.metadata["solverType"]
            ?: return true
        return node.profile.solverType.value.equals(requiredSolverType, ignoreCase = true)
    }

    /**
     * Resolve the task model type using the same explicit declarations as the
     * scheduler engine.  An opaque reference remains UNKNOWN; it must never be
     * treated as a wildcard for a concrete node capability.
     *
     * 使用与调度引擎相同的显式声明解析模型类型。不可推断的引用保持 UNKNOWN，
     * 绝不能把 UNKNOWN 当作具体节点能力的通配符。
     */
    private fun resolvedModelType(task: TaskState): NormalizedModelType? {
        val modelType = task.payload.modelData.modelType
        if (modelType != NormalizedModelType.UNKNOWN) {
            return modelType
        }
        val declared = task.payload.extension["modelType"]
            ?: task.payload.taskMeta.metadata["modelType"]
            ?: task.payload.taskMeta.targetType?.value
        val parsed = declared?.trim()?.uppercase()?.let {
            when (it) {
                "LINEAR", "LP" -> NormalizedModelType.LINEAR
                "QUADRATIC", "QP" -> NormalizedModelType.QUADRATIC
                "CP", "CONSTRAINT-PROGRAMMING", "CONSTRAINT_PROGRAMMING" -> NormalizedModelType.CP
                "UNKNOWN" -> NormalizedModelType.UNKNOWN
                else -> null
            }
        }
        if (parsed != null) {
            return parsed
        }
        return if (task.payload.modelData.ref != null && task.payload.modelData.format == null) {
            NormalizedModelType.UNKNOWN
        } else {
            null
        }
    }

    private suspend fun resolveAwaitException(task: TaskState, maxRounds: UInt64): RemoteSolverException {
        val onlineNodes = nodeStatePort.listNodes(onlineOnly = true)
        val compatibleNodes = onlineNodes.filter { isNodeCompatible(task, it) }
        val compatibleNodeCount = compatibleNodes.size
        val budgetExhausted = isBudgetExhausted(task)
        val budgetAdmissionBlocked = isBudgetAdmissionBlocked(task, compatibleNodes)
        val taskFailureCode = task.latestResult?.extension
            ?.get("reasonCode")
            ?.let { runCatching { RemoteSolverErrorCode.valueOf(it) }.getOrNull() }
        val code = when {
            task.status == TaskStatus.FAILED && taskFailureCode != null ->
                taskFailureCode

            task.status == TaskStatus.FAILED && (budgetExhausted || budgetAdmissionBlocked) ->
                RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED

            task.status == TaskStatus.FAILED ->
                RemoteSolverErrorCode.TASK_FAILED

            compatibleNodeCount == 0 ->
                RemoteSolverErrorCode.NO_COMPATIBLE_NODE_AVAILABLE

            else ->
                RemoteSolverErrorCode.TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS
        }
        val message = when (code) {
            RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED ->
                "Task ${task.taskId} failed because budget is exhausted."

            RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT ->
                "Task ${task.taskId} failed because hard timeout is exceeded."

            RemoteSolverErrorCode.TASK_FAILED_SLICE_TIMEOUT ->
                "Task ${task.taskId} failed because slice timeout is exceeded."

            RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED ->
                "Task ${task.taskId} failed due to solver execution error."

            RemoteSolverErrorCode.TASK_FAILED ->
                "Task ${task.taskId} failed before completion."

            RemoteSolverErrorCode.NO_COMPATIBLE_NODE_AVAILABLE ->
                "Task ${task.taskId} has no compatible online node."

            RemoteSolverErrorCode.TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS ->
                "Task ${task.taskId} does not reach terminal status within maxRounds=$maxRounds, currentStatus=${task.status}."

            else ->
                "Task ${task.taskId} await exits unexpectedly."
        }
        return RemoteSolverException(
            code = code,
            message = message,
            metadata = mapOf(
                "taskId" to task.taskId.value,
                "maxRounds" to maxRounds.toString(),
                "currentStatus" to task.status.name,
                "compatibleNodeCount" to compatibleNodeCount.toString(),
                "budgetExhausted" to budgetExhausted.toString(),
                "budgetAdmissionBlocked" to budgetAdmissionBlocked.toString()
            )
        )
    }

    private suspend fun isBudgetExhausted(task: TaskState): Boolean {
        val budgetLimit = task.effectiveBudgetLimit() ?: return false
        if (task.consumedCost.toDouble() >= budgetLimit.toDouble()) {
            return true
        }
        val snapshot = budgetPort.snapshot(task.effectiveBudgetScope().value) ?: return false
        return snapshot.remaining <= 0.0
    }

    private suspend fun isBudgetAdmissionBlocked(task: TaskState, compatibleNodes: List<NodeState>): Boolean {
        val budgetLimit = task.effectiveBudgetLimit() ?: return false
        if (compatibleNodes.isEmpty()) {
            return false
        }
        val remaining = budgetPort.snapshot(task.effectiveBudgetScope().value)?.remaining
            ?: (budgetLimit.toDouble() - task.consumedCost.toDouble())
        val minEstimatedSliceCost = compatibleNodes.minOfOrNull { node ->
            estimateSliceCost(node, computeQuantumMs(task, node))
        } ?: return false
        return remaining < minEstimatedSliceCost
    }
}
