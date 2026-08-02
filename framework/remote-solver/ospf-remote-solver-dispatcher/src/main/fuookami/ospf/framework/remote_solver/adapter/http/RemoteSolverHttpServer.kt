/*
 * 远程求解器 HTTP 服务器
 *
 * Remote Solver HTTP Server
 *
 * 该模块提供基于 Ktor CIO 的 HTTP API 服务实现，是远程求解器系统的对外接口层。
 * 提供任务提交、状态查询、控制操作、监控查询和指标暴露等 RESTful API。
 * This module provides Ktor CIO-based HTTP API service implementation,
 * which is the external interface layer of the remote solver system.
 * Provides RESTful APIs for task submission, status query, control operations,
 * monitoring query and metrics exposition.
 */

package fuookami.ospf.framework.remote_solver.adapter.http

import fuookami.ospf.framework.remote_solver.application.RemoteSolverApiFacade
import fuookami.ospf.framework.remote_solver.application.SchedulerHotReloadRequest
import fuookami.ospf.framework.remote_solver.application.SchedulerRollbackRequest
import fuookami.ospf.framework.remote_solver.application.TaskSubmitRequest
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorMapper
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import fuookami.ospf.framework.remote_solver.port.MetricsScrapePort
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.kotlinx.json.json
import io.ktor.server.application.Application
import io.ktor.server.application.call
import io.ktor.server.application.install
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.plugins.BadRequestException
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.plugins.statuspages.StatusPages
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.response.respondRedirect
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.routing
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.net.ServerSocket

/**
 * 远程求解器 HTTP 服务器
 *
 * Remote Solver HTTP Server
 *
 * 该类提供 HTTP API 服务，基于 Ktor CIO 引擎实现高性能异步 HTTP 服务。
 * 主要 API 端点包括：
 * This class provides HTTP API service, using Ktor CIO engine for high-performance
 * async HTTP service. Main API endpoints include:
 *
 * - **健康检查**: `/health`, `/health/ready`, `/health/live`
 *   Health check: `/health`, `/health/ready`, `/health/live`
 * - **任务管理**: `/api/v1/tasks` (提交), `/api/v1/tasks/{taskId}` (查询), `/api/v1/tasks/{taskId}/stop` (停止), `/api/v1/tasks/{taskId}/resume` (恢复)
 *   Task management: `/api/v1/tasks` (submit), `/api/v1/tasks/{taskId}` (query), `/api/v1/tasks/{taskId}/stop` (stop), `/api/v1/tasks/{taskId}/resume` (resume)
 * - **调度器配置**: `/api/v1/scheduler/config/hot-reload`, `/api/v1/scheduler/config/rollback`, `/api/v1/scheduler/config/audits`
 *   Scheduler config: `/api/v1/scheduler/config/hot-reload`, `/api/v1/scheduler/config/rollback`, `/api/v1/scheduler/config/audits`
 * - **时间线查询**: `/api/v1/tasks/{taskId}/timeline`
 *   Timeline query: `/api/v1/tasks/{taskId}/timeline`
 * - **监控接口**: `/api/v1/monitor/overview`, `/monitor`, `/metrics`
 *   Monitor: `/api/v1/monitor/overview`, `/monitor`, `/metrics`
 *
 * @param apiFacade 远程求解器 API 门面，提供业务逻辑接口。
 *                   Remote solver API facade, providing business logic interface.
 * @param metricsPort 指标端口，用于 Prometheus 指标暴露。
 *                     Metrics port, for Prometheus metrics exposition.
 * @param tenantAuthEnabled 是否启用租户认证，启用后请求需要携带 X-Tenant-Id 头。
 *                           Whether to enable tenant authentication, requires X-Tenant-Id header if enabled.
 * @param monitorAuthEnabled 是否启用监控认证，启用后监控接口需要认证。
 *                            Whether to enable monitor authentication, requires authentication for monitor endpoints if enabled.
 * @param monitorLoginUrl 监控登录页面 URL，用于未认证用户重定向。
 *                         Monitor login page URL, for redirecting unauthenticated users.
 * @param monitorAllowedRoles 允许访问监控的角色集合。
 *                             Roles set allowed to access monitor.
 * @param host 监听主机地址，默认为 "0.0.0.0"（所有接口）。
 *              Listen host address, defaults to "0.0.0.0" (all interfaces).
 * @param port 监听端口，默认为 18080；设置为 0 时自动选择可用端口。
 *              Listen port, defaults to 18080; auto-selects available port when set to 0.
 */
class RemoteSolverHttpServer(
    private val apiFacade: RemoteSolverApiFacade,
    private val metricsPort: MetricsPort? = null,
    private val tenantAuthEnabled: Boolean = false,
    private val monitorAuthEnabled: Boolean = true,
    private val monitorLoginUrl: String = "/login",
    private val monitorAllowedRoles: Set<String> = setOf("admin", "monitor_read"),
    host: String = "0.0.0.0",
    port: Int = 18080
) {
    private val configuredPort = if (port == 0) findAvailablePort() else port
    private val engine = embeddedServer(CIO, host = host, port = configuredPort) {
        module(
            apiFacade = apiFacade,
            metricsPort = metricsPort,
            tenantAuthEnabled = tenantAuthEnabled,
            monitorAuthEnabled = monitorAuthEnabled,
            monitorLoginUrl = monitorLoginUrl,
            monitorAllowedRoles = monitorAllowedRoles
        )
    }

    /**
     * 启动服务器
     *
     * Start server
     *
     * 启动 HTTP 服务器，不阻塞调用线程。服务器会在后台线程中运行。
     * Starts HTTP server, not blocking calling thread. Server runs in background thread.
     */
    fun start() {
        engine.start(wait = false)
    }

    /**
     * 停止服务器
     *
     * Stop server
     *
     * 优雅停止 HTTP 服务器，等待指定时间让正在处理的请求完成。
     * Gracefully stops HTTP server, waiting specified time for in-progress requests to complete.
     *
     * @param delaySeconds 延迟停止时间（秒），用于优雅关闭。
     *                      Delay stop time in seconds, for graceful shutdown.
     */
    fun stop(delaySeconds: Int = 0) {
        val delayMs = delaySeconds.toLong().coerceAtLeast(0L) * 1000L
        engine.stop(gracePeriodMillis = delayMs, timeoutMillis = delayMs + 1000L)
    }

    /**
     * 获取实际监听端口
     *
     * Get actual listening port
     *
     * 返回服务器实际使用的端口，当初始化时端口设置为 0 时，返回自动选择的可用端口。
     * Returns actual port used by server, when port was set to 0 during initialization,
     * returns auto-selected available port.
     *
     * @return 实际监听端口。
     *         Actual listening port.
     */
    fun port(): Int = configuredPort

    /**
     * 查找可用端口
     *
     * Find available port
     *
     * 通过绑定临时 socket 来获取系统分配的可用端口。
     * Gets system-allocated available port by binding temporary socket.
     *
     * @return 可用端口。
     *         Available port.
     */
    private fun findAvailablePort(): Int =
        ServerSocket(0).use { socket -> socket.localPort }
}

/**
 * Ktor Application 模块配置
 *
 * Ktor Application Module Configuration
 *
 * 配置 Ktor 应用模块，安装插件和定义路由规则。
 * Configures Ktor application module, installing plugins and defining routing rules.
 *
 * @param apiFacade API 门面实例。
 *                   API facade instance.
 * @param metricsPort 指标端口实例。
 *                     Metrics port instance.
 * @param tenantAuthEnabled 租户认证开关。
 *                           Tenant authentication switch.
 * @param monitorAuthEnabled 监控认证开关。
 *                            Monitor authentication switch.
 * @param monitorLoginUrl 监控登录 URL。
 *                         Monitor login URL.
 * @param monitorAllowedRoles 允许的监控角色。
 *                             Allowed monitor roles.
 */
