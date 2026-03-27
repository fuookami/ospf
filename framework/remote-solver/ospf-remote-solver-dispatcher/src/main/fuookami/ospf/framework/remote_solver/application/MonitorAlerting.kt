/*
 * 监控告警服务
 *
 * 本模块提供监控告警功能，
 * 用于检测系统异常并触发告警通知。
 * 支持事件路由和 Webhook 推送两种告警方式。
 *
 * Monitor Alerting Service
 *
 * This module provides monitoring alerting functionality,
 * used to detect system anomalies and trigger alert notifications.
 * Supports two alerting methods: event routing and webhook push.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.EventTopics
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.EventPort
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.charset.StandardCharsets
import java.time.Duration
import java.util.Locale
import kotlin.math.max
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * 监控告警配置
 *
 * Monitor alerting configuration.
 *
 * @param enabled 是否启用告警，默认为 false
 *                Whether alerting is enabled, defaults to false
 * @param checkIntervalMs 检查间隔时间（毫秒），默认为 5000ms
 *                         Check interval in milliseconds, defaults to 5000ms
 * @param overviewLimit 监控概览限制，默认为 300
 *                       Monitoring overview limit, defaults to 300
 * @param cooldownMs 告警冷却时间（毫秒），默认为 60000ms
 *                   Alert cooldown period in milliseconds, defaults to 60000ms
 * @param escalateAfterConsecutive 连续触发后升级次数，默认为 3
 *                                  Consecutive triggers before escalation, defaults to 3
 * @param staleNodesThreshold 停滞节点阈值，默认为 1
 *                             Stale nodes threshold, defaults to 1
 * @param offlineNodesThreshold 离线节点阈值，默认为 1
 *                               Offline nodes threshold, defaults to 1
 * @param failedTasksThreshold 失败任务阈值，默认为 5
 *                              Failed tasks threshold, defaults to 5
 * @param failedRatioThreshold 失败比例阈值，默认为 0.3
 *                              Failed ratio threshold, defaults to 0.3
 * @param queueDepthThreshold 队列深度阈值，默认为 100
 *                             Queue depth threshold, defaults to 100
 * @param routeEventEnabled 是否启用事件路由，默认为 true
 *                           Whether event routing is enabled, defaults to true
 * @param routeWebhookEnabled 是否启用 Webhook 推送，默认为 false
 *                             Whether webhook push is enabled, defaults to false
 */
data class MonitorAlertingConfig(
    val enabled: Boolean = false,
    val checkIntervalMs: Long = 5000L,
    val overviewLimit: Int = 300,
    val cooldownMs: Long = 60_000L,
    val escalateAfterConsecutive: Int = 3,
    val staleNodesThreshold: Int = 1,
    val offlineNodesThreshold: Int = 1,
    val failedTasksThreshold: Int = 5,
    val failedRatioThreshold: Double = 0.3,
    val queueDepthThreshold: Int = 100,
    val routeEventEnabled: Boolean = true,
    val routeWebhookEnabled: Boolean = false
)

/**
 * 监控告警状态
 *
 * Monitor alert state.
 */
enum class MonitorAlertState {
    TRIGGERED,
    ESCALATED,
    RECOVERED
}

/**
 * 监控告警严重程度
 *
 * Monitor alert severity.
 */
enum class MonitorAlertSeverity {
    WARNING,
    CRITICAL
}

/**
 * 监控告警通知
 *
 * 可序列化的告警通知数据结构。
 *
 * Monitor alert notification.
 *
 * Serializable alert notification data structure.
 *
 * @param ruleKey 规则键
 *                Rule key
 * @param state 告警状态
 *              Alert state
 * @param severity 严重程度
 *                  Severity
 * @param message 告警消息
 *                Alert message
 * @param occurredAtEpochMs 发生时间戳（毫秒）
 *                          Occurred timestamp in milliseconds
 * @param context 上下文信息键值对
 *                Context information key-value pairs
 */
@Serializable
data class MonitorAlertNotification(
    val ruleKey: String,
    val state: String,
    val severity: String,
    val message: String,
    val occurredAtEpochMs: Long,
    val context: Map<String, String> = emptyMap()
)

/**
 * 监控告警 Webhook 发布器接口
 *
 * Monitor alert webhook publisher interface.
 */
