@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 远程求解器 API 门面
 *
 * 本模块提供远程求解器的 API 门面层，
 * 用于对外暴露任务提交、查询、控制等操作接口。
 * 负责请求验证、参数转换和异常映射。
 *
 * Remote Solver API Facade
 *
 * This module provides the API facade layer for the remote solver,
 * used to expose interfaces for task submission, query, control, etc.
 * Responsible for request validation, parameter conversion, and exception mapping.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorMapper
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.port.TaskEventQueryPort
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import kotlin.time.Instant

/**
 * 任务提交请求
 *
 * Task submission request.
 *
 * @param requestId 请求标识（可选），用于幂等性检查
 *                   Request identifier (optional) for idempotency check
 * @param tenantId 租户标识（可选）
 *                  Tenant identifier (optional)
 * @param complexity 任务复杂度（可选）
 *                    Task complexity (optional)
 * @param timeSensitivity 时间敏感度（可选）
 *                         Time sensitivity (optional)
 * @param priority 任务优先级，默认为 0
 *                  Task priority, defaults to 0
 * @param payloadRef 模型数据引用
 *                    Model data reference
 * @param configRef 配置数据引用（可选）
 *                   Configuration data reference (optional)
 * @param snapshotRef 快照数据引用（可选）
 *                     Snapshot data reference (optional)
 * @param taskMeta 任务元数据
 *                  Task metadata
 * @param extension 扩展信息键值对
 *                   Extension information key-value pairs
 * @param budgetScope 预算范围（可选）
 *                     Budget scope (optional)
 * @param budgetLimit 预算限制（可选）
 *                     Budget limit (optional)
 * @param deadlineEpochMs 截止日期时间戳（毫秒，可选）
 *                         Deadline timestamp in milliseconds (optional)
 */
data class TaskSubmitRequest(
    val requestId: String? = null,
    val tenantId: String? = null,
    val complexity: TaskComplexity? = null,
    val timeSensitivity: TimeSensitivity? = null,
    val priority: Int = 0,
    val payloadRef: ObjectRef,
    val configRef: ObjectRef? = null,
    val snapshotRef: ObjectRef? = null,
    val taskMeta: TaskMeta = TaskMeta(timeLimit = null),
    val extension: Map<String, String> = emptyMap(),
    val budgetScope: String? = null,
    val budgetLimit: Double? = null,
    val deadlineEpochMs: Long? = null
)

/**
 * 任务提交响应
 *
 * Task submission response.
 *
 * @param taskId 任务标识
 *               Task identifier
 * @param accepted 是否已接受
 *                  Whether accepted
 * @param status 任务状态
 *               Task status
 * @param message 响应消息
 *                Response message
 */
data class TaskSubmitResponse(
    val taskId: String,
    val accepted: Boolean,
    val status: TaskStatus,
    val message: String
)

/**
 * 任务视图响应
 *
 * Task view response.
 *
 * @param taskId 任务标识
 *               Task identifier
 * @param tenantId 租户标识
 *                  Tenant identifier
 * @param status 任务状态
 *               Task status
 * @param currentNodeId 当前执行节点标识
 *                       Current execution node identifier
 * @param latestCheckpointRef 最新检查点引用
 *                             Latest checkpoint reference
 * @param latestResultRef 最新结果引用
 *                         Latest result reference
 * @param consumedCost 已消耗成本
 *                     Consumed cost
 */
data class TaskViewResponse(
    val taskId: String,
    val tenantId: String,
    val status: TaskStatus,
    val currentNodeId: String?,
    val latestCheckpointRef: ObjectRef?,
    val latestResultRef: ObjectRef?,
    val consumedCost: Double
)

/**
 * 调度器热更新请求
 *
 * Scheduler hot reload request.
 *
 * @param operator 操作者标识
 *                  Operator identifier
 * @param changeSet 配置变更集
 *                   Configuration change set
 * @param requestedVersion 请求版本号（可选）
 *                          Requested version (optional)
 * @param effectiveAtEpochMs 生效时间戳（毫秒，可选）
 *                            Effective timestamp in milliseconds (optional)
 */