private fun Application.module(
    apiFacade: RemoteSolverApiFacade,
    metricsPort: MetricsPort?,
    tenantAuthEnabled: Boolean,
    monitorAuthEnabled: Boolean,
    monitorLoginUrl: String,
    monitorAllowedRoles: Set<String>
) {
    val normalizedMonitorLoginUrl = monitorLoginUrl.trim().takeIf { it.isNotEmpty() } ?: "/login"
    val normalizedMonitorAllowedRoles = monitorAllowedRoles
        .asSequence()
        .map { it.trim().lowercase() }
        .filter { it.isNotEmpty() }
        .toSet()
        .ifEmpty { setOf("admin", "monitor_read") }

    // 安装 JSON 内容协商插件
    // Install JSON content negotiation plugin
    install(ContentNegotiation) {
        json(
            Json {
                ignoreUnknownKeys = true
                isLenient = true
            }
        )
    }

    // 安装异常处理插件
    // Install exception handling plugin
    install(StatusPages) {
        exception<RemoteSolverException> { call, e -> respondError(call, e) }
        exception<IllegalArgumentException> { call, e -> respondError(call, e) }
        exception<BadRequestException> { call, e -> respondError(call, e) }
        exception<Throwable> { call, e -> respondError(call, e) }
    }

    // 定义路由规则
    // Define routing rules
    routing {
        // ==================== 健康检查端点 ====================
        // ==================== Health check endpoints ====================

        /**
         * 基础健康检查
         *
         * Basic health check
         *
         * GET /health - 返回服务健康状态。
         * Returns service health status.
         */
        get("/health") {
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "healthy",
                    traceId = call.extractTraceId(),
                    data = HealthCheckResponse(
                        status = "UP",
                        timestampEpochMs = System.currentTimeMillis()
                    )
                )
            )
        }

        /**
         * 就绪检查
         *
         * Readiness check
         *
         * GET /health/ready - 返回服务是否已准备好接收请求。
         * Returns whether service is ready to accept requests.
         */
        get("/health/ready") {
            val isReady = apiFacade.isReady()
            if (isReady) {
                call.respond(
                    ApiEnvelope(
                        code = "OK",
                        message = "ready",
                        traceId = call.extractTraceId(),
                        data = HealthCheckResponse(
                            status = "UP",
                            timestampEpochMs = System.currentTimeMillis()
                        )
                    )
                )
            } else {
                call.respond(
                    HttpStatusCode.ServiceUnavailable,
                    ApiEnvelope<Unit>(
                        code = "NOT_READY",
                        message = "service is not ready",
                        traceId = call.extractTraceId(),
                        data = null
                    )
                )
            }
        }

        /**
         * 存活检查
         *
         * Liveness check
         *
         * GET /health/live - 返回服务是否存活。
         * Returns whether service is alive.
         */
        get("/health/live") {
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "alive",
                    traceId = call.extractTraceId(),
                    data = HealthCheckResponse(
                        status = "UP",
                        timestampEpochMs = System.currentTimeMillis()
                    )
                )
            )
        }

        /**
         * 能力与协议版本探测。
         *
         * Capability and protocol version probe.
         *
         * GET /api/v1/capabilities - 返回当前在线节点支持的模型类型和协议版本。
         * Returns model types and protocol versions supported by current online nodes.
         */
        get("/api/v1/capabilities") {
            val capabilities = apiFacade.capabilities()
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    traceId = call.extractTraceId(),
                    data = SolverCapabilitiesHttpResponse(
                        schemaVersion = capabilities.schemaVersion,
                        protocolVersions = capabilities.protocolVersions,
                        supportedModelTypes = capabilities.supportedModelTypes,
                        supportsPortableCheckpoint = capabilities.supportsPortableCheckpoint,
                        supportsNativeCheckpoint = capabilities.supportsNativeCheckpoint
                    )
                )
            )
        }

        // ==================== 任务管理端点 ====================
        // ==================== Task management endpoints ====================

        /**
         * 提交任务
         *
         * Submit task
         *
         * POST /api/v1/tasks - 提交新的求解任务。
         * Submits new solving task.
         */
        post("/api/v1/tasks") {
            val body = call.receive<SubmitTaskHttpRequest>()
            val headerTenantId = call.request.headers["X-Tenant-Id"]?.trim()?.takeIf { it.isNotEmpty() }
            if (tenantAuthEnabled && headerTenantId == null) {
                call.respond(
                    HttpStatusCode.Forbidden,
                    ApiEnvelope<Unit>(
                        code = "TENANT_AUTH_REQUIRED",
                        message = "X-Tenant-Id header is required",
                        traceId = call.extractTraceId(),
                        data = null
                    )
                )
                return@post
            }
            if (tenantAuthEnabled && body.tenantId != null && body.tenantId != headerTenantId) {
                call.respond(
                    HttpStatusCode.Forbidden,
                    ApiEnvelope<Unit>(
                        code = "TENANT_MISMATCH",
                        message = "tenantId in body does not match X-Tenant-Id",
                        traceId = call.extractTraceId(),
                        data = null
                    )
                )
                return@post
            }
            val payloadRef = body.payloadRef?.trim()
                ?: throw IllegalArgumentException("payloadRef is required")
            val request = TaskSubmitRequest(
                requestId = body.requestId,
                tenantId = if (tenantAuthEnabled) headerTenantId else body.tenantId,
                complexity = body.complexity?.let { TaskComplexity.valueOf(it) },
                timeSensitivity = body.timeSensitivity?.let { TimeSensitivity.valueOf(it) },
                priority = body.priority ?: 0,
                payloadRef = ObjectRef.of(path = payloadRef),
                budgetScope = body.budgetScope,
                budgetLimit = body.budgetLimit,
                deadlineEpochMs = body.deadlineEpochMs
            )
            val submitted = apiFacade.submit(request)
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    traceId = call.extractTraceId(),
                    data = SubmitTaskHttpResponse(
                        taskId = submitted.taskId,
                        accepted = submitted.accepted,
                        status = submitted.status.name,
                        message = submitted.message
                    )
                )
            )
        }

        /**
         * 查询任务状态
         *
         * Query task status
         *
         * GET /api/v1/tasks/{taskId} - 获取指定任务的状态信息。
         * Gets status information of specified task.
         */
        get("/api/v1/tasks/{taskId}") {
            val taskId = call.parameters["taskId"]?.trim()
            if (taskId.isNullOrBlank()) {
                throw IllegalArgumentException("taskId must not be blank")
            }
            val task = apiFacade.get(taskId)
            if (task == null) {
                call.respond(
                    HttpStatusCode.NotFound,
                    ApiEnvelope<Unit>(
                        code = "TASK_NOT_FOUND",
                        message = "Task not found",
                        traceId = call.extractTraceId(),
                        data = null
                    )
                )
                return@get
            }
            if (!call.verifyTenantAccess(task.tenantId, tenantAuthEnabled)) {
                return@get
            }
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    traceId = call.extractTraceId(),
                    data = TaskViewHttpResponse(
                        taskId = task.taskId,
                        tenantId = task.tenantId,
                        status = task.status.name,
                        currentNodeId = task.currentNodeId,
                        latestCheckpointPath = task.latestCheckpointRef?.path?.value,
                        latestResultPath = task.latestResultRef?.path?.value,
                        consumedCost = task.consumedCost
                    )
                )
            )
        }

        /**
         * 停止任务
         *
         * Stop task
         *
         * POST /api/v1/tasks/{taskId}/stop - 停止正在运行的求解任务。
         * Stops running solving task.
         */
        post("/api/v1/tasks/{taskId}/stop") {
            val taskId = call.parameters["taskId"]?.trim()
            if (taskId.isNullOrBlank()) {
                throw IllegalArgumentException("taskId must not be blank")
            }

            // Auth check: verify tenant access before executing stop
            // 认证检查：执行停止前验证租户访问权限
            if (tenantAuthEnabled) {
                val existingTask = apiFacade.get(taskId)
                if (existingTask == null) {
                    call.respond(
                        HttpStatusCode.NotFound,
                        ApiEnvelope<Unit>(
                            code = "TASK_NOT_FOUND",
                            message = "Task not found",
                            traceId = call.extractTraceId(),
                            data = null
                        )
                    )
                    return@post
                }
                val headerTenantId = call.request.headers["X-Tenant-Id"]?.trim()?.takeIf { it.isNotEmpty() }
                if (headerTenantId == null) {
                    call.respond(
                        HttpStatusCode.Forbidden,
                        ApiEnvelope<Unit>(
                            code = "TENANT_AUTH_REQUIRED",
                            message = "X-Tenant-Id header is required",
                            traceId = call.extractTraceId(),
                            data = null
                        )
                    )
                    return@post
                }
                if (headerTenantId != existingTask.tenantId) {
                    call.respond(
                        HttpStatusCode.Forbidden,
                        ApiEnvelope<Unit>(
                            code = "TENANT_ACCESS_DENIED",
                            message = "tenant is not allowed to access this task",
                            traceId = call.extractTraceId(),
                            data = null
                        )
                    )
                    return@post
                }
            }

            val body = runCatching { call.receive<StopTaskHttpRequest>() }.getOrNull()
            val reason = body?.reason?.takeIf { it.isNotBlank() } ?: "Stopped by API"
            val operator = body?.operator ?: call.request.headers["X-User-Id"]?.trim()?.takeIf { it.isNotEmpty() }
            val source = body?.source ?: "api"
            val task = apiFacade.stop(taskId, reason, operator, source)
            if (task == null) {
                call.respond(
                    HttpStatusCode.NotFound,
                    ApiEnvelope<Unit>(
                        code = "TASK_NOT_FOUND",
                        message = "Task not found",
                        traceId = call.extractTraceId(),
                        data = null
                    )
                )
                return@post
            }
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    traceId = call.extractTraceId(),
                    data = TaskActionHttpResponse(
                        taskId = task.taskId,
                        status = task.status.name
                    )
                )
            )
        }

        /**
         * 恢复任务
         *
         * Resume task
         *
         * POST /api/v1/tasks/{taskId}/resume - 恢复已暂停的求解任务。
         * Resumes paused solving task.
         */
        post("/api/v1/tasks/{taskId}/resume") {
            val taskId = call.parameters["taskId"]?.trim()
            if (taskId.isNullOrBlank()) {
                throw IllegalArgumentException("taskId must not be blank")
            }

            // Auth check: verify tenant access before executing resume
            // 认证检查：执行恢复前验证租户访问权限
            if (tenantAuthEnabled) {
                val existingTask = apiFacade.get(taskId)
                if (existingTask == null) {
                    call.respond(
                        HttpStatusCode.NotFound,
                        ApiEnvelope<Unit>(
                            code = "TASK_NOT_FOUND",
                            message = "Task not found",
                            traceId = call.extractTraceId(),
                            data = null
                        )
                    )
                    return@post
                }
                val headerTenantId = call.request.headers["X-Tenant-Id"]?.trim()?.takeIf { it.isNotEmpty() }
                if (headerTenantId == null) {
                    call.respond(
                        HttpStatusCode.Forbidden,
                        ApiEnvelope<Unit>(
                            code = "TENANT_AUTH_REQUIRED",
                            message = "X-Tenant-Id header is required",
                            traceId = call.extractTraceId(),
                            data = null
                        )
                    )
                    return@post
                }
                if (headerTenantId != existingTask.tenantId) {
                    call.respond(
                        HttpStatusCode.Forbidden,
                        ApiEnvelope<Unit>(
                            code = "TENANT_ACCESS_DENIED",
                            message = "tenant is not allowed to access this task",
                            traceId = call.extractTraceId(),
                            data = null
                        )
                    )
                    return@post
                }
            }

            val body = runCatching { call.receive<ResumeTaskHttpRequest>() }.getOrNull()
            val operator = body?.operator ?: call.request.headers["X-User-Id"]?.trim()?.takeIf { it.isNotEmpty() }
            val source = body?.source ?: "api"
            val reason = body?.reason
            val task = apiFacade.resume(taskId, operator, source, reason)
            if (task == null) {
                call.respond(
                    HttpStatusCode.NotFound,
                    ApiEnvelope<Unit>(
                        code = "TASK_NOT_FOUND",
                        message = "Task not found",
                        traceId = call.extractTraceId(),
                        data = null
                    )
                )
                return@post
            }
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    traceId = call.extractTraceId(),
                    data = TaskActionHttpResponse(
                        taskId = task.taskId,
                        status = task.status.name
                    )
                )
            )
        }

        // ==================== 调度器配置端点 ====================
        // ==================== Scheduler config endpoints ====================

        /**
         * 调度器配置热加载
         *
         * Scheduler config hot reload
         *
         * POST /api/v1/scheduler/config/hot-reload - 动态更新调度器配置。
         * Dynamically updates scheduler configuration.
         */
        post("/api/v1/scheduler/config/hot-reload") {
            val body = call.receive<SchedulerHotReloadHttpRequest>()
            val audit = apiFacade.hotReloadScheduler(
                SchedulerHotReloadRequest(
                    operator = body.operator ?: "",
                    changeSet = body.changeSet ?: emptyMap(),
                    requestedVersion = body.requestedVersion,
                    effectiveAtEpochMs = body.effectiveAtEpochMs
                )
            )
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    data = SchedulerConfigAuditHttpResponse(
                        version = audit.version,
                        previousVersion = audit.previousVersion,
                        operator = audit.operator,
                        effectiveAtEpochMs = audit.effectiveAtEpochMs,
                        changeSet = audit.changeSet,
                        rollbackFromVersion = audit.rollbackFromVersion
                    )
                )
            )
        }

        /**
         * 调度器配置回滚
         *
         * Scheduler config rollback
         *
         * POST /api/v1/scheduler/config/rollback - 回滚调度器配置到指定版本。
         * Rolls back scheduler configuration to specified version.
         */
        post("/api/v1/scheduler/config/rollback") {
            val body = call.receive<SchedulerRollbackHttpRequest>()
            val audit = apiFacade.rollbackScheduler(
                SchedulerRollbackRequest(
                    operator = body.operator ?: "",
                    targetVersion = body.targetVersion ?: "",
                    requestedVersion = body.requestedVersion,
                    effectiveAtEpochMs = body.effectiveAtEpochMs
                )
            )
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    data = SchedulerConfigAuditHttpResponse(
                        version = audit.version,
                        previousVersion = audit.previousVersion,
                        operator = audit.operator,
                        effectiveAtEpochMs = audit.effectiveAtEpochMs,
                        changeSet = audit.changeSet,
                        rollbackFromVersion = audit.rollbackFromVersion
                    )
                )
            )
        }

        /**
         * 查询配置审计记录
         *
         * Query config audit records
         *
         * GET /api/v1/scheduler/config/audits - 获取调度器配置变更历史。
         * Gets scheduler configuration change history.
         */
        get("/api/v1/scheduler/config/audits") {
            val limit = call.request.queryParameters["limit"]?.trim()?.toIntOrNull() ?: 100
            val audits = apiFacade.listSchedulerConfigAudits(limit)
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    data = audits.map { audit ->
                        SchedulerConfigAuditHttpResponse(
                            version = audit.version,
                            previousVersion = audit.previousVersion,
                            operator = audit.operator,
                            effectiveAtEpochMs = audit.effectiveAtEpochMs,
                            changeSet = audit.changeSet,
                            rollbackFromVersion = audit.rollbackFromVersion
                        )
                    }
                )
            )
        }

        // ==================== 时间线端点 ====================
        // ==================== Timeline endpoint ====================

        /**
         * 查询任务时间线
         *
         * Query task timeline
         *
         * GET /api/v1/tasks/{taskId}/timeline - 重构并返回任务的事件时间线。
         * Reconstructs and returns task event timeline.
         */
        get("/api/v1/tasks/{taskId}/timeline") {
            val taskId = call.parameters["taskId"]?.trim()
            if (taskId.isNullOrBlank()) {
                throw IllegalArgumentException("taskId must not be blank")
            }
            val limit = call.request.queryParameters["limit"]?.trim()?.toIntOrNull() ?: 500
            val report = apiFacade.replayTaskTimeline(taskId = taskId, limitEvents = limit)
            val task = apiFacade.get(taskId)
            if (task != null && !call.verifyTenantAccess(task.tenantId, tenantAuthEnabled)) {
                return@get
            }
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    data = TaskTimelineHttpResponse(
                        taskId = report.taskId,
                        generatedAtEpochMs = report.generatedAtEpochMs,
                        status = report.status,
                        events = report.events.map { event ->
                            TaskTimelineEventHttpResponse(
                                atEpochMs = event.atEpochMs,
                                type = event.type,
                                summary = event.summary,
                                attributes = event.attributes
                            )
                        },
                        gaps = report.gaps.map { gap ->
                            TaskTimelineGapHttpResponse(
                                severity = gap.severity,
                                message = gap.message,
                                relatedIds = gap.relatedIds
                            )
                        }
                    )
                )
            )
        }

        // ==================== 监控端点 ====================
        // ==================== Monitor endpoints ====================

        /**
         * 监控概览 JSON API
         *
         * Monitor overview JSON API
         *
         * GET /api/v1/monitor/overview - 返回系统监控概览数据（JSON 格式）。
         * Returns system monitoring overview data (JSON format).
         */
        get("/api/v1/monitor/overview") {
            if (
                !call.requireMonitorAccess(
                    monitorAuthEnabled = monitorAuthEnabled,
                    monitorLoginUrl = normalizedMonitorLoginUrl,
                    monitorAllowedRoles = normalizedMonitorAllowedRoles,
                    redirectOnUnauthenticated = false
                )
            ) {
                return@get
            }
            val limit = call.request.queryParameters["limit"]?.trim()?.toIntOrNull() ?: 200
            val report = apiFacade.monitorOverview(limitRecentTasks = limit)
            call.respond(
                ApiEnvelope(
                    code = "OK",
                    message = "success",
                    data = MonitorOverviewHttpResponse(
                        scheduler = MonitorSchedulerHttpResponse(
                            schedulerConfigVersion = report.scheduler.schedulerConfigVersion,
                            generatedAtEpochMs = report.scheduler.generatedAtEpochMs,
                            nodeHeartbeatTimeoutMs = report.scheduler.nodeHeartbeatTimeoutMs
                        ),
                        nodeTotals = report.nodeTotals,
                        tasks = MonitorTaskHttpResponse(
                            totalObservedTasks = report.tasks.totalObservedTasks,
                            queueDepth = report.tasks.queueDepth,
                            runningTasks = report.tasks.runningTasks,
                            failedTasks = report.tasks.failedTasks,
                            completedTasks = report.tasks.completedTasks,
                            statusCounts = report.tasks.statusCounts,
                            recentTasks = report.tasks.recentTasks.map { task ->
                                MonitorRecentTaskHttpResponse(
                                    taskId = task.taskId,
                                    tenantId = task.tenantId,
                                    status = task.status,
                                    complexity = task.complexity,
                                    timeSensitivity = task.timeSensitivity,
                                    priority = task.priority,
                                    assignedNodeId = task.assignedNodeId,
                                    consumedCost = task.consumedCost,
                                    updatedAtEpochMs = task.updatedAtEpochMs
                                )
                            }
                        ),
                        nodes = report.nodes.map { node ->
                            MonitorNodeHttpResponse(
                                nodeId = node.nodeId,
                                online = node.online,
                                health = node.health,
                                solverType = node.solverType,
                                performanceScore = node.performanceScore,
                                pricePerSecond = node.pricePerSecond,
                                availableUnits = node.availableUnits,
                                parallelUnits = node.parallelUnits,
                                lastHeartbeatEpochMs = node.lastHeartbeatEpochMs,
                                heartbeatLagMs = node.heartbeatLagMs
                            )
                        }
                    )
                )
            )
        }

        /**
         * 监控仪表盘页面
         *
         * Monitor dashboard page
         *
         * GET /monitor - 返回内置的 HTML 监控仪表盘页面。
         * Returns built-in HTML monitor dashboard page.
         */
        get("/monitor") {
            if (
                !call.requireMonitorAccess(
                    monitorAuthEnabled = monitorAuthEnabled,
                    monitorLoginUrl = normalizedMonitorLoginUrl,
                    monitorAllowedRoles = normalizedMonitorAllowedRoles,
                    redirectOnUnauthenticated = true
                )
            ) {
                return@get
            }
            call.respondText(
                text = monitorDashboardHtml(),
                contentType = ContentType.parse("text/html; charset=utf-8"),
                status = HttpStatusCode.OK
            )
        }

        /**
         * Prometheus 指标端点
         *
         * Prometheus metrics endpoint
         *
         * GET /metrics - 暴露 Prometheus 格式的指标数据，供 Prometheus 服务器抓取。
         * Exposes Prometheus format metrics data for Prometheus server to scrape.
         */
        get("/metrics") {
            val scrapePort = metricsPort as? MetricsScrapePort
            if (scrapePort == null) {
                call.respond(
                    HttpStatusCode.NotFound,
                    ApiEnvelope<Unit>(
                        code = "METRICS_NOT_ENABLED",
                        message = "metrics scrape endpoint is not enabled",
                        data = null
                    )
                )
                return@get
            }
            call.respondText(
                text = scrapePort.scrape(),
                contentType = ContentType.parse("text/plain; version=0.0.4; charset=utf-8"),
                status = HttpStatusCode.OK
            )
        }
    }
}