interface MonitorAlertWebhookPublisher {
    /**
     * 发布告警通知
     *
     * Publishes alert notification.
     *
     * @param notification 告警通知
     *                     Alert notification
     */
    suspend fun publish(notification: MonitorAlertNotification)
}

/**
 * HTTP 监控告警 Webhook 发布器
 *
 * 通过 HTTP POST 方式推送告警通知的实现。
 *
 * HTTP monitor alert webhook publisher.
 *
 * Implementation that pushes alert notifications via HTTP POST.
 *
 * @param url Webhook URL
 *            Webhook URL
 * @param timeoutMs 超时时间（毫秒），默认为 3000ms
 *                  Timeout in milliseconds, defaults to 3000ms
 * @param json JSON 序列化配置
 *             JSON serialization configuration
 * @param client HTTP 客户端
 *               HTTP client
 */
class HttpMonitorAlertWebhookPublisher(
    url: String,
    timeoutMs: Long = 3000L,
    private val json: Json = Json { encodeDefaults = true },
    private val client: HttpClient = HttpClient.newBuilder()
        .connectTimeout(Duration.ofMillis(timeoutMs.coerceAtLeast(100L)))
        .build()
) : MonitorAlertWebhookPublisher {
    private val endpoint = URI.create(url.trim())
    private val requestTimeout = Duration.ofMillis(timeoutMs.coerceAtLeast(100L))

    override suspend fun publish(notification: MonitorAlertNotification) {
        val payload = json.encodeToString(notification)
        val request = HttpRequest.newBuilder(endpoint)
            .timeout(requestTimeout)
            .header("Content-Type", "application/json; charset=utf-8")
            .POST(HttpRequest.BodyPublishers.ofString(payload, StandardCharsets.UTF_8))
            .build()
        val response = client.send(request, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8))
        if (response.statusCode() !in 200..299) {
            throw IllegalStateException(
                "webhook status=${response.statusCode()}, body=${response.body().take(512)}"
            )
        }
    }
}

/**
 * 监控告警服务
 *
 * 用于检测系统异常并触发告警通知的核心服务。
 * 支持告警升级、冷却和恢复机制。
 *
 * Monitor alerting service.
 *
 * Core service for detecting system anomalies and triggering alert notifications.
 * Supports alert escalation, cooldown, and recovery mechanisms.
 *
 * @param apiFacade 远程求解器 API 门面，用于获取监控数据
 *                   Remote solver API facade for getting monitoring data
 * @param eventPort 事件端口，用于发布告警事件
 *                   Event port for publishing alert events
 * @param clock 时钟端口，用于获取当前时间
 *              Clock port for getting current time
 * @param config 告警配置
 *               Alerting configuration
 * @param webhookPublisher Webhook 发布器（可选）
 *                         Webhook publisher (optional)
 * @param logger 日志记录函数
 *               Logger function
 */
