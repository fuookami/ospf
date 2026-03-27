/*
 * Event Models
 * 事件模型
 *
 * This file defines the core event data structures for the Remote Solver Dispatcher.
 * 本文件定义了远程求解器调度器的核心事件数据结构。
 *
 * Events are used for:
 * 事件用于：
 * - Recording event history for audit and replay
 * - 记录事件历史以供审计和重放
 * - Wrapping events with envelope metadata for tracing
 * - 使用信封元数据包装事件以支持追踪
 * - Defining standard topic names for event publishing
 * - 定义事件发布的标准主题名称
 */
package fuookami.ospf.framework.remote_solver.domain

/**
 * Event Record
 * 事件记录
 *
 * A persistent record of an event for audit and replay purposes.
 * 用于审计和重放的事件持久化记录。
 *
 * @param eventId Unique identifier for the event.
 *                事件的唯一标识符。
 * @param topic The topic this event was published to.
 *              事件发布到的主题。
 * @param key The partition key for the event.
 *            事件的分区键。
 * @param payload The serialized event payload.
 *                序列化的事件载荷。
 * @param createdAtEpochMs Timestamp when the event was created.
 *                          事件创建时间戳（毫秒）。
 * @param schemaVersion The schema version of the event payload.
 *                       事件载荷的schema版本。
 * @param deliveryAttempt Number of attempts to deliver this event.
 *                         事件投递尝试次数。
 * @param headers Additional metadata headers.
 *                附加元数据头。
 */
data class EventRecord(
    val eventId: String,
    val topic: String,
    val key: String,
    val payload: ByteArray,
    val createdAtEpochMs: Long,
    val schemaVersion: Int = EventSchema.CURRENT_VERSION,
    val deliveryAttempt: Int = 0,
    val headers: Map<String, String> = emptyMap()
)

/**
 * Event Schema
 * 事件Schema
 *
 * Schema version management for event payloads.
 * 事件载荷的Schema版本管理。
 *
 * Provides version validation and header normalization.
 * 提供版本验证和头部规范化功能。
 */
object EventSchema {
    /**
     * Current schema version.
     * 当前Schema版本。
     */
    const val CURRENT_VERSION = 1

    /**
     * Minimum supported schema version.
     * 最小支持的Schema版本。
     */
    const val MIN_SUPPORTED_VERSION = 1

    /**
     * Maximum supported schema version.
     * 最大支持的Schema版本。
     */
    const val MAX_SUPPORTED_VERSION = CURRENT_VERSION

    /**
     * Header name for schema version.
     * Schema版本的头部名称。
     */
    const val HEADER_NAME = "schemaVersion"

    /**
     * Resolves the schema version from headers.
     * 从头部解析Schema版本。
     *
     * @param headers The event headers to parse.
     *                要解析的事件头部。
     * @return The resolved schema version.
     *         解析出的Schema版本。
     * @throws IllegalArgumentException if the version is invalid or unsupported.
     *         如果版本无效或不支持则抛出异常。
     */
    fun resolveSchemaVersion(headers: Map<String, String>): Int {
        val raw = headers[HEADER_NAME]?.trim()
        if (raw.isNullOrBlank()) {
            return CURRENT_VERSION
        }
        val parsed = raw.toIntOrNull()
            ?: throw IllegalArgumentException("Invalid schemaVersion '$raw': must be a positive integer")
        if (parsed <= 0) {
            throw IllegalArgumentException("Invalid schemaVersion '$parsed': must be a positive integer")
        }
        if (parsed < MIN_SUPPORTED_VERSION || parsed > MAX_SUPPORTED_VERSION) {
            throw IllegalArgumentException(
                "Unsupported schemaVersion '$parsed': supported range is [$MIN_SUPPORTED_VERSION, $MAX_SUPPORTED_VERSION]"
            )
        }
        return parsed
    }

    /**
     * Normalizes headers to ensure schema version is present.
     * 规范化头部以确保Schema版本存在。
     *
     * @param headers The headers to normalize.
     *                要规范化的头部。
     * @return Headers with schema version ensured.
     *         确保包含Schema版本的头部。
     */
    fun normalizeHeaders(headers: Map<String, String>): Map<String, String> {
        val schemaVersion = resolveSchemaVersion(headers)
        val current = headers[HEADER_NAME]
        if (current == schemaVersion.toString()) {
            return headers
        }
        return headers + (HEADER_NAME to schemaVersion.toString())
    }