/**
 * 验证监控访问权限
 *
 * Verify monitor access permission
 *
 * 检查请求是否满足监控访问的认证和权限要求。如果认证失败或权限不足，
 * 返回相应的错误响应或重定向。
 *
 * Checks if request meets monitor access authentication and permission requirements.
 * If authentication fails or permission insufficient, returns appropriate error
 * response or redirect.
 *
 * @param monitorAuthEnabled 监控认证是否启用。
 *                            Whether monitor authentication is enabled.
 * @param monitorLoginUrl 监控登录页面 URL。
 *                         Monitor login page URL.
 * @param monitorAllowedRoles 允许访问的角色集合。
 *                             Roles set allowed to access.
 * @param redirectOnUnauthenticated 未认证时是否重定向到登录页。
 *                                   Whether to redirect to login page when unauthenticated.
 * @return 是否有权限访问。
 *         Whether has permission to access.
 */
private suspend fun io.ktor.server.application.ApplicationCall.requireMonitorAccess(
    monitorAuthEnabled: Boolean,
    monitorLoginUrl: String,
    monitorAllowedRoles: Set<String>,
    redirectOnUnauthenticated: Boolean
): Boolean {
    if (!monitorAuthEnabled) {
        return true
    }
    val userId = request.headers["X-User-Id"]?.trim()?.takeIf { it.isNotEmpty() }
    if (userId == null) {
        if (redirectOnUnauthenticated) {
            respondRedirect(monitorLoginUrl, permanent = false)
        } else {
            respond(
                HttpStatusCode.Unauthorized,
                ApiEnvelope<Unit>(
                    code = "UNAUTHENTICATED",
                    message = "authentication is required",
                    data = null
                )
            )
        }
        return false
    }
    val roles = request.headers["X-User-Roles"]
        ?.split(",")
        ?.asSequence()
        ?.map { it.trim().lowercase() }
        ?.filter { it.isNotEmpty() }
        ?.toSet()
        ?: emptySet()
    if (roles.intersect(monitorAllowedRoles).isEmpty()) {
        respond(
            HttpStatusCode.Forbidden,
            ApiEnvelope<Unit>(
                code = "MONITOR_ACCESS_DENIED",
                message = "monitor access requires one of roles: ${monitorAllowedRoles.joinToString(",")}",
                data = null
            )
        )
        return false
    }
    return true
}

