/**
 * RemoteSolverApiMain - 远程求解器 API 主入口
 *
 * RemoteSolverApiMain - Remote solver API main entry point.
 *
 * 远程求解器 HTTP API 服务器的启动入口点。
 * 提供任务提交、状态查询、监控和管理功能的 HTTP 接口。
 *
 * Bootstrap entry point for remote solver HTTP API server.
 * Provides HTTP interfaces for task submission, status query, monitoring, and management.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import fuookami.ospf.framework.remote_solver.adapter.http.RemoteSolverHttpServer
import fuookami.ospf.framework.remote_solver.application.HttpMonitorAlertWebhookPublisher
import fuookami.ospf.framework.remote_solver.application.MonitorAlertingConfig
import fuookami.ospf.framework.remote_solver.application.MonitorAlertingService
import kotlinx.coroutines.runBlocking
import java.util.concurrent.atomic.AtomicBoolean

/**
 * 远程求解器 API 主入口对象
 *
 * Remote solver API main entry object.
 *
 * 启动 HTTP API 服务器，包含调度循环和监控告警功能。
 * 支持租户认证、监控认证和 webhook 告警配置。
 *
 * Starts HTTP API server, including scheduling loop and monitoring alerting.
 * Supports tenant authentication, monitor authentication, and webhook alerting configuration.
 */
object RemoteSolverApiMain {
    /**
     * 主入口方法
     *
     * Main entry method.
     *
     * 启动远程求解器 API 服务器的主方法。
     * 解析配置、创建运行时组件、启动 HTTP 服务器并运行调度循环。
     *
     * Main method for starting remote solver API server.
     * Parses configuration, creates runtime components, starts HTTP server,
     * and runs scheduling loop.
     *
     * @param args 命令行参数
     *             CLI arguments
     */
    @JvmStatic
    fun main(args: Array<String>) {
        val configPath = BootstrapCliSupport.resolveConfigPath(args)
        val host = resolveHost(args)
        val properties = BootstrapCliSupport.loadProperties(configPath)
        val port = resolvePort(args, properties)
        val tenantAuthEnabled = resolveTenantAuthEnabled(properties)
        val monitorAuthEnabled = resolveMonitorAuthEnabled(properties)
        val monitorLoginUrl = resolveMonitorLoginUrl(properties)
        val monitorAllowedRoles = resolveMonitorAllowedRoles(properties)
        val runtime = RemoteSolverBootstrapFactory.create(properties = properties)
        val monitorAlerting = resolveMonitorAlerting(runtime = runtime, properties = properties)
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            metricsPort = runtime.metricsPort,
            tenantAuthEnabled = tenantAuthEnabled,
            monitorAuthEnabled = monitorAuthEnabled,
            monitorLoginUrl = monitorLoginUrl,
            monitorAllowedRoles = monitorAllowedRoles,
            host = host,
            port = port
        )
        val intervalMs = properties["scheduler.loop.interval-ms"]
            ?.trim()
            ?.toLongOrNull()
            ?.coerceAtLeast(10L)
            ?: 200L

        println("remote-solver api started")
        println("config=$configPath")
        println("host=$host")
        println("port=${server.port()}")
        println("scheduleIntervalMs=$intervalMs")
        println("tenantAuthEnabled=$tenantAuthEnabled")
        println("monitorAuthEnabled=$monitorAuthEnabled")
        println("monitorLoginUrl=$monitorLoginUrl")
        println("monitorAllowedRoles=${monitorAllowedRoles.joinToString(",")}")
        monitorAlerting?.let {
            println("monitorAlertEnabled=true")
        } ?: println("monitorAlertEnabled=false")

        val running = AtomicBoolean(true)
        Runtime.getRuntime().addShutdownHook(
            Thread {
                running.set(false)
                server.stop(0)
                println("remote-solver api stopping")
            }
        )
        server.start()