    /**
     * Normalizes envelope headers with default values.
     * 使用默认值规范化信封头部。
     *
     * @param headers The original headers.
     *                原始头部。
     * @param topic The event topic.
     *              事件主题。
     * @param occurredAtEpochMs The event occurrence timestamp.
     *                           事件发生时间戳（毫秒）。
     * @param defaultProducer The default producer identifier.
     *                         默认生产者标识符。
     * @return Normalized headers with all required fields.
     *         包含所有必需字段的规范化头部。
     * @throws IllegalArgumentException if required parameters are invalid.
     *         如果必需参数无效则抛出异常。
     */
    fun normalizeEnvelopeHeaders(
        headers: Map<String, String>,
        topic: String,
        occurredAtEpochMs: Long,
        defaultProducer: String
    ): Map<String, String> {
        require(topic.isNotBlank()) { "topic must not be blank" }
        require(defaultProducer.isNotBlank()) { "defaultProducer must not be blank" }
        require(occurredAtEpochMs > 0L) { "occurredAtEpochMs must be positive" }

        val eventType = headers[EventEnvelopeHeaders.EVENT_TYPE]?.trim()
            ?.takeIf { it.isNotBlank() }
            ?: topic
        val producer = headers[EventEnvelopeHeaders.PRODUCER]?.trim()
            ?.takeIf { it.isNotBlank() }
            ?: defaultProducer
        val occurredAt = headers[EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS]?.trim()
            ?.let {
                it.toLongOrNull()
                    ?: throw IllegalArgumentException(
                        "Invalid occurredAtEpochMs '$it': must be a positive integer"
                    )
            }
            ?: occurredAtEpochMs
        val tenantId = headers[EventEnvelopeHeaders.TENANT_ID]?.trim()
            ?.takeIf { it.isNotBlank() }
            ?: "default"
        if (occurredAt <= 0L) {
            throw IllegalArgumentException("Invalid occurredAtEpochMs '$occurredAt': must be positive")
        }

        var normalized = headers
        normalized = normalized + (EventEnvelopeHeaders.EVENT_TYPE to eventType)
        normalized = normalized + (EventEnvelopeHeaders.PRODUCER to producer)
        normalized = normalized + (EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS to occurredAt.toString())
        normalized = normalized + (EventEnvelopeHeaders.TENANT_ID to tenantId)
        return normalized
    }
}

/**
 * Event Envelope
 * 事件信封
 *
 * A wrapper structure for events with metadata for distributed tracing.
 * 用于分布式追踪的事件包装结构，包含元数据。
 *
 * @param eventId Unique identifier for the event.
 *                事件的唯一标识符。
 * @param eventType The type/classification of the event.
 *                   事件的类型/分类。
 * @param schemaVersion The schema version of the payload.
 *                       载荷的Schema版本。
 * @param occurredAtEpochMs Timestamp when the event occurred.
 *                           事件发生时间戳（毫秒）。
 * @param producer The identifier of the event producer.
 *                 事件生产者标识符。
 * @param traceId Distributed tracing trace ID.
 *                分布式追踪的Trace ID。
 * @param spanId Distributed tracing span ID.
 *               分布式追踪的Span ID。
 * @param payload The serialized event payload.
 *                序列化的事件载荷。
 * @param attributes Additional event attributes.
 *                   附加事件属性。
 */
data class EventEnvelope(
    val eventId: String,
    val eventType: String,
    val schemaVersion: Int,
    val occurredAtEpochMs: Long,
    val producer: String,
    val traceId: String? = null,
    val spanId: String? = null,
    val payload: ByteArray,
    val attributes: Map<String, String> = emptyMap()
)

/**
 * Event Envelope Headers
 * 事件信封头部
 *
 * Standard header names for event envelopes.
 * 事件信封的标准头部名称。
 */
object EventEnvelopeHeaders {
    /**
     * Event type header name.
     * 事件类型头部名称。
     */
    const val EVENT_TYPE = "eventType"

    /**
     * Producer header name.
     * 生产者头部名称。
     */
    const val PRODUCER = "producer"

    /**
     * Trace ID header name.
     * Trace ID头部名称。
     */
    const val TRACE_ID = "traceId"

    /**
     * Span ID header name.
     * Span ID头部名称。
     */
    const val SPAN_ID = "spanId"

    /**
     * Occurred timestamp header name.
     * 发生时间戳头部名称。
     */
    const val OCCURRED_AT_EPOCH_MS = "occurredAtEpochMs"

    /**
     * Tenant ID header name.
     * 租户ID头部名称。
     */
    const val TENANT_ID = "tenantId"
}

/**
 * Event Topics
 * 事件主题
 *
 * Standard topic names for event publishing in the Remote Solver system.
 * 远程求解器系统中事件发布的标准主题名称。
 */
object EventTopics {
    /**
     * Topic for solving requests.
     * 求解请求主题。
     */
    const val SOLVING_REQUEST = "SolvingRequest"

    /**
     * Topic for solving control commands (stop, resume, etc.).
     * 求解控制命令主题（停止、恢复等）。
     */
    const val SOLVING_CONTROL = "SolvingControl"

    /**
     * Topic for task dispatch events.
     * 任务分发事件主题。
     */
    const val TASK_DISPATCH = "TaskDispatch"

    /**
     * Topic for solver heartbeat events.
     * 求解器心跳事件主题。
     */
    const val SOLVER_HEARTBEAT = "SolverHeartBeat"

    /**
     * Topic for slice lifecycle events.
     * Slice生命周期事件主题。
     */
    const val SLICE_LIFECYCLE = "SliceLifecycle"

    /**
     * Topic for task result events.
     * 任务结果事件主题。
     */
    const val TASK_RESULT = "TaskResult"

    /**
     * Topic for cost and capability update events.
     * 成本和能力更新事件主题。
     */
    const val COST_AND_CAPABILITY_UPDATE = "CostAndCapabilityUpdate"

    /**
     * Topic for monitor alert events.
     * 监控告警事件主题。
     */
    const val MONITOR_ALERT = "MonitorAlert"
}