/**
 * 验证租户访问权限
 *
 * Verify tenant access permission
 *
 * 检查请求是否有权访问指定租户的资源。通过比较请求头中的租户 ID 和资源的租户 ID 进行验证。
 *
 * Checks if request has permission to access specified tenant's resource.
 * Validates by comparing tenant ID in request header with resource's tenant ID.
 *
 * @param taskTenantId 任务所属的租户 ID。
 *                      Task's tenant ID.
 * @param tenantAuthEnabled 租户认证是否启用。
 *                           Whether tenant authentication is enabled.
 * @return 是否有权限访问。
 *         Whether has permission to access.
 */
private suspend fun io.ktor.server.application.ApplicationCall.verifyTenantAccess(
    taskTenantId: String,
    tenantAuthEnabled: Boolean
): Boolean {
    if (!tenantAuthEnabled) {
        return true
    }
    val headerTenantId = request.headers["X-Tenant-Id"]?.trim()?.takeIf { it.isNotEmpty() }
    if (headerTenantId == null) {
        respond(
            HttpStatusCode.Forbidden,
            ApiEnvelope<Unit>(
                code = "TENANT_AUTH_REQUIRED",
                message = "X-Tenant-Id header is required",
                data = null
            )
        )
        return false
    }
    if (headerTenantId != taskTenantId) {
        respond(
            HttpStatusCode.Forbidden,
            ApiEnvelope<Unit>(
                code = "TENANT_ACCESS_DENIED",
                message = "tenant is not allowed to access this task",
                data = null
            )
        )
        return false
    }
    return true
}