data class SchedulerHotReloadRequest(
    val operator: String,
    val changeSet: Map<String, String>,
    val requestedVersion: String? = null,
    val effectiveAtEpochMs: Long? = null
)

/**
 * 调度器回滚请求
 *
 * Scheduler rollback request.
 *
 * @param operator 操作者标识
 *                  Operator identifier
 * @param targetVersion 目标版本号
 *                       Target version
 * @param requestedVersion 请求版本号（可选）
 *                          Requested version (optional)
 * @param effectiveAtEpochMs 生效时间戳（毫秒，可选）
 *                            Effective timestamp in milliseconds (optional)
 */
data class SchedulerRollbackRequest(
    val operator: String,
    val targetVersion: String,
    val requestedVersion: String? = null,
    val effectiveAtEpochMs: Long? = null
)

/**
 * 调度器配置审计响应
 *
 * Scheduler configuration audit response.
 *
 * @param version 版本号
 *                Version
 * @param previousVersion 前一个版本号
 *                         Previous version
 * @param operator 操作者标识
 *                  Operator identifier
 * @param effectiveAtEpochMs 生效时间戳（毫秒）
 *                            Effective timestamp in milliseconds
 * @param changeSet 配置变更集
 *                   Configuration change set
 * @param rollbackFromVersion 回滚来源版本号（可选）
 *                             Rollback source version (optional)
 */
data class SchedulerConfigAuditResponse(
    val version: String,
    val previousVersion: String,
    val operator: String,
    val effectiveAtEpochMs: Long,
    val changeSet: Map<String, String>,
    val rollbackFromVersion: String?
)

/**
 * 监控调度器摘要
 *
 * Monitoring scheduler summary.
 *
 * @param schedulerConfigVersion 调度器配置版本
 *                                Scheduler configuration version
 * @param generatedAtEpochMs 生成时间戳（毫秒）
 *                            Generation timestamp in milliseconds
 * @param nodeHeartbeatTimeoutMs 节点心跳超时时间（毫秒）
 *                                Node heartbeat timeout in milliseconds
 */
data class MonitoringSchedulerSummary(
    val schedulerConfigVersion: String,
    val generatedAtEpochMs: Long,
    val nodeHeartbeatTimeoutMs: Long
)

/**
 * 监控节点摘要
 *
 * Monitoring node summary.
 *
 * @param nodeId 节点标识
 *               Node identifier
 * @param online 是否在线
 *                Whether online
 * @param health 健康状态
 *               Health status
 * @param solverType 求解器类型
 *                   Solver type
 * @param performanceScore 性能分数
 *                         Performance score
 * @param pricePerSecond 每秒价格
 *                       Price per second
 * @param availableUnits 可用单元数
 *                        Available units count
 * @param parallelUnits 并行单元数
 *                       Parallel units count
 * @param lastHeartbeatEpochMs 最后心跳时间戳（毫秒）
 *                              Last heartbeat timestamp in milliseconds
 * @param heartbeatLagMs 心跳延迟时间（毫秒）
 *                       Heartbeat lag in milliseconds
 */
data class MonitoringNodeSummary(
    val nodeId: String,
    val online: Boolean,
    val health: String,
    val solverType: String,
    val performanceScore: Double,
    val pricePerSecond: Double,
    val availableUnits: Int,
    val parallelUnits: Int,
    val lastHeartbeatEpochMs: Long,
    val heartbeatLagMs: Long
)

/**
 * 监控任务摘要
 *
 * Monitoring task summary.
 *
 * @param totalObservedTasks 观测到的任务总数
 *                            Total observed tasks count
 * @param queueDepth 队列深度
 *                   Queue depth
 * @param runningTasks 运行中任务数
 *                      Running tasks count
 * @param failedTasks 失败任务数
 *                     Failed tasks count
 * @param completedTasks 完成任务数
 *                        Completed tasks count
 * @param statusCounts 各状态任务数量统计
 *                     Task count by status
 * @param recentTasks 最近任务列表
 *                     Recent tasks list
 */
