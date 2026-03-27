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
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.DispatchId
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorMapper
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
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
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.runBlocking
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
    private val schedulerConfigAuditPort: SchedulerConfigAuditPort? = null
) {
    companion object {
        private val TERMINAL_STATUSES = setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)
    }

    private val runningHandleByTaskId = ConcurrentHashMap<String, ExecutionHandle>()
    private val latestGapByTaskId = ConcurrentHashMap<String, Double>()
    private val dispatcherId = idGenerator.newId("dispatcher")
    private val runtimeSchedulerConfigRef = AtomicReference(SchedulerRuntimeConfig.from(config))
    private val schedulerConfigVersionRef = AtomicReference(config.schedulerConfigVersion.ifBlank { "v0" })
    private val schedulerHotReloadSeq = AtomicInteger(0)
    private val schedulerSnapshots = ConcurrentHashMap<String, SchedulerRuntimeConfig>()
    private val schedulerHotReloadAudits = CopyOnWriteArrayList<SchedulerHotReloadAuditRecord>()

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
                    val latest = persisted.maxByOrNull { it.effectiveAtEpochMs }
                    if (latest != null) {
                        schedulerSnapshots[latest.version]?.let { snapshot ->
                            runtimeSchedulerConfigRef.set(snapshot)
                            schedulerConfigVersionRef.set(latest.version)
                        }
                    }
                }
            }
        }
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
        val normalizedOperator = operator.trim().ifEmpty { "unknown" }
        val previousVersion = schedulerConfigVersionRef.get()
        val previous = runtimeSchedulerConfigRef.get()
        val next = applySchedulerChangeSet(previous, changeSet)
        val newVersion = requestedVersion?.trim()?.takeIf { it.isNotEmpty() }
            ?: "hot-${effectiveAtEpochMs}-${schedulerHotReloadSeq.incrementAndGet()}"
        require(newVersion != previousVersion) {
            "requestedVersion must be different from current version '$previousVersion'"
        }

        runtimeSchedulerConfigRef.set(next)
        schedulerConfigVersionRef.set(newVersion)
        schedulerSnapshots[newVersion] = next
        val audit = SchedulerHotReloadAuditRecord(
            version = newVersion,
            previousVersion = previousVersion,
            operator = normalizedOperator,
            effectiveAtEpochMs = effectiveAtEpochMs,
            changeSet = changeSet.toMap(),
            rollbackFromVersion = null
        )
        schedulerHotReloadAudits.add(audit)
        schedulerConfigAuditPort?.saveSnapshot(newVersion, next)
        schedulerConfigAuditPort?.append(audit)
        return audit
    }

    suspend fun rollbackSchedulerHotReload(
        targetVersion: String,
        operator: String,
        effectiveAtEpochMs: Long = clock.nowEpochMs(),
        requestedVersion: String? = null
    ): SchedulerHotReloadAuditRecord {
        ensureHotReloadEnabled()
        val normalizedTargetVersion = targetVersion.trim()
        require(normalizedTargetVersion.isNotEmpty()) { "targetVersion must not be blank" }
        val snapshot = schedulerSnapshots[normalizedTargetVersion]
            ?: throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Unknown scheduler config version '$normalizedTargetVersion'"
            )
        val previousVersion = schedulerConfigVersionRef.get()
        val newVersion = requestedVersion?.trim()?.takeIf { it.isNotEmpty() }
            ?: "rollback-${normalizedTargetVersion}-${schedulerHotReloadSeq.incrementAndGet()}"
        require(newVersion != previousVersion) {
            "requestedVersion must be different from current version '$previousVersion'"
        }
        runtimeSchedulerConfigRef.set(snapshot)
        schedulerConfigVersionRef.set(newVersion)
        schedulerSnapshots[newVersion] = snapshot
        val audit = SchedulerHotReloadAuditRecord(
            version = newVersion,
            previousVersion = previousVersion,
            operator = operator.trim().ifEmpty { "unknown" },
            effectiveAtEpochMs = effectiveAtEpochMs,
            changeSet = snapshot.toChangeSet(),
            rollbackFromVersion = normalizedTargetVersion
        )
        schedulerHotReloadAudits.add(audit)
        schedulerConfigAuditPort?.saveSnapshot(newVersion, snapshot)
        schedulerConfigAuditPort?.append(audit)
        return audit
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

        // Use tenant-scoped idempotency check
        val existingTask = taskStatePort.getTaskByRequestId(resolvedTenantId, stableRequestId)
        if (existingTask != null) {
            return existingTask
        }

        val resolvedComplexity = complexity ?: inferComplexity(payload)
        val resolvedTimeSensitivity = timeSensitivity ?: TimeSensitivity.NON_REALTIME
        val (normalizedTimeSensitivity, downgradedRealtimeComplexTask) =
            normalizeSubmission(resolvedComplexity, resolvedTimeSensitivity)
        val taskId = TaskId.of(idGenerator.newId("task"))
        val scope = normalizeBudgetScope(budgetScope, resolvedTenantId, taskId)
        if (budgetLimit != null) {
            budgetPort.configureBudget(scope.value, budgetLimit.toDouble())
        }
        val task = TaskState(
            taskId = taskId,
            requestId = stableRequestId,
            tenantId = resolvedTenantId,
            status = TaskStatus.QUEUED,
            complexity = resolvedComplexity,
            timeSensitivity = normalizedTimeSensitivity,
            priority = priority,
            deadline = deadline,
            payload = payload,
            createdAt = now,
            updatedAt = now,
            budgetScope = scope,
            budgetLimit = budgetLimit
        )
        taskStatePort.upsertTask(task)
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

            val activeSlices = taskStatePort.getSlices(task.taskId)
                .filter { it.status in setOf(SliceStatus.PLANNED, SliceStatus.RUNNING, SliceStatus.CHECKPOINTING) }
            activeSlices.forEach { slice ->
                taskStatePort.updateSlice(
                    slice.copy(
                        status = SliceStatus.FAILED,
                        finishedAt = now,
                        error = reason
                    )
                )
            }

            val stopped = task.copy(
                status = TaskStatus.STOPPED,
                assignedNodeId = null,
                updatedAt = now
            )
            taskStatePort.upsertTask(stopped)
            clearLearningState(task.taskId.value)
            task.assignedNodeId?.let { nodeId ->
                nodeStatePort.releaseUnit(nodeId.value)
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
                    val resumedStatus = if (task.latestSnapshotRef != null || task.complexity == TaskComplexity.COMPLEX) {
                        TaskStatus.SUSPENDED
                    } else {
                        TaskStatus.QUEUED
                    }
                    val resumed = task.copy(
                        status = resumedStatus,
                        assignedNodeId = null,
                        updatedAt = now
                    )
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
                TaskStatus.ACCEPTED -> task

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

        try {
            val latestTask = taskStatePort.getTask(task.taskId) ?: return null
            if (latestTask.status !in setOf(TaskStatus.QUEUED, TaskStatus.SUSPENDED, TaskStatus.ACCEPTED, TaskStatus.WAITING_FOR_BUDGET)) {
                return null
            }
            val hardTimeoutReason = hardTimeoutReason(latestTask)
            if (hardTimeoutReason != null) {
                metricsPort.increment("task.failed", tags = mapOf("reason" to "hard_timeout"))
                return markFailed(latestTask, hardTimeoutReason, RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT)
            }

            // Gather available nodes first
            val availableNodes = nodeStatePort.listNodes(onlineOnly = true)
                .asSequence()
                .filter { it.availableUnits > 0 }
                .filter { isNodeCompatible(latestTask, it) }
                .toList()

            // Budget check with degradation logic
            var selectedNode: NodeState? = null
            if (!checkBudget(latestTask)) {
                // Try budget degradation before failing
                val degradationResult = tryBudgetDegradation(latestTask, availableNodes)
                when (degradationResult) {
                    is BudgetDegradationResult.CanProceedWithNode -> {
                        selectedNode = degradationResult.node
                    }
                    BudgetDegradationResult.WaitForBudget -> {
                        // Mark as waiting for budget instead of failing
                        if (latestTask.status != TaskStatus.WAITING_FOR_BUDGET) {
                            val now = clock.now()
                            val waitingTask = latestTask.copy(
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
                        return null
                    }
                    BudgetDegradationResult.FailAfterDegradation -> {
                        val failed = markFailed(
                            latestTask,
                            "Budget exceeded after degradation",
                            RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED
                        )
                        metricsPort.increment("task.failed", tags = mapOf("reason" to "budget"))
                        return failed
                    }
                }
            }

            // Normal node selection if budget is OK or degradation found a node
            if (selectedNode == null) {
                if (availableNodes.isEmpty()) {
                    return null
                }
                selectedNode = schedulerEngine.chooseNode(latestTask, availableNodes) ?: return null
            }
            if (!nodeStatePort.occupyUnit(selectedNode.nodeId)) {
                return null
            }

            val acceptedTask = acceptForDispatch(latestTask)
            if (acceptedTask == null) {
                nodeStatePort.releaseUnit(selectedNode.nodeId)
                return null
            }

            val now = clock.now()
            val dispatchId = DispatchId.of(idGenerator.newId("dispatch"))
            val sliceId = SliceId.of(idGenerator.newId("slice"))
            val quantumMs = computeQuantumMs(acceptedTask, selectedNode)
            if (wouldExceedBudget(acceptedTask, selectedNode, quantumMs)) {
                nodeStatePort.releaseUnit(selectedNode.nodeId)

                // Try budget degradation before failing
                val degradationResult = tryBudgetDegradation(acceptedTask, availableNodes)
                when (degradationResult) {
                    is BudgetDegradationResult.CanProceedWithNode -> {
                        // Switch to cheaper node
                        val cheaperNode = degradationResult.node
                        if (!nodeStatePort.occupyUnit(cheaperNode.nodeId)) {
                            return null
                        }
                        selectedNode = cheaperNode
                        // Continue with cheaper node - recompute quantum
                        val cheaperQuantumMs = computeQuantumMs(acceptedTask, cheaperNode)
                        if (wouldExceedBudget(acceptedTask, cheaperNode, cheaperQuantumMs)) {
                            // Even cheaper node exceeds budget - wait
                            nodeStatePort.releaseUnit(cheaperNode.nodeId)
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
                            return null
                        }
                        // Proceed with cheaper node - fall through to dispatch logic
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
                        return null
                    }
                    BudgetDegradationResult.FailAfterDegradation -> {
                        val failed = markFailed(
                            acceptedTask,
                            "Budget exceeded after degradation",
                            RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED
                        )
                        metricsPort.increment("task.failed", tags = mapOf("reason" to "budget"))
                        return failed
                    }
                }
            }

            var currentTask = acceptedTask.copy(
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
                    quantumMs = quantumMs
                ).toByteArray(),
                tenantId = acceptedTask.tenantId,
                idempotencyKey = "dispatch:$dispatchId"
            )

            var currentSlice = SliceState(
                sliceId = sliceId,
                taskId = acceptedTask.taskId,
                dispatchId = dispatchId,
                status = SliceStatus.PLANNED,
                nodeId = selectedNode.nodeId,
                quantum = quantumMs.toDuration(DurationUnit.MILLISECONDS)
            )
            taskStatePort.appendSlice(currentSlice)

            return executeDispatchedSlice(
                acceptedTask = acceptedTask,
                initialTask = currentTask,
                initialSlice = currentSlice,
                selectedNode = selectedNode,
                sliceId = sliceId,
                quantumMs = quantumMs
            )
        } finally {
            distributedLockPort.release(lease)
        }
    }

    private suspend fun executeDispatchedSlice(
        acceptedTask: TaskState,
        initialTask: TaskState,
        initialSlice: SliceState,
        selectedNode: NodeState,
        sliceId: SliceId,
        quantumMs: Long
    ): TaskState {
        var currentTask = initialTask
        var currentSlice = initialSlice
        return try {
            val checkpointForResume = acceptedTask.latestSnapshotRef
            val resumedFromCheckpoint = checkpointForResume != null && selectedNode.profile.supportsWarmStart
            val payload = acceptedTask.payload
            val handle = if (checkpointForResume != null && selectedNode.profile.supportsWarmStart) {
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
            runningHandleByTaskId[acceptedTask.taskId.value] = handle

            currentTask = currentTask.copy(status = TaskStatus.RUNNING, updatedAt = clock.now())
            taskStatePort.upsertTask(currentTask)

            currentSlice = currentSlice.copy(status = SliceStatus.RUNNING, startedAt = clock.now())
            taskStatePort.updateSlice(currentSlice)
            publishSliceLifecycle(
                slice = currentSlice,
                action = if (resumedFromCheckpoint) "slice-resume" else "slice-start",
                taskStatus = currentTask.status,
                tenantId = acceptedTask.tenantId,
                idempotencySuffix = if (resumedFromCheckpoint) "resume" else "start"
            )

            val sliceResult = solverExecutionPort.awaitSliceEnd(handle, quantumMs)
            val sliceTimeoutReason = sliceTimeoutReason(sliceResult, quantumMs)
            val checkpointRef = if (selectedNode.profile.supportsCheckpoint) {
                currentSlice = currentSlice.copy(status = SliceStatus.CHECKPOINTING)
                taskStatePort.updateSlice(currentSlice)
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-checkpointing",
                    taskStatus = currentTask.status,
                    tenantId = acceptedTask.tenantId,
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

            if (checkpointRef != null) {
                checkpointPort.save(
                    CheckpointMetadata(
                        taskId = acceptedTask.taskId,
                        sliceId = sliceId,
                        ref = checkpointRef,
                        createdAt = clock.now()
                    )
                )
            }

            val costRecord = computeCost(acceptedTask, sliceId.value, selectedNode, sliceResult.elapsedMs)
            budgetPort.commit(acceptedTask.budgetScope.value, costRecord.totalCost)
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
                sliceResult = sliceResult
            )
            val consumedCostAfterSlice = Flt64(currentTask.consumedCost.toDouble() + costRecord.totalCost)
            val latestSnapshotAfterSlice = checkpointRef ?: currentTask.latestSnapshotRef

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

            val updatedTask = if (sliceResult.completed) {
                val finalResult = solverExecutionPort.fetchFinalResult(handle) ?: SolveResult(
                    feasible = sliceResult.feasible,
                    optimal = true,
                    objectiveValue = sliceResult.objectiveValue?.toDouble(),
                    gap = sliceResult.gap?.toDouble(),
                    elapsedMs = sliceResult.elapsedMs,
                    checkpointRef = checkpointRef
                )
                currentSlice = currentSlice.copy(
                    status = SliceStatus.COMPLETED,
                    checkpointRef = checkpointRef,
                    finishedAt = clock.now()
                )
                currentTask.copy(
                    status = TaskStatus.COMPLETED,
                    latestResult = finalResult,
                    latestSnapshotRef = checkpointRef,
                    consumedCost = consumedCostAfterSlice,
                    updatedAt = clock.now()
                )
            } else {
                currentSlice = currentSlice.copy(
                    status = SliceStatus.SUSPENDED,
                    checkpointRef = checkpointRef,
                    finishedAt = clock.now()
                )
                currentTask.copy(
                    status = TaskStatus.SUSPENDED,
                    latestSnapshotRef = checkpointRef ?: currentTask.latestSnapshotRef,
                    consumedCost = consumedCostAfterSlice,
                    updatedAt = clock.now()
                )
            }
            taskStatePort.updateSlice(currentSlice)
            taskStatePort.upsertTask(updatedTask)
            if (updatedTask.status == TaskStatus.COMPLETED) {
                clearLearningState(updatedTask.taskId.value)
            }
            if (updatedTask.status == TaskStatus.COMPLETED) {
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-end",
                    taskStatus = updatedTask.status,
                    tenantId = acceptedTask.tenantId,
                    idempotencySuffix = "end-completed"
                )
            } else {
                publishSliceLifecycle(
                    slice = currentSlice,
                    action = "slice-suspend",
                    taskStatus = updatedTask.status,
                    tenantId = acceptedTask.tenantId,
                    idempotencySuffix = "suspend"
                )
            }
            publishEvent(
                topic = if (updatedTask.status == TaskStatus.COMPLETED) EventTopics.TASK_RESULT else EventTopics.SLICE_LIFECYCLE,
                key = updatedTask.taskId,
                payload = updatedTask.summaryPayload(),
                tenantId = acceptedTask.tenantId,
                idempotencyKey = if (updatedTask.status == TaskStatus.COMPLETED) {
                    "task:${updatedTask.taskId}:completed"
                } else {
                    "slice:${currentSlice.sliceId}:suspended"
                }
            )
            updatedTask
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
                idempotencySuffix = "end-failed"
            )
            val reasonCode = if (e is RemoteSolverException) {
                e.code
            } else {
                RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED
            }
            markFailed(
                currentTask,
                e.message ?: "Unknown failure",
                reasonCode
            )
        } finally {
            runningHandleByTaskId.remove(acceptedTask.taskId.value)
            nodeStatePort.releaseUnit(selectedNode.nodeId)
        }
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

    private suspend fun checkBudget(task: TaskState): Boolean {
        val limit = task.budgetLimit ?: return true
        if (task.consumedCost.toDouble() >= limit.toDouble()) {
            return false
        }
        val snapshot = budgetPort.snapshot(task.budgetScope.value) ?: return true
        return snapshot.remaining > 0
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
        val snapshot = budgetPort.snapshot(task.budgetScope.value)
        val budgetLimit = task.budgetLimit?.toDouble() ?: Double.MAX_VALUE
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
            .filter { it.complexity == TaskComplexity.COMPLEX }
            .map { max(0L, now - it.createdAt.toEpochMilliseconds()).toDouble() }
            .toList()
        val maxAge = complexAges.maxOrNull() ?: 1.0

        return tasks.sortedWith(
            compareByDescending<TaskState> {
                if (it.complexity == TaskComplexity.COMPLEX) {
                    complexRoundRobinPriority(it, now, maxAge)
                } else {
                    simplePriorityScore(it)
                }
            }.thenBy { it.createdAt }
        )
    }

    private fun simplePriorityScore(task: TaskState): Double {
        val realtimeBonus = if (task.timeSensitivity == TimeSensitivity.REALTIME) 1.0 else 0.0
        return task.priority.toDouble() * 10.0 + realtimeBonus
    }

    private fun complexRoundRobinPriority(task: TaskState, now: Long, maxAge: Double): Double {
        val runtime = runtimeSchedulerConfigRef.get()
        val urgency = estimateUrgency(task, now)
        val waitingAge = max(0L, now - task.createdAt.toEpochMilliseconds()).toDouble() / max(1.0, maxAge)
        val progressNeed = estimateProgressNeed(task)
        val costSensitivity = estimateCostSensitivity(task)
        val priorityBoost = task.priority.toDouble() / 100.0
        return runtime.complexUrgencyWeight * urgency +
            runtime.complexWaitingAgeWeight * waitingAge +
            runtime.complexProgressNeedWeight * progressNeed -
            runtime.complexCostSensitivityWeight * costSensitivity +
            priorityBoost
    }

    private fun estimateUrgency(task: TaskState, now: Long): Double {
        val deadlineInstant = task.deadline ?: return 0.0
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
        return if (task.latestSnapshotRef == null) {
            1.0
        } else {
            0.5
        }
    }

    private fun estimateCostSensitivity(task: TaskState): Double {
        val budgetLimit = task.budgetLimit ?: return 0.0
        val limit = budgetLimit.toDouble()
        if (limit <= 0.0) {
            return 1.0
        }
        return (task.consumedCost.toDouble() / limit).coerceIn(0.0, 1.0)
    }

    private fun resolveHardTimeoutEpochMs(task: TaskState): Long? =
        task.payload.taskMeta.timeLimitMs
            ?.takeIf { it > 0L }
            ?.let { task.createdAt.plus(it.toDuration(DurationUnit.MILLISECONDS)).toEpochMilliseconds() }

    private fun hardTimeoutReason(task: TaskState, nowEpochMs: Long = clock.nowEpochMs()): String? {
        val timeoutEpochMs = resolveHardTimeoutEpochMs(task) ?: return null
        if (nowEpochMs <= timeoutEpochMs) {
            return null
        }
        return "Hard timeout exceeded: now=$nowEpochMs, timeoutAt=$timeoutEpochMs"
    }

    private fun sliceTimeoutReason(sliceResult: SliceResult, quantumMs: Long): String? {
        val allowedMs = max(0L, quantumMs) + max(0L, config.sliceTimeoutGraceMs)
        if (sliceResult.elapsedMs <= allowedMs) {
            return null
        }
        return "Slice timeout exceeded: elapsedMs=${sliceResult.elapsedMs}, allowedMs=$allowedMs"
    }

    private fun computeQuantumMs(task: TaskState, node: NodeState): Long {
        val runtime = runtimeSchedulerConfigRef.get()
        if (task.complexity != TaskComplexity.COMPLEX) {
            return runtime.simpleTaskQuantumMs
        }

        val performance = max(0.1, node.profile.performanceScore.toDouble())
        val solveEstimateMs = runtime.complexSolveEstimateMs.toDouble() / performance
        val checkpointEstimateMs = if (node.profile.supportsCheckpoint) {
            runtime.complexCheckpointEstimateMs.toDouble()
        } else {
            0.0
        }
        val baselineQuantum = runtime.complexQuantumAlpha * solveEstimateMs -
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
        return clamped.toLong()
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

    private fun parseDoubleConfigValue(key: String, raw: String, minValue: Double): Double {
        val parsed = raw.trim().toDoubleOrNull()
            ?: throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Invalid double value for '$key': '$raw'"
            )
        return parsed.coerceAtLeast(minValue)
    }

    private suspend fun markFailed(
        task: TaskState,
        reason: String,
        reasonCode: RemoteSolverErrorCode = RemoteSolverErrorCode.TASK_FAILED
    ): TaskState {
        val normalizedReasonCode = RemoteSolverErrorMapper.reasonCodeOf(reasonCode)
        val failed = task.copy(
            status = TaskStatus.FAILED,
            latestResult = SolveResult(
                feasible = false,
                optimal = false,
                objectiveValue = task.latestResult?.objectiveValue?.toDouble(),
                gap = task.latestResult?.gap?.toDouble(),
                elapsedMs = task.latestResult?.elapsedMs ?: 0L,
                checkpointRef = task.latestSnapshotRef,
                message = reason,
                extension = mapOf("reasonCode" to normalizedReasonCode)
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
            for (slice in slices) {
                if (slice.nodeId?.value != nodeId) {
                    continue
                }
                if (slice.status !in setOf(SliceStatus.PLANNED, SliceStatus.RUNNING, SliceStatus.CHECKPOINTING)) {
                    continue
                }
                taskStatePort.updateSlice(
                    slice.copy(
                        status = SliceStatus.FAILED,
                        finishedAt = kotlin.time.Instant.fromEpochMilliseconds(now),
                        error = "Node heartbeat timeout"
                    )
                )
            }

            val nextStatus = if (task.latestSnapshotRef != null) TaskStatus.SUSPENDED else TaskStatus.QUEUED
            taskStatePort.upsertTask(
                task.copy(
                    status = nextStatus,
                    assignedNodeId = null,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(now)
                )
            )
            metricsPort.increment("task.recovered", tags = mapOf("reason" to "node_timeout"))
            publishEvent(
                topic = EventTopics.SLICE_LIFECYCLE,
                key = task.taskId,
                payload = SliceLifecyclePayload(
                    taskId = task.taskId.value,
                    sliceId = "unknown",
                    dispatchId = "unknown",
                    action = "slice-end",
                    status = "FAILED",
                    taskStatus = nextStatus.name,
                    reason = "node_timeout"
                ).toByteArray(),
                tenantId = task.tenantId,
                idempotencyKey = "task:${task.taskId}:recovered:node_timeout"
            )
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
            budgetScope = task.budgetScope.value,
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
        val budgetLimit = task.budgetLimit ?: return false
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
            return (1.0 + (1.0 - normalizedGap) * 0.2).coerceIn(0.5, 2.0)
        }
        val improvement = (previous - normalizedGap).coerceAtLeast(0.0)
        val regression = (normalizedGap - previous).coerceAtLeast(0.0)
        return (1.0 + improvement * 2.0 - regression).coerceIn(0.5, 2.0)
    }

    private fun clearLearningState(taskId: String) {
        latestGapByTaskId.remove(taskId)
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

    private suspend fun publishSliceLifecycle(
        slice: SliceState,
        action: String,
        taskStatus: TaskStatus,
        tenantId: Any,
        reason: String? = null,
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
            reason = reason
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

    private fun isNodeCompatible(task: TaskState, node: NodeState): Boolean {
        val requiredSolverType = task.payload.taskMeta.solverType?.value
            ?: task.payload.extension["solverType"]
            ?: task.payload.taskMeta.metadata["solverType"]
            ?: return true
        return node.profile.solverType.value.equals(requiredSolverType, ignoreCase = true)
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
        val budgetLimit = task.budgetLimit ?: return false
        if (task.consumedCost.toDouble() >= budgetLimit.toDouble()) {
            return true
        }
        val snapshot = budgetPort.snapshot(task.budgetScope.value) ?: return false
        return snapshot.remaining <= 0.0
    }

    private suspend fun isBudgetAdmissionBlocked(task: TaskState, compatibleNodes: List<NodeState>): Boolean {
        val budgetLimit = task.budgetLimit ?: return false
        if (compatibleNodes.isEmpty()) {
            return false
        }
        val remaining = budgetPort.snapshot(task.budgetScope.value)?.remaining
            ?: (budgetLimit.toDouble() - task.consumedCost.toDouble())
        val minEstimatedSliceCost = compatibleNodes.minOfOrNull { node ->
            estimateSliceCost(node, computeQuantumMs(task, node))
        } ?: return false
        return remaining < minEstimatedSliceCost
    }
}