/**
 * 返回错误响应
 *
 * Return error response
 *
 * 将异常转换为标准化的 API 错误响应，包含错误码、错误消息和追踪 ID。
 * Converts exception to standardized API error response,
 * containing error code, error message and trace ID.
 *
 * @param call Ktor ApplicationCall 实例。
 *              Ktor ApplicationCall instance.
 * @param throwable 异常对象。
 *                  Exception object.
 */
private suspend fun respondError(
    call: io.ktor.server.application.ApplicationCall,
    throwable: Throwable
) {
    val normalized = if (throwable is BadRequestException) {
        RemoteSolverException(
            code = RemoteSolverErrorCode.INVALID_ARGUMENT,
            message = throwable.message ?: "Invalid argument",
            cause = throwable
        )
    } else {
        RemoteSolverErrorMapper.normalize(throwable)
    }
    call.respond(
        mapHttpStatus(normalized.code),
        ApiEnvelope<Unit>(
            code = normalized.code.name,
            message = normalized.message,
            traceId = call.extractTraceId(),
            data = null
        )
    )
}

/**
 * 提取追踪 ID
 *
 * Extract trace ID
 *
 * 从请求头中提取追踪 ID 或请求 ID，用于请求链路追踪。
 * Extracts trace ID or request ID from request headers for request chain tracing.
 *
 * @return 追踪 ID，如果不存在则返回 null。
 *         Trace ID, or null if not exists.
 */
private fun io.ktor.server.application.ApplicationCall.extractTraceId(): String? {
    return request.headers["X-Trace-Id"]?.trim()?.takeIf { it.isNotEmpty() }
        ?: request.headers["X-Request-Id"]?.trim()?.takeIf { it.isNotEmpty() }
}

/**
 * 映射错误码到 HTTP 状态码
 *
 * Map error code to HTTP status code
 *
 * 将内部错误码转换为对应的 HTTP 状态码，用于 HTTP 响应。
 * Converts internal error code to corresponding HTTP status code for HTTP response.
 *
 * @param code 远程求解器错误码。
 *              Remote solver error code.
 * @return HTTP 状态码。
 *         HTTP status code.
 */
private fun mapHttpStatus(code: RemoteSolverErrorCode): HttpStatusCode =
    when (code) {
        RemoteSolverErrorCode.INVALID_ARGUMENT -> HttpStatusCode.BadRequest
        RemoteSolverErrorCode.INVALID_TASK_STATE_TRANSITION -> HttpStatusCode.Conflict
        RemoteSolverErrorCode.NO_ELIGIBLE_NODE_AVAILABLE,
        RemoteSolverErrorCode.NO_COMPATIBLE_NODE_AVAILABLE,
        RemoteSolverErrorCode.NODE_OFFLINE -> HttpStatusCode.ServiceUnavailable
        RemoteSolverErrorCode.TASK_FAILED,
        RemoteSolverErrorCode.TASK_FAILED_HARD_TIMEOUT,
        RemoteSolverErrorCode.TASK_FAILED_SLICE_TIMEOUT,
        RemoteSolverErrorCode.TASK_FAILED_BUDGET_EXCEEDED,
        RemoteSolverErrorCode.SOLVER_EXECUTION_FAILED,
        RemoteSolverErrorCode.CHECKPOINT_EXPORT_FAILED,
        RemoteSolverErrorCode.REMOTE_SOLVE_NOT_COMPLETED_WITHIN_MAX_ROUNDS,
        RemoteSolverErrorCode.TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS -> HttpStatusCode.UnprocessableEntity
        RemoteSolverErrorCode.EVENT_PUBLISH_FAILED,
        RemoteSolverErrorCode.STORAGE_IO_FAILED,
        RemoteSolverErrorCode.INTERNAL_ERROR -> HttpStatusCode.InternalServerError
    }

// ==================== HTTP 请求/响应数据类 ====================
// ==================== HTTP request/response data classes ====================

/**
 * 提交任务 HTTP 请求体
 *
 * Submit task HTTP request body
 *
 * 包含任务提交所需的所有参数。
 * Contains all parameters required for task submission.
 *
 * @param requestId 请求 ID，用于幂等控制。
 *                   Request ID for idempotency control.
 * @param tenantId 租户 ID。
 *                  Tenant ID.
 * @param complexity 任务复杂度（SIMPLE/COMPLEX）。
 *                    Task complexity (SIMPLE/COMPLEX).
 * @param timeSensitivity 时间敏感性（HIGH/LOW）。
 *                         Time sensitivity (HIGH/LOW).
 * @param priority 任务优先级。
 *                  Task priority.
 * @param payloadRef 输入数据对象引用路径。
 *                   Input data object reference path.
 * @param budgetScope 预算范围标识。
 *                    Budget scope identifier.
 * @param budgetLimit 预算上限。
 *                     Budget limit.
 * @param deadlineEpochMs 截止时间戳。
 *                         Deadline timestamp.
 */
@Serializable
private data class SubmitTaskHttpRequest(
    val requestId: String? = null,
    val tenantId: String? = null,
    val complexity: String? = null,
    val timeSensitivity: String? = null,
    val priority: Int? = null,
    val payloadRef: String? = null,
    val budgetScope: String? = null,
    val budgetLimit: Double? = null,
    val deadlineEpochMs: Long? = null
)

/**
 * 停止任务 HTTP 请求体
 *
 * Stop task HTTP request body
 *
 * 包含停止任务所需的参数。
 * Contains parameters required for stopping task.
 *
 * @param reason 停止原因描述。
 *               Stop reason description.
 * @param operator 操作者标识。
 *                 Operator identifier.
 * @param source 操作来源标识。
 *               Operation source identifier.
 */
@Serializable
private data class StopTaskHttpRequest(
    val reason: String? = null,
    val operator: String? = null,
    val source: String? = null
)