        runBlocking {
            while (running.get()) {
                runtime.service.scheduleOnce()
                monitorAlerting?.processIfDue()
                Thread.sleep(intervalMs)
            }
        }
    }

    /**
     * 解析服务器主机地址
     *
     * Resolves server host address.
     *
     * 从命令行参数解析服务器监听地址，默认为 0.0.0.0。
     *
     * Parses server listening address from CLI arguments,
     * defaults to 0.0.0.0.
     *
     * @param args 命令行参数
     *             CLI arguments
     * @return 主机地址
     *         Host address
     */
    private fun resolveHost(args: Array<String>): String {
        val parsed = BootstrapCliSupport.parseArgs(args)
        return parsed["host"]?.trim()?.takeIf { it.isNotEmpty() } ?: "0.0.0.0"
    }

    /**
     * 解析服务器端口
     *
     * Resolves server port.
     *
     * 从命令行参数或配置文件解析服务器监听端口。
     * 优先级：CLI 参数 > 配置文件 > 默认值（18080）。
     *
     * Parses server listening port from CLI arguments or config file.
     * Priority: CLI argument > Config file > Default (18080).
     *
     * @param args 命令行参数
     *             CLI arguments
     * @param properties 配置属性
     *                   Configuration properties
     * @return 端口号（1-65535）
     *         Port number (1-65535)
     * @throws IllegalArgumentException 端口范围无效时抛出
     *                                  Thrown when port range is invalid
     */
    private fun resolvePort(args: Array<String>, properties: Map<String, String>): Int {
        val parsed = BootstrapCliSupport.parseArgs(args)
        val cliPort = parsed["port"]?.toIntOrNull()
        val raw = cliPort
            ?: properties["api.http.port"]?.trim()?.toIntOrNull()
            ?: 18080
        require(raw in 1..65535) { "api port must be between 1 and 65535, but was $raw" }
        return raw
    }

    /**
     * 解析租户认证启用状态
     *
     * Resolves tenant authentication enabled status.
     *
     * 从配置属性解析是否启用租户认证，默认不启用。
     *
     * Parses whether to enable tenant authentication from config properties,
     * defaults to disabled.
     *
     * @param properties 配置属性
     *                   Configuration properties
     * @return 是否启用租户认证
     *         Whether tenant authentication is enabled
     */
    private fun resolveTenantAuthEnabled(properties: Map<String, String>): Boolean {
        return resolveBooleanProperty(
            properties = properties,
            key = "api.tenant.auth.enabled",
            defaultValue = false
        )
    }

    /**
     * 解析监控认证启用状态
     *
     * Resolves monitor authentication enabled status.
     *
     * 从配置属性解析是否启用监控端点认证，默认启用。
     *
     * Parses whether to enable monitor endpoint authentication from config properties,
     * defaults to enabled.
     *
     * @param properties 配置属性
     *                   Configuration properties
     * @return 是否启用监控认证
     *         Whether monitor authentication is enabled
     */
    private fun resolveMonitorAuthEnabled(properties: Map<String, String>): Boolean {
        return resolveBooleanProperty(
            properties = properties,
            key = "api.monitor.auth.enabled",
            defaultValue = true
        )
    }

    /**
     * 解析监控登录 URL
     *
     * Resolves monitor login URL.
     *
     * 从配置属性解析监控端点的登录页面 URL，默认为 /login。
     *
     * Parses login page URL for monitor endpoints from config properties,
     * defaults to /login.
     *
     * @param properties 配置属性
     *                   Configuration properties
     * @return 登录页面 URL
     *         Login page URL
     */
    private fun resolveMonitorLoginUrl(properties: Map<String, String>): String {
        return properties["api.auth.login-url"]
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?: "/login"
    }

    /**
     * 解析监控允许的角色集合
     *
     * Resolves monitor allowed roles set.
     *
     * 从配置属性解析允许访问监控端点的用户角色集合。
     * 默认为 admin 和 monitor_read。
     *
     * Parses user roles allowed to access monitor endpoints from config properties.
     * Defaults to admin and monitor_read.
     *
     * @param properties 配置属性
     *                   Configuration properties
     * @return 允许的角色集合
     *         Allowed roles set
     */
    private fun resolveMonitorAllowedRoles(properties: Map<String, String>): Set<String> {
        val parsed = properties["api.monitor.auth.roles"]
            ?.split(",")
            ?.asSequence()
            ?.map { it.trim().lowercase() }
            ?.filter { it.isNotEmpty() }
            ?.toSet()
            ?: emptySet()
        return parsed.ifEmpty { setOf("admin", "monitor_read") }
    }

    /**
     * 解析布尔类型配置属性
     *
     * Resolves boolean configuration property.
     *
     * 通用的布尔属性解析方法，支持多种布尔值格式。
     *
     * Generic boolean property resolution method,
     * supporting multiple boolean value formats.
     *
     * @param properties 配置属性映射
     *                   Configuration properties map
     * @param key 属性键名
     *            Property key name
     * @param defaultValue 默认值
     *                     Default value
     * @return 解析后的布尔值
     *         Parsed boolean value
     * @throws IllegalArgumentException 值格式无效时抛出
     *                                  Thrown when value format is invalid
     */
    private fun resolveBooleanProperty(
        properties: Map<String, String>,
        key: String,
        defaultValue: Boolean
    ): Boolean {
        val raw = properties[key]?.trim()?.lowercase()
            ?: return defaultValue
        return when (raw) {
            "true", "1", "yes", "y", "on" -> true
            "false", "0", "no", "n", "off" -> false
            else -> throw IllegalArgumentException("Invalid boolean value for '$key': '$raw'")
        }
    }

    /**
     * 解析监控告警服务
     *
     * Resolves monitoring alerting service.
     *
     * 根据配置属性创建监控告警服务实例。
     * 配置检查间隔、阈值、告警路由等参数。
     *
     * Creates monitoring alerting service instance based on config properties.
     * Configures check interval, thresholds, alert routing, etc.
     *
     * @param runtime 远程求解器运行时
     *                 Remote solver runtime
     * @param properties 配置属性
     *                   Configuration properties
     * @return 监控告警服务实例，禁用时返回 null
     *         Monitoring alerting service instance, null when disabled
     * @throws IllegalArgumentException 必要配置缺失时抛出
     *                                  Thrown when required configuration is missing
     */
    private fun resolveMonitorAlerting(
        runtime: RemoteSolverRuntime,
        properties: Map<String, String>
    ): MonitorAlertingService? {
        val enabled = resolveBooleanProperty(
            properties = properties,
            key = "monitor.alert.enabled",
            defaultValue = false
        )
        if (!enabled) {
            return null
        }
        val checkIntervalMs = parseLongProperty(
            properties = properties,
            key = "monitor.alert.check-interval-ms",
            defaultValue = 5_000L,
            minValue = 0L
        )
        val overviewLimit = parseIntProperty(
            properties = properties,
            key = "monitor.alert.overview-limit",
            defaultValue = 300,
            minValue = 1
        )
        val cooldownMs = parseLongProperty(
            properties = properties,
            key = "monitor.alert.cooldown-ms",
            defaultValue = 60_000L,
            minValue = 0L
        )
        val escalateAfterConsecutive = parseIntProperty(
            properties = properties,
            key = "monitor.alert.escalate-after-consecutive",
            defaultValue = 3,
            minValue = 1
        )
        val staleNodesThreshold = parseIntProperty(
            properties = properties,
            key = "monitor.alert.stale-nodes-threshold",
            defaultValue = 1,
            minValue = 0
        )
        val offlineNodesThreshold = parseIntProperty(
            properties = properties,
            key = "monitor.alert.offline-nodes-threshold",
            defaultValue = 1,
            minValue = 0
        )
        val failedTasksThreshold = parseIntProperty(
            properties = properties,
            key = "monitor.alert.failed-tasks-threshold",
            defaultValue = 5,
            minValue = 0
        )
        val failedRatioThreshold = parseDoubleProperty(
            properties = properties,
            key = "monitor.alert.failed-ratio-threshold",
            defaultValue = 0.3,
            minValue = 0.0,
            maxValue = 1.0
        )
        val queueDepthThreshold = parseIntProperty(
            properties = properties,
            key = "monitor.alert.queue-depth-threshold",
            defaultValue = 100,
            minValue = 0
        )
        val routeEventEnabled = resolveBooleanProperty(
            properties = properties,
            key = "monitor.alert.route.event.enabled",
            defaultValue = true
        )
        val routeWebhookEnabled = resolveBooleanProperty(
            properties = properties,
            key = "monitor.alert.route.webhook.enabled",
            defaultValue = false
        )
        val webhookUrl = properties["monitor.alert.route.webhook.url"]
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
        if (routeWebhookEnabled && webhookUrl == null) {
            throw IllegalArgumentException(
                "monitor.alert.route.webhook.url is required when monitor.alert.route.webhook.enabled=true"
            )
        }
        val webhookTimeoutMs = parseLongProperty(
            properties = properties,
            key = "monitor.alert.route.webhook.timeout-ms",
            defaultValue = 3_000L,
            minValue = 100L
        )
        val config = MonitorAlertingConfig(
            enabled = true,
            checkIntervalMs = checkIntervalMs,
            overviewLimit = overviewLimit,
            cooldownMs = cooldownMs,
            escalateAfterConsecutive = escalateAfterConsecutive,
            staleNodesThreshold = staleNodesThreshold,
            offlineNodesThreshold = offlineNodesThreshold,
            failedTasksThreshold = failedTasksThreshold,
            failedRatioThreshold = failedRatioThreshold,
            queueDepthThreshold = queueDepthThreshold,
            routeEventEnabled = routeEventEnabled,
            routeWebhookEnabled = routeWebhookEnabled
        )
        println("monitorAlertCheckIntervalMs=${config.checkIntervalMs}")
        println("monitorAlertCooldownMs=${config.cooldownMs}")
        println("monitorAlertEscalateAfterConsecutive=${config.escalateAfterConsecutive}")
        println("monitorAlertRouteEventEnabled=${config.routeEventEnabled}")
        println("monitorAlertRouteWebhookEnabled=${config.routeWebhookEnabled}")
        if (config.routeWebhookEnabled) {
            println("monitorAlertWebhookUrl=$webhookUrl")
            println("monitorAlertWebhookTimeoutMs=$webhookTimeoutMs")
        }
        val webhookPublisher = if (config.routeWebhookEnabled) {
            HttpMonitorAlertWebhookPublisher(
                url = webhookUrl!!,
                timeoutMs = webhookTimeoutMs
            )
        } else {
            null
        }
        return MonitorAlertingService(
            apiFacade = runtime.apiFacade,
            eventPort = runtime.eventPort,
            clock = runtime.service.clockPort(),
            config = config,
            webhookPublisher = webhookPublisher,
            logger = { message -> println(message) }
        )
    }

    /**
     * 解析长整型配置属性
     *
     * Parses long integer configuration property.
     *
     * 解析配置属性中的长整型值，支持最小值约束。
     *
     * Parses long integer value from config properties,
     * supporting minimum value constraint.
     *
     * @param properties 配置属性映射
     *                   Configuration properties map
     * @param key 属性键名
     *            Property key name
     * @param defaultValue 默认值
     *                     Default value
     * @param minValue 最小值约束
     *                  Minimum value constraint
     * @return 解析后的长整型值
     *         Parsed long integer value
     * @throws IllegalArgumentException 值格式无效时抛出
     *                                  Thrown when value format is invalid
     */
    private fun parseLongProperty(
        properties: Map<String, String>,
        key: String,
        defaultValue: Long,
        minValue: Long
    ): Long {
        val raw = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return defaultValue
        val parsed = raw.toLongOrNull()
            ?: throw IllegalArgumentException("Invalid long value for '$key': '$raw'")
        return parsed.coerceAtLeast(minValue)
    }

    /**
     * 解析整型配置属性
     *
     * Parses integer configuration property.
     *
     * 解析配置属性中的整型值，支持最小值约束。
     *
     * Parses integer value from config properties,
     * supporting minimum value constraint.
     *
     * @param properties 配置属性映射
     *                   Configuration properties map
     * @param key 属性键名
     *            Property key name
     * @param defaultValue 默认值
     *                     Default value
     * @param minValue 最小值约束
     *                  Minimum value constraint
     * @return 解析后的整型值
     *         Parsed integer value
     * @throws IllegalArgumentException 值格式无效时抛出
     *                                  Thrown when value format is invalid
     */
    private fun parseIntProperty(
        properties: Map<String, String>,
        key: String,
        defaultValue: Int,
        minValue: Int
    ): Int {
        val raw = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return defaultValue
        val parsed = raw.toIntOrNull()
            ?: throw IllegalArgumentException("Invalid int value for '$key': '$raw'")
        return parsed.coerceAtLeast(minValue)
    }

    /**
     * 解析双精度浮点型配置属性
     *
     * Parses double precision floating point configuration property.
     *
     * 解析配置属性中的双精度浮点型值，支持范围约束。
     *
     * Parses double precision floating point value from config properties,
     * supporting range constraint.
     *
     * @param properties 配置属性映射
     *                   Configuration properties map
     * @param key 属性键名
     *            Property key name
     * @param defaultValue 默认值
     *                     Default value
     * @param minValue 最小值约束
     *                  Minimum value constraint
     * @param maxValue 最大值约束
     *                  Maximum value constraint
     * @return 解析后的双精度浮点型值
     *         Parsed double precision floating point value
     * @throws IllegalArgumentException 值格式或范围无效时抛出
     *                                  Thrown when value format or range is invalid
     */
    private fun parseDoubleProperty(
        properties: Map<String, String>,
        key: String,
        defaultValue: Double,
        minValue: Double,
        maxValue: Double
    ): Double {
        val raw = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return defaultValue
        val parsed = raw.toDoubleOrNull()
            ?: throw IllegalArgumentException("Invalid double value for '$key': '$raw'")
        if (parsed < minValue || parsed > maxValue) {
            throw IllegalArgumentException(
                "Invalid double value for '$key': '$parsed'. Expected range [$minValue, $maxValue]"
            )
        }
        return parsed
    }
}