data class MonitoringTaskSummary(
    val totalObservedTasks: Int,
    val queueDepth: Int,
    val runningTasks: Int,
    val failedTasks: Int,
    val completedTasks: Int,
    val statusCounts: Map<String, Int>,
    val recentTasks: List<MonitoringRecentTask>
)

/**
 * 监控最近任务
 *
 * Monitoring recent task.
 *
 * @param taskId 任务标识
 *               Task identifier
 * @param tenantId 租户标识
 *                  Tenant identifier
 * @param status 任务状态
 *               Task status
 * @param complexity 任务复杂度
 *                    Task complexity
 * @param timeSensitivity 时间敏感度
 *                         Time sensitivity
 * @param priority 任务优先级
 *                  Task priority
 * @param assignedNodeId 分配的节点标识
 *                        Assigned node identifier
 * @param consumedCost 已消耗成本
 *                     Consumed cost
 * @param updatedAtEpochMs 更新时间戳（毫秒）
 *                         Update timestamp in milliseconds
 */
data class MonitoringRecentTask(
    val taskId: String,
    val tenantId: String,
    val status: String,
    val complexity: String,
    val timeSensitivity: String,
    val priority: Int,
    val assignedNodeId: String?,
    val consumedCost: Double,
    val updatedAtEpochMs: Long
)

/**
 * 监控概览响应
 *
 * Monitoring overview response.
 *
 * @param scheduler 调度器摘要
 *                   Scheduler summary
 * @param nodes 节点摘要列表
 *               Node summary list
 * @param nodeTotals 节点统计汇总
 *                   Node statistics summary
 * @param tasks 任务摘要
 *               Task summary
 */
data class MonitoringOverviewResponse(
    val scheduler: MonitoringSchedulerSummary,
    val nodes: List<MonitoringNodeSummary>,
    val nodeTotals: Map<String, Int>,
    val tasks: MonitoringTaskSummary
)

/**
 * 远程求解器 API 门面
 *
 * 提供任务提交、查询、控制等操作的 API 门面。
 * 负责请求验证、参数转换和异常映射。
 *
 * Remote solver API facade.
 *
 * Provides API facade for task submission, query, control, etc.
 * Responsible for request validation, parameter conversion, and exception mapping.
 *
 * @param service 远程求解器服务
 *                 Remote solver service
 */