/**
 * 恢复任务 HTTP 请求体
 *
 * Resume task HTTP request body
 *
 * 包含恢复任务所需的参数。
 * Contains parameters required for resuming task.
 *
 * @param operator 操作者标识。
 *                 Operator identifier.
 * @param source 操作来源标识。
 *               Operation source identifier.
 * @param reason 恢复原因描述。
 *               Resume reason description.
 */
@Serializable
private data class ResumeTaskHttpRequest(
    val operator: String? = null,
    val source: String? = null,
    val reason: String? = null
)

/**
 * 调度器热加载 HTTP 请求体
 *
 * Scheduler hot reload HTTP request body
 *
 * 包含调度器配置热加载所需的参数。
 * Contains parameters required for scheduler config hot reload.
 *
 * @param operator 操作者标识。
 *                 Operator identifier.
 * @param changeSet 配置变更键值对集合。
 *                  Config change key-value pairs set.
 * @param requestedVersion 请求的目标版本号。
 *                          Requested target version number.
 * @param effectiveAtEpochMs 生效时间戳。
 *                            Effective timestamp.
 */
@Serializable
private data class SchedulerHotReloadHttpRequest(
    val operator: String? = null,
    val changeSet: Map<String, String>? = null,
    val requestedVersion: String? = null,
    val effectiveAtEpochMs: Long? = null
)

/**
 * 调度器回滚 HTTP 请求体
 *
 * Scheduler rollback HTTP request body
 *
 * 包含调度器配置回滚所需的参数。
 * Contains parameters required for scheduler config rollback.
 *
 * @param operator 操作者标识。
 *                 Operator identifier.
 * @param targetVersion 回滚目标版本号。
 *                       Rollback target version number.
 * @param requestedVersion 请求的新版本号。
 *                          Requested new version number.
 * @param effectiveAtEpochMs 生效时间戳。
 *                            Effective timestamp.
 */
@Serializable
private data class SchedulerRollbackHttpRequest(
    val operator: String? = null,
    val targetVersion: String? = null,
    val requestedVersion: String? = null,
    val effectiveAtEpochMs: Long? = null
)

/**
 * 提交任务 HTTP 响应体
 *
 * Submit task HTTP response body
 *
 * 返回任务提交结果信息。
 * Returns task submission result information.
 *
 * @param taskId 任务 ID。
 *               Task ID.
 * @param accepted 是否已接受任务。
 *                  Whether task was accepted.
 * @param status 任务状态名称。
 *               Task status name.
 * @param message 结果消息。
 *                Result message.
 */
@Serializable
private data class SubmitTaskHttpResponse(
    val taskId: String,
    val accepted: Boolean,
    val status: String,
    val message: String
)

/**
 * 任务视图 HTTP 响应体
 *
 * Task view HTTP response body
 *
 * 返回任务详情信息。
 * Returns task detail information.
 *
 * @param taskId 任务 ID。
 *               Task ID.
 * @param tenantId 租户 ID。
 *                  Tenant ID.
 * @param status 任务状态名称。
 *               Task status name.
 * @param currentNodeId 当前执行节点 ID。
 *                      Current execution node ID.
 * @param latestCheckpointPath 最新检查点路径。
 *                              Latest checkpoint path.
 * @param latestResultPath 最新结果路径。
 *                          Latest result path.
 * @param consumedCost 已消耗成本。
 *                     Consumed cost.
 */
@Serializable
private data class TaskViewHttpResponse(
    val taskId: String,
    val tenantId: String,
    val status: String,
    val currentNodeId: String? = null,
    val latestCheckpointPath: String? = null,
    val latestResultPath: String? = null,
    val consumedCost: Double
)

/**
 * 任务操作 HTTP 响应体
 *
 * Task action HTTP response body
 *
 * 返回任务操作结果信息。
 * Returns task operation result information.
 *
 * @param taskId 任务 ID。
 *               Task ID.
 * @param status 任务状态名称。
 *               Task status name.
 */
@Serializable
private data class TaskActionHttpResponse(
    val taskId: String,
    val status: String
)

/**
 * 调度器配置审计 HTTP 响应体
 *
 * Scheduler config audit HTTP response body
 *
 * 返回调度器配置变更审计记录。
 * Returns scheduler config change audit record.
 *
 * @param version 当前版本号。
 *                 Current version number.
 * @param previousVersion 前一版本号。
 *                        Previous version number.
 * @param operator 操作者标识。
 *                 Operator identifier.
 * @param effectiveAtEpochMs 生效时间戳。
 *                            Effective timestamp.
 * @param changeSet 配置变更集合。
 *                  Config change set.
 * @param rollbackFromVersion 回滚来源版本号（如果是回滚操作）。
 *                             Rollback source version number (if rollback operation).
 */
@Serializable
private data class SchedulerConfigAuditHttpResponse(
    val version: String,
    val previousVersion: String,
    val operator: String,
    val effectiveAtEpochMs: Long,
    val changeSet: Map<String, String>,
    val rollbackFromVersion: String? = null
)

/**
 * 任务时间线 HTTP 响应体
 *
 * Task timeline HTTP response body
 *
 * 返回任务事件时间线重构结果。
 * Returns task event timeline reconstruction result.
 *
 * @param taskId 任务 ID。
 *               Task ID.
 * @param generatedAtEpochMs 生成时间戳。
 *                            Generation timestamp.
 * @param status 任务状态。
 *               Task status.
 * @param events 事件列表。
 *               Events list.
 * @param gaps 时间间隙列表（可能的问题点）。
 *             Time gaps list (potential problem points).
 */
@Serializable
private data class TaskTimelineHttpResponse(
    val taskId: String,
    val generatedAtEpochMs: Long,
    val status: String,
    val events: List<TaskTimelineEventHttpResponse>,
    val gaps: List<TaskTimelineGapHttpResponse>
)

/**
 * 任务时间线事件 HTTP 响应体
 *
 * Task timeline event HTTP response body
 *
 * 表示时间线上的单个事件。
 * Represents single event on timeline.
 *
 * @param atEpochMs 事件发生时间戳。
 *                   Event occurrence timestamp.
 * @param type 事件类型。
 *             Event type.
 * @param summary 事件摘要描述。
 *                Event summary description.
 * @param attributes 事件属性集合。
 *                   Event attributes set.
 */
@Serializable
private data class TaskTimelineEventHttpResponse(
    val atEpochMs: Long,
    val type: String,
    val summary: String,
    val attributes: Map<String, String> = emptyMap()
)

/**
 * 任务时间线间隙 HTTP 响应体
 *
 * Task timeline gap HTTP response body
 *
 * 表示时间线上的异常间隙，可能表示问题或数据缺失。
 * Represents abnormal gap on timeline, possibly indicating problem or data missing.
 *
 * @param severity 间隙严重程度（INFO/WARN/ERROR）。
 *                  Gap severity level (INFO/WARN/ERROR).
 * @param message 间隙描述消息。
 *                Gap description message.
 * @param relatedIds 相关 ID 映射。
 *                   Related IDs mapping.
 */
@Serializable
private data class TaskTimelineGapHttpResponse(
    val severity: String,
    val message: String,
    val relatedIds: Map<String, String> = emptyMap()
)

/**
 * 监控概览 HTTP 响应体
 *
 * Monitor overview HTTP response body
 *
 * 返回系统监控概览数据。
 * Returns system monitoring overview data.
 *
 * @param scheduler 调度器状态信息。
 *                  Scheduler status information.
 * @param nodeTotals 节点统计汇总。
 *                   Node statistics summary.
 * @param tasks 任务统计信息。
 *              Task statistics information.
 * @param nodes 节点详细信息列表。
 *              Node detail information list.
 */
@Serializable
private data class MonitorOverviewHttpResponse(
    val scheduler: MonitorSchedulerHttpResponse,
    val nodeTotals: Map<String, Int>,
    val tasks: MonitorTaskHttpResponse,
    val nodes: List<MonitorNodeHttpResponse>
)