class MonitorAlertingService(
    private val apiFacade: RemoteSolverApiFacade,
    private val eventPort: EventPort,
    private val clock: ClockPort,
    config: MonitorAlertingConfig,
    private val webhookPublisher: MonitorAlertWebhookPublisher? = null,
    private val logger: (String) -> Unit = {}
) {
    private data class ActiveRuleState(
        var baseSeverity: MonitorAlertSeverity,
        var consecutiveActiveRounds: Int,
        var escalationLevel: Int,
        var lastNotifiedAtEpochMs: Long
    )

    private data class RuleActivation(
        val ruleKey: String,
        val severity: MonitorAlertSeverity,
        val message: String,
        val context: Map<String, String>
    )

    private val normalizedConfig = config.copy(
        checkIntervalMs = config.checkIntervalMs.coerceAtLeast(0L),
        overviewLimit = config.overviewLimit.coerceIn(1, 1000),
        cooldownMs = config.cooldownMs.coerceAtLeast(0L),
        escalateAfterConsecutive = config.escalateAfterConsecutive.coerceAtLeast(1),
        staleNodesThreshold = config.staleNodesThreshold.coerceAtLeast(0),
        offlineNodesThreshold = config.offlineNodesThreshold.coerceAtLeast(0),
        failedTasksThreshold = config.failedTasksThreshold.coerceAtLeast(0),
        failedRatioThreshold = config.failedRatioThreshold.coerceIn(0.0, 1.0),
        queueDepthThreshold = config.queueDepthThreshold.coerceAtLeast(0)
    )
    private val json = Json { encodeDefaults = true }
    private val activeStateByRuleKey = LinkedHashMap<String, ActiveRuleState>()
    private var lastCheckedAtEpochMs: Long = Long.MIN_VALUE

    /**
     * 如果到期则处理告警检查
     *
     * 根据配置的检查间隔，如果到期则执行告警检查。
     *
     * Processes alert check if due.
     *
     * Executes alert check based on configured check interval if due.
     */
    suspend fun processIfDue() {
        if (!normalizedConfig.enabled) {
            return
        }
        val now = clock.nowEpochMs()
        if (
            lastCheckedAtEpochMs != Long.MIN_VALUE &&
            now - lastCheckedAtEpochMs < normalizedConfig.checkIntervalMs
        ) {
            return
        }
        lastCheckedAtEpochMs = now
        processAt(now)
    }

    /**
     * 立即处理告警检查
     *
     * Processes alert check immediately.
     */
    suspend fun processNow() {
        if (!normalizedConfig.enabled) {
            return
        }
        processAt(clock.nowEpochMs())
    }

    private suspend fun processAt(now: Long) {
        val overview = apiFacade.monitorOverview(limitRecentTasks = normalizedConfig.overviewLimit)
        val activeRules = evaluate(overview)
        val activeByKey = activeRules.associateBy { it.ruleKey }
        val recoveredKeys = activeStateByRuleKey.keys.filter { !activeByKey.containsKey(it) }
        recoveredKeys.forEach { key ->
            val previous = activeStateByRuleKey.remove(key) ?: return@forEach
            val severity = escalateSeverity(previous.baseSeverity, previous.escalationLevel)
            val notification = MonitorAlertNotification(
                ruleKey = key,
                state = MonitorAlertState.RECOVERED.name,
                severity = severity.name,
                message = "alert recovered",
                occurredAtEpochMs = now,
                context = mapOf(
                    "escalationLevel" to previous.escalationLevel.toString(),
                    "consecutiveActiveRounds" to previous.consecutiveActiveRounds.toString()
                )
            )
            route(notification)
        }
        activeRules.forEach { rule ->
            val current = activeStateByRuleKey[rule.ruleKey]
            if (current == null) {
                activeStateByRuleKey[rule.ruleKey] = ActiveRuleState(
                    baseSeverity = rule.severity,
                    consecutiveActiveRounds = 1,
                    escalationLevel = 0,
                    lastNotifiedAtEpochMs = now
                )
                route(
                    MonitorAlertNotification(
                        ruleKey = rule.ruleKey,
                        state = MonitorAlertState.TRIGGERED.name,
                        severity = rule.severity.name,
                        message = rule.message,
                        occurredAtEpochMs = now,
                        context = rule.context
                    )
                )
                return@forEach
            }
            current.baseSeverity = rule.severity
            current.consecutiveActiveRounds += 1
            val shouldEscalate = current.consecutiveActiveRounds >= normalizedConfig.escalateAfterConsecutive &&
                current.consecutiveActiveRounds % normalizedConfig.escalateAfterConsecutive == 0
            val cooldownReached = now - current.lastNotifiedAtEpochMs >= normalizedConfig.cooldownMs
            if (shouldEscalate && cooldownReached) {
                current.escalationLevel += 1
                current.lastNotifiedAtEpochMs = now
                route(
                    MonitorAlertNotification(
                        ruleKey = rule.ruleKey,
                        state = MonitorAlertState.ESCALATED.name,
                        severity = escalateSeverity(rule.severity, current.escalationLevel).name,
                        message = rule.message,
                        occurredAtEpochMs = now,
                        context = rule.context + mapOf(
                            "escalationLevel" to current.escalationLevel.toString(),
                            "consecutiveActiveRounds" to current.consecutiveActiveRounds.toString()
                        )
                    )
                )
            }
        }
    }

    private fun evaluate(overview: MonitoringOverviewResponse): List<RuleActivation> {
        val activations = mutableListOf<RuleActivation>()
        val staleNodes = (overview.nodeTotals["staleNodes"] ?: 0).coerceAtLeast(0)
        if (normalizedConfig.staleNodesThreshold > 0 && staleNodes >= normalizedConfig.staleNodesThreshold) {
            activations.add(
                RuleActivation(
                    ruleKey = "stale_nodes",
                    severity = MonitorAlertSeverity.WARNING,
                    message = "stale nodes reached threshold",
                    context = mapOf(
                        "staleNodes" to staleNodes.toString(),
                        "threshold" to normalizedConfig.staleNodesThreshold.toString()
                    )
                )
            )
        }
        val offlineNodes = (overview.nodeTotals["offlineNodes"] ?: 0).coerceAtLeast(0)
        if (normalizedConfig.offlineNodesThreshold > 0 && offlineNodes >= normalizedConfig.offlineNodesThreshold) {
            activations.add(
                RuleActivation(
                    ruleKey = "offline_nodes",
                    severity = MonitorAlertSeverity.CRITICAL,
                    message = "offline nodes reached threshold",
                    context = mapOf(
                        "offlineNodes" to offlineNodes.toString(),
                        "threshold" to normalizedConfig.offlineNodesThreshold.toString()
                    )
                )
            )
        }
        val failedTasks = overview.tasks.failedTasks.coerceAtLeast(0)
        val observedTasks = max(1, overview.tasks.totalObservedTasks)
        val failedRatio = failedTasks.toDouble() / observedTasks.toDouble()
        if (
            normalizedConfig.failedTasksThreshold > 0 &&
            failedTasks >= normalizedConfig.failedTasksThreshold &&
            failedRatio >= normalizedConfig.failedRatioThreshold
        ) {
            activations.add(
                RuleActivation(
                    ruleKey = "failed_task_ratio",
                    severity = MonitorAlertSeverity.CRITICAL,
                    message = "failed task ratio reached threshold",
                    context = mapOf(
                        "failedTasks" to failedTasks.toString(),
                        "totalObservedTasks" to observedTasks.toString(),
                        "failedRatio" to String.format(Locale.US, "%.4f", failedRatio),
                        "failedTasksThreshold" to normalizedConfig.failedTasksThreshold.toString(),
                        "failedRatioThreshold" to String.format(Locale.US, "%.4f", normalizedConfig.failedRatioThreshold)
                    )
                )
            )
        }
        val queueDepth = overview.tasks.queueDepth.coerceAtLeast(0)
        if (normalizedConfig.queueDepthThreshold > 0 && queueDepth >= normalizedConfig.queueDepthThreshold) {
            activations.add(
                RuleActivation(
                    ruleKey = "queue_depth",
                    severity = MonitorAlertSeverity.WARNING,
                    message = "queue depth reached threshold",
                    context = mapOf(
                        "queueDepth" to queueDepth.toString(),
                        "threshold" to normalizedConfig.queueDepthThreshold.toString()
                    )
                )
            )
        }
        return activations
    }

    private suspend fun route(notification: MonitorAlertNotification) {
        if (normalizedConfig.routeEventEnabled) {
            runCatching {
                val cooldownBucket = max(1L, normalizedConfig.cooldownMs)
                val idempotencyWindow = notification.occurredAtEpochMs / cooldownBucket
                eventPort.publish(
                    topic = EventTopics.MONITOR_ALERT,
                    key = notification.ruleKey,
                    payload = json.encodeToString(notification).toByteArray(StandardCharsets.UTF_8),
                    headers = mapOf(
                        "idempotencyKey" to "monitor-alert:${notification.ruleKey}:${notification.state}:$idempotencyWindow",
                        "monitorAlertState" to notification.state,
                        "monitorAlertSeverity" to notification.severity
                    )
                )
            }.onFailure { throwable ->
                logger("monitor alert event route failed: ${throwable.message}")
            }
        }
        val publisher = webhookPublisher
        if (normalizedConfig.routeWebhookEnabled && publisher != null) {
            runCatching {
                publisher.publish(notification)
            }.onFailure { throwable ->
                logger("monitor alert webhook route failed: ${throwable.message}")
            }
        }
    }

    private fun escalateSeverity(base: MonitorAlertSeverity, escalationLevel: Int): MonitorAlertSeverity {
        if (escalationLevel <= 0) {
            return base
        }
        return when (base) {
            MonitorAlertSeverity.WARNING -> MonitorAlertSeverity.CRITICAL
            MonitorAlertSeverity.CRITICAL -> MonitorAlertSeverity.CRITICAL
        }
    }
}
