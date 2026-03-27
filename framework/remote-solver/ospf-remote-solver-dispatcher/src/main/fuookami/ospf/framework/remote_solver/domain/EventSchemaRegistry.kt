/*
 * Event Schema Registry
 * 事件Schema注册表
 *
 * This file provides schema validation and registry for event types.
 * 本文件提供事件类型的Schema验证和注册功能。
 *
 * The registry ensures:
 * 注册表确保：
 * - All events have required headers
 * - 所有事件具有必需的头部
 * - Schema versions are supported
 * - Schema版本受支持
 * - Validation can be strict or lenient
 * - 验证可以是严格或宽松模式
 */
package fuookami.ospf.framework.remote_solver.domain

/**
 * Event Schema Validation Mode
 * 事件Schema验证模式
 *
 * Defines the strictness level for schema validation.
 * 定义Schema验证的严格程度级别。
 *
 * - STRICT: Reject events with unknown types or missing headers.
 *   严格模式：拒绝未知类型或缺少头部的事件。
 * - LENIENT: Allow events with unknown types, skip validation.
 *   宽松模式：允许未知类型的事件，跳过验证。
 */
enum class EventSchemaValidationMode {
    STRICT,
    LENIENT;

    companion object {
        /**
         * Parses validation mode from a string key.
         * 从字符串键解析验证模式。
         *
         * @param key The string key to parse.
         *            要解析的字符串键。
         * @return The parsed validation mode, or null if invalid/blank.
         *         解析出的验证模式，如果无效或空白则返回null。
         */
        fun fromKeyOrNull(key: String?): EventSchemaValidationMode? {
            if (key.isNullOrBlank()) {
                return null
            }
            return values().firstOrNull { it.name.equals(key.trim(), ignoreCase = true) }
        }
    }
}

/**
 * Event Schema Definition
 * 事件Schema定义
 *
 * Defines the schema requirements for a specific event type.
 * 定义特定事件类型的Schema要求。
 *
 * @param eventType The event type this schema applies to.
 *                   此Schema适用的事件类型。
 * @param requiredHeaders Set of required header names.
 *                         必需的头部名称集合。
 * @param supportedSchemaVersions Supported schema versions for this event type.
 *                                 此事件类型支持的Schema版本。
 */
data class EventSchemaDefinition(
    val eventType: String,
    val requiredHeaders: Set<String>,
    val supportedSchemaVersions: Set<Int> = setOf(EventSchema.CURRENT_VERSION)
)

/**
 * Event Schema Registry
 * 事件Schema注册表
 *
 * Registry for managing and validating event schemas.
 * 管理和验证事件Schema的注册表。
 *
 * Provides validation for both publish and consume stages.
 * 为发布和消费阶段提供验证。
 *
 * @param definitions Collection of schema definitions to register.
 *                    要注册的Schema定义集合。
 */
class EventSchemaRegistry(
    definitions: Collection<EventSchemaDefinition>
) {
    private val definitionsByEventType = definitions.associateBy { it.eventType }

    /**
     * Validates an event for publishing.
     * 验证事件以供发布。
     *
     * @param topic The event topic.
     *              事件主题。
     * @param headers The event headers.
     *                事件头部。
     * @param schemaVersion The schema version.
     *                       Schema版本。
     * @param mode The validation mode.
     *             验证模式。
     * @throws IllegalArgumentException if validation fails in strict mode.
     *         如果在严格模式下验证失败则抛出异常。
     */
    fun validateForPublish(
        topic: String,
        headers: Map<String, String>,
        schemaVersion: Int,
        mode: EventSchemaValidationMode
    ) {
        validate(
            stage = "publish",
            topic = topic,
            headers = headers,
            schemaVersion = schemaVersion,
            mode = mode
        )
    }

    /**
     * Validates an event for consumption.
     * 验证事件以供消费。
     *
     * @param topic The event topic.
     *              事件主题。
     * @param headers The event headers.
     *                事件头部。
     * @param schemaVersion The schema version.
     *                       Schema版本。
     * @param mode The validation mode.
     *             验证模式。
     * @throws IllegalArgumentException if validation fails in strict mode.
     *         如果在严格模式下验证失败则抛出异常。
     */
    fun validateForConsume(
        topic: String,
        headers: Map<String, String>,
        schemaVersion: Int,
        mode: EventSchemaValidationMode
    ) {
        validate(
            stage = "consume",
            topic = topic,
            headers = headers,
            schemaVersion = schemaVersion,
            mode = mode
        )
    }

    /**
     * Internal validation logic.
     * 内部验证逻辑。
     *
     * @param stage The validation stage (publish or consume).
     *              验证阶段（发布或消费）。
     * @param topic The event topic.
     *              事件主题。
     * @param headers The event headers.
     *                事件头部。
     * @param schemaVersion The schema version.
     *                       Schema版本。
     * @param mode The validation mode.
     *             验证模式。
     * @throws IllegalArgumentException if validation fails.
     *         如果验证失败则抛出异常。
     */
    private fun validate(
        stage: String,
        topic: String,
        headers: Map<String, String>,
        schemaVersion: Int,
        mode: EventSchemaValidationMode
    ) {
        val eventType = headers[EventEnvelopeHeaders.EVENT_TYPE]?.trim()
            ?.takeIf { it.isNotBlank() }
            ?: topic
        val definition = definitionsByEventType[eventType]
        if (definition == null) {
            if (mode == EventSchemaValidationMode.STRICT) {
                throw IllegalArgumentException(
                    "Unsupported eventType '$eventType' for $stage. " +
                        "Registered: ${definitionsByEventType.keys.sorted().joinToString(", ")}"
                )
            }
            return
        }
        definition.requiredHeaders.forEach { headerName ->
            val value = headers[headerName]?.trim()
            if (value.isNullOrEmpty()) {
                throw IllegalArgumentException(
                    "Missing required header '$headerName' for eventType '$eventType' on $stage"
                )
            }
        }
        if (!definition.supportedSchemaVersions.contains(schemaVersion)) {
            throw IllegalArgumentException(
                "Unsupported schemaVersion '$schemaVersion' for eventType '$eventType' on $stage. " +
                    "Supported: ${definition.supportedSchemaVersions.sorted().joinToString(", ")}"
            )
        }
    }

    companion object {
        /**
         * Default required headers for all event types.
         * 所有事件类型的默认必需头部。
         */
        private val defaultRequiredHeaders = setOf(
            EventSchema.HEADER_NAME,
            EventEnvelopeHeaders.EVENT_TYPE,
            EventEnvelopeHeaders.PRODUCER,
            EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS,
            EventEnvelopeHeaders.TENANT_ID
        )

        /**
         * Creates a default registry with standard event types.
         * 创建包含标准事件类型的默认注册表。
         *
         * @return Default EventSchemaRegistry instance.
         *         默认的EventSchemaRegistry实例。
         */
        fun default(): EventSchemaRegistry =
            EventSchemaRegistry(
                listOf(
                    EventTopics.SOLVING_REQUEST,
                    EventTopics.SOLVING_CONTROL,
                    EventTopics.TASK_DISPATCH,
                    EventTopics.SOLVER_HEARTBEAT,
                    EventTopics.SLICE_LIFECYCLE,
                    EventTopics.TASK_RESULT,
                    EventTopics.COST_AND_CAPABILITY_UPDATE,
                    EventTopics.MONITOR_ALERT
                ).map { eventType ->
                    EventSchemaDefinition(
                        eventType = eventType,
                        requiredHeaders = defaultRequiredHeaders
                    )
                }
            )
    }
}