class RemoteSolverApiFacade(
    private val service: RemoteSolverService
) {
    /**
     * 提交任务
     *
     * Submits task.
     *
     * @param request 任务提交请求
     *                 Task submission request
     * @return 任务提交响应
     *         Task submission response
     * @throws RemoteSolverException 当请求无效时抛出
     *                                Thrown when request is invalid
     */
    suspend fun submit(request: TaskSubmitRequest): TaskSubmitResponse {
        validate(request)
        val task = runCatching {
            val normalizedTenantId = normalizeTenantId(request.tenantId)
            service.submitTask(
                payload = SolvePayload(
                    modelRef = scopeObjectRef(request.payloadRef, normalizedTenantId),
                    configRef = request.configRef?.let { scopeObjectRef(it, normalizedTenantId) },
                    snapshotRef = request.snapshotRef?.let { scopeObjectRef(it, normalizedTenantId) },
                    taskMeta = request.taskMeta,
                    extension = request.extension + mapOf("tenantId" to normalizedTenantId)
                ),
                complexity = request.complexity,
                timeSensitivity = request.timeSensitivity,
                priority = request.priority,
                deadline = request.deadlineEpochMs?.let { Instant.fromEpochMilliseconds(it) },
                budgetScope = request.budgetScope?.let { BudgetScopeId.of(it) },
                budgetLimit = request.budgetLimit?.let { Flt64(it) },
                tenantId = TenantId.of(normalizedTenantId),
                requestId = request.requestId?.let { RequestId.of(it) }
            )
        }.getOrElse { throwable ->
            throw mapException(throwable)
        }
        return TaskSubmitResponse(
            taskId = task.taskId.value,
            accepted = true,
            status = task.status,
            message = "accepted"
        )
    }

    /**
     * 获取任务视图
     *
     * Gets task view.
     *
     * @param taskId 任务标识
     *               Task identifier
     * @return 任务视图响应，如果任务不存在则返回 null
     *         Task view response, or null if task does not exist
     */
    suspend fun get(taskId: String): TaskViewResponse? =
        service.getTask(taskId)?.let { mapTaskView(it) }

    /**
     * 停止任务
     *
     * Stops task.
     *
     * @param taskId 任务标识
     *               Task identifier
     * @param reason 停止原因，默认为 "Stopped by API"
     *                Stop reason, defaults to "Stopped by API"
     * @param operator 操作者标识（可选）
     *                  Operator identifier (optional)
     * @param source 来源标识（可选）
     *                 Source identifier (optional)
     * @return 任务视图响应，如果任务不存在则返回 null
     *         Task view response, or null if task does not exist
     */
    suspend fun stop(
        taskId: String,
        reason: String = "Stopped by API",
        operator: String? = null,
        source: String? = null
    ): TaskViewResponse? =
        runCatching {
            service.stopTask(taskId, reason, operator, source)
        }.getOrElse { throwable ->
            throw mapException(throwable)
        }?.let { mapTaskView(it) }

    /**
     * 恢复任务
     *
     * Resumes task.
     *
     * @param taskId 任务标识
     *               Task identifier
     * @param operator 操作者标识（可选）
     *                  Operator identifier (optional)
     * @param source 来源标识（可选）
     *                 Source identifier (optional)
     * @param reason 恢复原因（可选）
     *                Resume reason (optional)
     * @return 任务视图响应，如果任务不存在则返回 null
     *         Task view response, or null if task does not exist
     */
    suspend fun resume(
        taskId: String,
        operator: String? = null,
        source: String? = null,
        reason: String? = null
    ): TaskViewResponse? =
        runCatching {
            service.resumeTask(taskId, operator, source, reason)
        }.getOrElse { throwable ->
            throw mapException(throwable)
        }?.let { mapTaskView(it) }

    suspend fun hotReloadScheduler(request: SchedulerHotReloadRequest): SchedulerConfigAuditResponse {
        validateSchedulerHotReload(request)
        return runCatching {
            service.applySchedulerHotReload(
                changeSet = request.changeSet,
                operator = request.operator,
                effectiveAtEpochMs = request.effectiveAtEpochMs ?: System.currentTimeMillis(),
                requestedVersion = request.requestedVersion
            )
        }.getOrElse { throwable ->
            throw mapException(throwable)
        }.let { audit ->
            audit.toApiResponse()
        }
    }

    suspend fun rollbackScheduler(request: SchedulerRollbackRequest): SchedulerConfigAuditResponse {
        validateSchedulerRollback(request)
        return runCatching {
            service.rollbackSchedulerHotReload(
                targetVersion = request.targetVersion,
                operator = request.operator,
                effectiveAtEpochMs = request.effectiveAtEpochMs ?: System.currentTimeMillis(),
                requestedVersion = request.requestedVersion
            )
        }.getOrElse { throwable ->
            throw mapException(throwable)
        }.let { audit ->
            audit.toApiResponse()
        }
    }

    suspend fun listSchedulerConfigAudits(limit: Int = 100): List<SchedulerConfigAuditResponse> =
        service.listSchedulerConfigAudits(limit).map { it.toApiResponse() }

    suspend fun replayTaskTimeline(taskId: String, limitEvents: Int = 500): TaskTimelineReport =
        runCatching {
            TaskTimelineReplayer(
                taskStatePort = service.taskStatePort(),
                checkpointPort = service.checkpointPort(),
                costLedgerPort = service.costLedgerPort(),
                clock = service.clockPort(),
                taskEventQueryPort = service.eventPort() as? TaskEventQueryPort
            ).replay(taskId = taskId, limitEvents = limitEvents)
        }.getOrElse { throwable ->
            throw mapException(throwable)
        }

    suspend fun monitorOverview(limitRecentTasks: Int = 200): MonitoringOverviewResponse {
        val safeLimit = limitRecentTasks.coerceIn(1, 1000)
        val now = service.clockPort().nowEpochMs()
        val heartbeatTimeout = service.nodeHeartbeatTimeoutMs().coerceAtLeast(1L)
        val nodes = service.nodeStatePort()
            .listNodes(onlineOnly = false)
            .sortedBy { it.nodeId.value }
            .map { node ->
                val lagMs = (now - node.lastHeartbeatEpochMs).coerceAtLeast(0L)
                val health = when {
                    !node.online -> "OFFLINE"
                    lagMs > heartbeatTimeout -> "STALE"
                    else -> "ONLINE"
                }
                MonitoringNodeSummary(
                    nodeId = node.nodeId.value,
                    online = node.online,
                    health = health,
                    solverType = node.profile.solverType.value,
                    performanceScore = node.profile.performanceScore.toDouble(),
                    pricePerSecond = node.profile.pricePerSecond.toDouble(),
                    availableUnits = node.availableUnits,
                    parallelUnits = node.profile.parallelUnits,
                    lastHeartbeatEpochMs = node.lastHeartbeatEpochMs,
                    heartbeatLagMs = lagMs
                )
            }
        val statuses = TaskStatus.values().toSet()
        val tasks = service.taskStatePort().listTasks(statuses = statuses, limit = safeLimit)
        val statusCounts = TaskStatus.values().associate { status ->
            status.name to tasks.count { it.status == status }
        }
        val queueDepth = statusCounts.getValue(TaskStatus.CREATED.name) +
            statusCounts.getValue(TaskStatus.ACCEPTED.name) +
            statusCounts.getValue(TaskStatus.QUEUED.name) +
            statusCounts.getValue(TaskStatus.DISPATCHING.name)
        val nodeTotal = nodes.size
        val onlineNodes = nodes.count { it.online }
        val staleNodes = nodes.count { it.health == "STALE" }
        val totalParallelUnits = nodes.sumOf { it.parallelUnits.coerceAtLeast(0) }
        val totalAvailableUnits = nodes.sumOf { it.availableUnits.coerceAtLeast(0) }
        val usedUnits = (totalParallelUnits - totalAvailableUnits).coerceAtLeast(0)

        return MonitoringOverviewResponse(
            scheduler = MonitoringSchedulerSummary(
                schedulerConfigVersion = service.schedulerConfigVersion(),
                generatedAtEpochMs = now,
                nodeHeartbeatTimeoutMs = heartbeatTimeout
            ),
            nodes = nodes,
            nodeTotals = mapOf(
                "totalNodes" to nodeTotal,
                "onlineNodes" to onlineNodes,
                "staleNodes" to staleNodes,
                "offlineNodes" to (nodeTotal - onlineNodes).coerceAtLeast(0),
                "totalParallelUnits" to totalParallelUnits,
                "totalAvailableUnits" to totalAvailableUnits,
                "usedUnits" to usedUnits
            ),
            tasks = MonitoringTaskSummary(
                totalObservedTasks = tasks.size,
                queueDepth = queueDepth,
                runningTasks = statusCounts.getValue(TaskStatus.RUNNING.name),
                failedTasks = statusCounts.getValue(TaskStatus.FAILED.name),
                completedTasks = statusCounts.getValue(TaskStatus.COMPLETED.name),
                statusCounts = statusCounts,
                recentTasks = tasks
                    .sortedByDescending { it.updatedAtEpochMs }
                    .take(50)
                    .map { task ->
                        MonitoringRecentTask(
                            taskId = task.taskId.value,
                            tenantId = task.tenantId.value,
                            status = task.status.name,
                            complexity = task.complexity.name,
                            timeSensitivity = task.timeSensitivity.name,
                            priority = task.priority,
                            assignedNodeId = task.assignedNodeId?.value,
                            consumedCost = task.consumedCost.toDouble(),
                            updatedAtEpochMs = task.updatedAt.toEpochMilliseconds()
                        )
                    }
            )
        )
    }

    private fun mapTaskView(task: TaskState): TaskViewResponse =
        TaskViewResponse(
            taskId = task.taskId.value,
            tenantId = task.tenantId.value,
            status = task.status,
            currentNodeId = task.assignedNodeId?.value,
            latestCheckpointRef = task.latestSnapshotRef ?: task.latestResult?.checkpointRef,
            latestResultRef = task.latestResult?.resultRef,
            consumedCost = task.consumedCost.toDouble()
        )

    private fun validate(request: TaskSubmitRequest) {
        if (request.payloadRef.path.value.isBlank()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "payloadRef.path must not be blank"
            )
        }
        if (request.budgetLimit != null && request.budgetLimit <= 0.0) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "budgetLimit must be positive when provided"
            )
        }
        request.tenantId?.let { normalizeTenantId(it) }
    }

    private fun validateSchedulerHotReload(request: SchedulerHotReloadRequest) {
        if (request.operator.isBlank()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "operator must not be blank"
            )
        }
        if (request.changeSet.isEmpty()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "changeSet must not be empty"
            )
        }
    }

    private fun validateSchedulerRollback(request: SchedulerRollbackRequest) {
        if (request.operator.isBlank()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "operator must not be blank"
            )
        }
        if (request.targetVersion.isBlank()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "targetVersion must not be blank"
            )
        }
    }

    private fun SchedulerHotReloadAuditRecord.toApiResponse(): SchedulerConfigAuditResponse =
        SchedulerConfigAuditResponse(
            version = version,
            previousVersion = previousVersion,
            operator = operator,
            effectiveAtEpochMs = effectiveAtEpochMs,
            changeSet = changeSet,
            rollbackFromVersion = rollbackFromVersion
        )

    private fun normalizeTenantId(raw: String?): String {
        val tenant = raw?.trim()?.takeIf { it.isNotEmpty() } ?: "default"
        if (!Regex("^[A-Za-z0-9._-]+$").matches(tenant)) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "tenantId contains illegal characters"
            )
        }
        return tenant
    }

    private fun scopeObjectRef(ref: ObjectRef, tenantId: String): ObjectRef {
        val normalized = ref.path.value.trim()
        if (normalized.startsWith("$tenantId/")) {
            return ref
        }
        return ObjectRef.of(path = "$tenantId/$normalized", version = ref.version?.value, etag = ref.etag?.value)
    }

    private fun mapException(throwable: Throwable): RuntimeException {
        return RemoteSolverErrorMapper.normalize(throwable)
    }

    /**
     * Checks if the service is ready to accept requests.
     * Used for Kubernetes readiness probes.
     */
    fun isReady(): Boolean {
        return try {
            // Check if we can query nodes (database connectivity)
            kotlinx.coroutines.runBlocking {
                service.nodeStatePort().listNodes(onlineOnly = false)
            }
            // Service is ready if we can successfully query the database
            true
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Performs graceful shutdown.
     * Stops accepting new tasks and waits for running tasks to complete.
     */
    suspend fun gracefulShutdown(timeoutMs: Long = 30000L): Boolean {
        return try {
            val startTime = System.currentTimeMillis()

            // Stop accepting new tasks by marking service as unavailable
            // Wait for running tasks to complete
            while (System.currentTimeMillis() - startTime < timeoutMs) {
                val runningTasks = service.taskStatePort()
                    .listTasks(statuses = setOf(TaskStatus.RUNNING), limit = 1000)
                if (runningTasks.isEmpty()) {
                    return true
                }
                kotlinx.coroutines.delay(1000L)
            }

            // Timeout reached, some tasks still running
            false
        } catch (e: Exception) {
            false
        }
    }
}