/**
 * 监控调度器 HTTP 响应体
 *
 * Monitor scheduler HTTP response body
 *
 * 调度器状态信息。
 * Scheduler status information.
 *
 * @param schedulerConfigVersion 调度器配置版本号。
 *                                Scheduler config version number.
 * @param generatedAtEpochMs 生成时间戳。
 *                            Generation timestamp.
 * @param nodeHeartbeatTimeoutMs 节点心跳超时阈值。
 *                                Node heartbeat timeout threshold.
 */
@Serializable
private data class MonitorSchedulerHttpResponse(
    val schedulerConfigVersion: String,
    val generatedAtEpochMs: Long,
    val nodeHeartbeatTimeoutMs: Long
)

/**
 * 监控任务 HTTP 响应体
 *
 * Monitor task HTTP response body
 *
 * 任务统计信息。
 * Task statistics information.
 *
 * @param totalObservedTasks 总观察任务数。
 *                            Total observed tasks count.
 * @param queueDepth 队列深度。
 *                   Queue depth.
 * @param runningTasks 运行中任务数。
 *                      Running tasks count.
 * @param failedTasks 失败任务数。
 *                    Failed tasks count.
 * @param completedTasks 完成任务数。
 *                       Completed tasks count.
 * @param statusCounts 按状态统计的任务数量。
 *                      Task count by status.
 * @param recentTasks 最近任务列表。
 *                    Recent tasks list.
 */
@Serializable
private data class MonitorTaskHttpResponse(
    val totalObservedTasks: Int,
    val queueDepth: Int,
    val runningTasks: Int,
    val failedTasks: Int,
    val completedTasks: Int,
    val statusCounts: Map<String, Int>,
    val recentTasks: List<MonitorRecentTaskHttpResponse>
)

/**
 * 监控最近任务 HTTP 响应体
 *
 * Monitor recent task HTTP response body
 *
 * 最近任务详情信息。
 * Recent task detail information.
 *
 * @param taskId 任务 ID。
 *               Task ID.
 * @param tenantId 租户 ID。
 *                  Tenant ID.
 * @param status 任务状态。
 *               Task status.
 * @param complexity 任务复杂度。
 *                    Task complexity.
 * @param timeSensitivity 时间敏感性。
 *                         Time sensitivity.
 * @param priority 任务优先级。
 *                  Task priority.
 * @param assignedNodeId 分配的节点 ID。
 *                       Assigned node ID.
 * @param consumedCost 已消耗成本。
 *                     Consumed cost.
 * @param updatedAtEpochMs 更新时间戳。
 *                          Update timestamp.
 */
@Serializable
private data class MonitorRecentTaskHttpResponse(
    val taskId: String,
    val tenantId: String,
    val status: String,
    val complexity: String,
    val timeSensitivity: String,
    val priority: Int,
    val assignedNodeId: String? = null,
    val consumedCost: Double,
    val updatedAtEpochMs: Long
)

/**
 * 监控节点 HTTP 响应体
 *
 * Monitor node HTTP response body
 *
 * 节点详情信息。
 * Node detail information.
 *
 * @param nodeId 节点 ID。
 *               Node ID.
 * @param online 是否在线。
 *                Whether online.
 * @param health 健康状态（ONLINE/STALE/OFFLINE）。
 *               Health status (ONLINE/STALE/OFFLINE).
 * @param solverType 求解器类型。
 *                   Solver type.
 * @param performanceScore 性能评分。
 *                         Performance score.
 * @param pricePerSecond 每秒价格。
 *                       Price per second.
 * @param availableUnits 可用单元数。
 *                       Available units count.
 * @param parallelUnits 并行单元总数。
 *                      Total parallel units count.
 * @param lastHeartbeatEpochMs 最后心跳时间戳。
 *                              Last heartbeat timestamp.
 * @param heartbeatLagMs 心跳延迟毫秒数。
 *                       Heartbeat lag milliseconds.
 */
@Serializable
private data class MonitorNodeHttpResponse(
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
 * 能力探测 HTTP 响应体。
 *
 * Capability probe HTTP response body.
 *
 * @property schemaVersion 能力摘要 schema 版本 / Capability summary schema version
 * @property protocolVersions 服务端支持的协议版本 / Protocol versions supported by the server
 * @property supportedModelTypes 当前在线节点支持的模型类型 / Model types supported by online nodes
 * @property supportsPortableCheckpoint 是否支持 portable checkpoint / Whether portable checkpoints are supported
 * @property supportsNativeCheckpoint 是否支持原生搜索状态恢复 / Whether native search-state resume is supported
 */
@Serializable
private data class SolverCapabilitiesHttpResponse(
    val schemaVersion: String,
    val protocolVersions: Set<String>,
    val supportedModelTypes: Set<String>,
    val supportsPortableCheckpoint: Boolean,
    val supportsNativeCheckpoint: Boolean
)

/**
 * API 响应封装
 *
 * API response envelope
 *
 * 标准化的 API 响应结构，包含状态码、消息、追踪 ID 和数据。
 * Standardized API response structure, containing status code, message, trace ID and data.
 *
 * @param code 状态码（"OK" 或错误码）。
 *             Status code ("OK" or error code).
 * @param message 响应消息。
 *                Response message.
 * @param traceId 追踪 ID。
 *                Trace ID.
 * @param data 响应数据，可为 null。
 *             Response data, can be null.
 */
@Serializable
private data class ApiEnvelope<T>(
    val code: String,
    val message: String,
    val traceId: String? = null,
    val data: T? = null
)

/**
 * 健康检查响应体
 *
 * Health check response body
 *
 * 健康检查状态信息。
 * Health check status information.
 *
 * @param status 状态（"UP" 或 "DOWN"）。
 *               Status ("UP" or "DOWN").
 * @param timestampEpochMs 检查时间戳。
 *                          Check timestamp.
 */
@Serializable
private data class HealthCheckResponse(
    val status: String,
    val timestampEpochMs: Long
)

/**
 * 监控仪表盘 HTML 内容
 *
 * Monitor dashboard HTML content
 *
 * 返回内置的单页 HTML 监控仪表盘，包含节点状态、任务队列、任务列表等可视化展示。
 * Returns built-in single-page HTML monitor dashboard, containing node status,
 * task queue, task list and other visual displays.
 *
 * @return HTML 页面内容字符串。
 *         HTML page content string.
 */
private fun monitorDashboardHtml(): String =
    """
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Remote Solver Monitor</title>
  <style>
    :root {
      --bg: #f4f7f2;
      --panel: #ffffff;
      --ink: #0f1720;
      --muted: #4b5563;
      --accent: #0f766e;
      --warn: #b45309;
      --danger: #b91c1c;
      --line: #d1d5db;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: "IBM Plex Sans", "Noto Sans SC", "Segoe UI", sans-serif;
      color: var(--ink);
      background: radial-gradient(circle at 20% 0%, #dff4ef 0%, var(--bg) 50%);
    }
    .wrap {
      max-width: 1120px;
      margin: 0 auto;
      padding: 20px 16px 32px;
    }
    h1 { margin: 0 0 6px; font-size: 30px; }
    .sub { color: var(--muted); margin-bottom: 16px; }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 10px;
      margin-bottom: 14px;
    }
    .card {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 12px;
    }
    .label { color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .06em; }
    .value { font-size: 24px; font-weight: 700; margin-top: 4px; }
    table {
      width: 100%;
      border-collapse: collapse;
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 10px;
      overflow: hidden;
    }
    th, td {
      text-align: left;
      padding: 8px 10px;
      border-bottom: 1px solid var(--line);
      font-size: 13px;
    }
    th { background: #ecfdf5; }
    tr:last-child td { border-bottom: none; }
    .ok { color: var(--accent); font-weight: 600; }
    .stale { color: var(--warn); font-weight: 600; }
    .offline { color: var(--danger); font-weight: 600; }
    .foot { margin-top: 10px; color: var(--muted); font-size: 12px; }
    .section-title { margin: 14px 0 8px; font-size: 16px; }
    .status-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
      gap: 8px;
      margin-bottom: 10px;
    }
    .status-pill {
      background: #f0fdf4;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 8px;
      min-height: 60px;
    }
    .status-name { font-size: 11px; color: var(--muted); text-transform: uppercase; }
    .status-value { font-size: 20px; font-weight: 700; margin-top: 3px; }
    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin: 10px 0;
      align-items: center;
    }
    .toolbar input, .toolbar select {
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 6px 8px;
      background: #fff;
      min-width: 160px;
    }
    .task-table-wrap {
      margin-top: 12px;
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 10px;
      overflow: auto;
    }
  </style>
</head>
<body>
  <div class="wrap">
    <h1>Remote Solver Monitor</h1>
    <div class="sub" id="meta">Loading...</div>
    <div class="grid">
      <div class="card"><div class="label">Queue Depth</div><div class="value" id="queueDepth">-</div></div>
      <div class="card"><div class="label">Running Tasks</div><div class="value" id="runningTasks">-</div></div>
      <div class="card"><div class="label">Failed Tasks</div><div class="value" id="failedTasks">-</div></div>
      <div class="card"><div class="label">Online Nodes</div><div class="value" id="onlineNodes">-</div></div>
      <div class="card"><div class="label">Stale Nodes</div><div class="value" id="staleNodes">-</div></div>
      <div class="card"><div class="label">Used Units</div><div class="value" id="usedUnits">-</div></div>
    </div>
    <h2 class="section-title">Task Status Distribution</h2>
    <div class="status-grid" id="statusGrid"></div>
    <h2 class="section-title">Node Filter</h2>
    <div class="toolbar">
      <select id="healthFilter">
        <option value="ALL">All Health</option>
        <option value="ONLINE">ONLINE</option>
        <option value="STALE">STALE</option>
        <option value="OFFLINE">OFFLINE</option>
      </select>
      <input id="nodeSearch" type="text" placeholder="Search node id or solver type" />
    </div>
    <table>
      <thead>
        <tr>
          <th>Node</th>
          <th>Health</th>
          <th>Solver</th>
          <th>Units</th>
          <th>Perf</th>
          <th>Price/s</th>
          <th>Heartbeat Lag(ms)</th>
        </tr>
      </thead>
      <tbody id="nodesBody">
        <tr><td colspan="7">Loading...</td></tr>
      </tbody>
    </table>
    <h2 class="section-title">Recent Tasks</h2>
    <div class="task-table-wrap">
      <table>
        <thead>
          <tr>
            <th>Task</th>
            <th>Tenant</th>
            <th>Status</th>
            <th>Complexity</th>
            <th>Sensitivity</th>
            <th>Priority</th>
            <th>Node</th>
            <th>Cost</th>
            <th>Updated</th>
          </tr>
        </thead>
        <tbody id="tasksBody">
          <tr><td colspan="9">Loading...</td></tr>
        </tbody>
      </table>
    </div>
    <div class="foot">Auto refresh every 3s.</div>
  </div>
  <script>
    let latestNodes = [];
    function renderNodes() {
      const healthFilter = document.getElementById('healthFilter').value;
      const query = document.getElementById('nodeSearch').value.trim().toLowerCase();
      const filtered = latestNodes.filter(node => {
        if (healthFilter !== 'ALL' && node.health !== healthFilter) {
          return false;
        }
        if (!query) {
          return true;
        }
        return (node.nodeId || '').toLowerCase().includes(query) ||
          (node.solverType || '').toLowerCase().includes(query);
      });
      const body = document.getElementById('nodesBody');
      if (!filtered.length) {
        body.innerHTML = '<tr><td colspan="7">No nodes matched current filter</td></tr>';
        return;
      }
      body.innerHTML = filtered.map(node => {
        const cls = node.health === 'ONLINE' ? 'ok' : (node.health === 'STALE' ? 'stale' : 'offline');
        return '<tr>' +
          '<td>' + node.nodeId + '</td>' +
          '<td class="' + cls + '">' + node.health + '</td>' +
          '<td>' + node.solverType + '</td>' +
          '<td>' + node.availableUnits + '/' + node.parallelUnits + '</td>' +
          '<td>' + Number(node.performanceScore).toFixed(2) + '</td>' +
          '<td>' + Number(node.pricePerSecond).toFixed(4) + '</td>' +
          '<td>' + node.heartbeatLagMs + '</td>' +
        '</tr>';
      }).join('');
    }

    function renderStatusCounts(statusCounts) {
      const grid = document.getElementById('statusGrid');
      const pairs = Object.entries(statusCounts || {});
      if (!pairs.length) {
        grid.innerHTML = '<div class="status-pill"><div class="status-name">none</div><div class="status-value">0</div></div>';
        return;
      }
      grid.innerHTML = pairs.map(([name, value]) =>
        '<div class="status-pill">' +
          '<div class="status-name">' + name + '</div>' +
          '<div class="status-value">' + value + '</div>' +
        '</div>'
      ).join('');
    }

    function renderRecentTasks(recentTasks) {
      const body = document.getElementById('tasksBody');
      if (!recentTasks || !recentTasks.length) {
        body.innerHTML = '<tr><td colspan="9">No tasks observed</td></tr>';
        return;
      }
      body.innerHTML = recentTasks.map(task =>
        '<tr>' +
          '<td>' + task.taskId + '</td>' +
          '<td>' + task.tenantId + '</td>' +
          '<td>' + task.status + '</td>' +
          '<td>' + task.complexity + '</td>' +
          '<td>' + task.timeSensitivity + '</td>' +
          '<td>' + task.priority + '</td>' +
          '<td>' + (task.assignedNodeId || '') + '</td>' +
          '<td>' + Number(task.consumedCost).toFixed(4) + '</td>' +
          '<td>' + new Date(task.updatedAtEpochMs).toLocaleString() + '</td>' +
        '</tr>'
      ).join('');
    }

    async function loadOverview() {
      try {
        const res = await fetch('/api/v1/monitor/overview?limit=300', { cache: 'no-store' });
        const json = await res.json();
        if (!json || json.code !== 'OK' || !json.data) {
          throw new Error('invalid response');
        }
        const d = json.data;
        document.getElementById('meta').textContent =
          'config=' + d.scheduler.schedulerConfigVersion + ' generatedAt=' + new Date(d.scheduler.generatedAtEpochMs).toLocaleString();
        document.getElementById('queueDepth').textContent = d.tasks.queueDepth;
        document.getElementById('runningTasks').textContent = d.tasks.runningTasks;
        document.getElementById('failedTasks').textContent = d.tasks.failedTasks;
        document.getElementById('onlineNodes').textContent = d.nodeTotals.onlineNodes ?? 0;
        document.getElementById('staleNodes').textContent = d.nodeTotals.staleNodes ?? 0;
        document.getElementById('usedUnits').textContent = d.nodeTotals.usedUnits ?? 0;
        renderStatusCounts(d.tasks.statusCounts || {});
        renderRecentTasks(d.tasks.recentTasks || []);

        if (!d.nodes || d.nodes.length === 0) {
          latestNodes = [];
          document.getElementById('nodesBody').innerHTML = '<tr><td colspan="7">No nodes registered</td></tr>';
          return;
        }
        latestNodes = d.nodes;
        renderNodes();
      } catch (e) {
        document.getElementById('meta').textContent = 'monitor loading failed: ' + e;
      }
    }
    document.getElementById('healthFilter').addEventListener('change', renderNodes);
    document.getElementById('nodeSearch').addEventListener('input', renderNodes);
    loadOverview();
    setInterval(loadOverview, 3000);
  </script>
</body>
</html>
""".trimIndent()
