/*
 * Kafka 事件端口适配器
 *
 * Kafka Event Port Adapter
 *
 * 该模块提供基于 Apache Kafka 的事件发布和订阅实现，是远程求解器系统的核心事件基础设施。
 * 支持幂等发布、延迟重试、事件溯源查询和 Schema 验证等功能。
 * This module provides Apache Kafka-based event publishing and subscription implementation,
 * which is the core event infrastructure of the remote solver system.
 * Supports idempotent publishing, delayed retry, event sourcing query and schema validation.
 */

package fuookami.ospf.framework.remote_solver.adapter.kafka

import fuookami.ospf.framework.remote_solver.domain.EventRecord
import fuookami.ospf.framework.remote_solver.domain.EventSchema
import fuookami.ospf.framework.remote_solver.domain.EventSchemaRegistry
import fuookami.ospf.framework.remote_solver.domain.EventSchemaValidationMode
import fuookami.ospf.framework.remote_solver.domain.EventTopics
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.EventPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.port.TaskEventQueryPort
import org.apache.kafka.clients.consumer.ConsumerConfig
import org.apache.kafka.clients.consumer.ConsumerRecord
import org.apache.kafka.clients.consumer.KafkaConsumer
import org.apache.kafka.clients.producer.KafkaProducer
import org.apache.kafka.clients.producer.ProducerConfig
import org.apache.kafka.clients.producer.ProducerRecord
import org.apache.kafka.common.TopicPartition
import org.apache.kafka.common.header.internals.RecordHeader
import org.apache.kafka.common.serialization.ByteArrayDeserializer
import org.apache.kafka.common.serialization.ByteArraySerializer
import org.apache.kafka.common.serialization.StringDeserializer
import org.apache.kafka.common.serialization.StringSerializer
import java.nio.charset.StandardCharsets
import java.time.Duration
import java.util.Properties
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.coroutines.Continuation
import kotlin.coroutines.EmptyCoroutineContext
import kotlin.coroutines.startCoroutine

/**
 * Kafka 事件端口
 *
 * Kafka Event Port
 *
 * 该类实现了 [EventPort] 和 [TaskEventQueryPort] 接口，使用 Apache Kafka 提供可靠的
 * 事件发布和订阅服务。主要特性包括：
 * This class implements [EventPort] and [TaskEventQueryPort] interfaces, using Apache Kafka
 * to provide reliable event publishing and subscription services. Main features include:
 *
 * - **幂等发布**: 通过幂等键去重，防止重复发布相同事件。
 *   Idempotent publishing: Deduplication via idempotency key, preventing duplicate publishing of same event.
 * - **延迟重试**: 支持配置延迟时间后重新发布失败的事件。
 *   Delayed retry: Supports republishing failed events after configured delay time.
 * - **事件溯源查询**: 可从 Kafka 和本地日志中查询任务相关的事件历史。
 *   Event sourcing query: Can query task-related event history from Kafka and local log.
 * - **Schema 验证**: 支持事件 Schema 验证，确保事件格式符合规范。
 *   Schema validation: Supports event schema validation, ensuring event format conforms to specification.
 *
 * @param clock 时钟端口，用于生成时间戳。
 *              Clock port for generating timestamps.
 * @param idGenerator ID 生成器端口，用于生成事件 ID 和订阅 ID。
 *                     ID generator port for generating event IDs and subscription IDs.
 * @param bootstrapServers Kafka broker 地址列表，格式为 "host1:port1,host2:port2"。
 *                          Kafka broker address list, format is "host1:port1,host2:port2".
 * @param clientId Kafka 客户端 ID 标识。
 *                  Kafka client ID identifier.
 * @param consumerPollIntervalMs 消费者轮询间隔（毫秒），用于订阅处理线程。
 *                                Consumer poll interval in milliseconds, for subscription handler thread.
 * @param queryPollTimeoutMs 查询消费者轮询超时（毫秒），用于事件溯源查询。
 *                            Query consumer poll timeout in milliseconds, for event sourcing query.
 * @param queryMaxPollRounds 查询最大轮询轮数，限制查询时间。
 *                            Query maximum poll rounds, limiting query time.
 * @param queryTopics 查询的主题集合，用于事件溯源查询。
 *                    Topics set for query, used for event sourcing query.
 * @param producerName 生产者名称，用于事件头中的 producer 字段。
 *                      Producer name, used for producer field in event headers.
 * @param schemaRegistry 事件 Schema 注册表，用于 Schema 验证。
 *                        Event schema registry, used for schema validation.
 * @param schemaValidationMode Schema 验证模式（严格或宽松）。
 *                              Schema validation mode (strict or lenient).
 */
class KafkaEventPort(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val bootstrapServers: String,
    clientId: String = "remote-solver-event-port",
    private val consumerPollIntervalMs: Long = 200L,
    private val queryPollTimeoutMs: Long = 1500L,
    private val queryMaxPollRounds: Int = 8,
    private val queryTopics: Set<String> = defaultQueryTopics(),
    private val producerName: String = "kafka-event-port",
    private val schemaRegistry: EventSchemaRegistry = EventSchemaRegistry.default(),
    private val schemaValidationMode: EventSchemaValidationMode = EventSchemaValidationMode.LENIENT
) : EventPort, TaskEventQueryPort {

    /**
     * 订阅信息
     *
     * Subscription Information
     *
     * 内部数据类，用于跟踪活跃的事件订阅。
     * Internal data class for tracking active event subscriptions.
     *
     * @param id 订阅标识。
     *            Subscription identifier.
     * @param topic 订阅的主题。
     *              Subscribed topic.
     * @param consumerGroup 消费者组标识。
     *                      Consumer group identifier.
     * @param handler 事件处理函数。
     *                Event handler function.
     * @param running 运行状态标志，用于控制订阅线程的生命周期。
     *                Running status flag, for controlling subscription thread lifecycle.
     * @param worker 订阅处理线程。
     *                Subscription handler thread.
     */
    private data class Subscription(
        val id: String,
        val topic: String,
        val consumerGroup: String,
        val handler: suspend (EventRecord) -> Unit,
        val running: AtomicBoolean,
        val worker: Thread
    )

    private val producer: KafkaProducer<String, ByteArray>
    private val retryScheduler: ScheduledExecutorService = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "kafka-event-port-retry").apply { isDaemon = true }
    }
    private val subscriptions = ConcurrentHashMap<String, Subscription>()
    private val acknowledgedByEventId = ConcurrentHashMap<String, Boolean>()
    private val publishedByIdempotencyKey = ConcurrentHashMap<String, EventRecord>()
    private val eventLog = java.util.concurrent.CopyOnWriteArrayList<EventRecord>()

    init {
        require(bootstrapServers.isNotBlank()) { "bootstrapServers must not be blank" }
        val producerProperties = Properties()
        producerProperties[ProducerConfig.BOOTSTRAP_SERVERS_CONFIG] = bootstrapServers
        producerProperties[ProducerConfig.CLIENT_ID_CONFIG] = clientId
        producerProperties[ProducerConfig.KEY_SERIALIZER_CLASS_CONFIG] = StringSerializer::class.java.name
        producerProperties[ProducerConfig.VALUE_SERIALIZER_CLASS_CONFIG] = ByteArraySerializer::class.java.name
        producerProperties[ProducerConfig.ACKS_CONFIG] = "all"
        producer = KafkaProducer(producerProperties)
    }

    /**
     * 发布事件
     *
     * Publish event
     *
     * 将事件发布到 Kafka，支持幂等发布以防止重复。事件头会自动规范化，
     * 包含生产者、时间戳、Schema 版本等信息。同时将事件记录保存到本地日志，
     * 以支持后续的事件溯源查询。
     *
     * Publishes event to Kafka, supporting idempotent publishing to prevent duplicates.
     * Event headers are automatically normalized, containing producer, timestamp,
     * schema version and other information. Also saves event record to local log
     * for subsequent event sourcing query support.
     *
     * @param topic 事件主题，对应 Kafka topic。
     *               Event topic, corresponding to Kafka topic.
     * @param key 事件键，用于 Kafka 分区路由。
     *             Event key, for Kafka partition routing.
     * @param payload 事件数据字节数组。
     *                 Event data byte array.
     * @param headers 事件头信息映射。
     *                 Event header information mapping.
     * @return 发布的事件记录，包含事件 ID、时间戳等信息。
     *         Published event record, containing event ID, timestamp and other information.
     */
    override suspend fun publish(
        topic: String,
        key: String,
        payload: ByteArray,
        headers: Map<String, String>
    ): EventRecord {
        val createdAtEpochMs = clock.nowEpochMs()
        val normalizedHeaders = EventSchema.normalizeEnvelopeHeaders(
            headers = EventSchema.normalizeHeaders(headers),
            topic = topic,
            occurredAtEpochMs = createdAtEpochMs,
            defaultProducer = producerName
        )
        val schemaVersion = EventSchema.resolveSchemaVersion(normalizedHeaders)
        schemaRegistry.validateForPublish(
            topic = topic,
            headers = normalizedHeaders,
            schemaVersion = schemaVersion,
            mode = schemaValidationMode
        )
        val idempotencyKey = normalizedHeaders["idempotencyKey"]
        if (!idempotencyKey.isNullOrBlank()) {
            val dedupeKey = "$topic|$key|$idempotencyKey"
            val deduped = synchronized(publishedByIdempotencyKey) {
                publishedByIdempotencyKey[dedupeKey]?.let { return it }
                val created = newRecord(
                    topic = topic,
                    key = key,
                    payload = payload,
                    schemaVersion = schemaVersion,
                    headers = normalizedHeaders,
                    deliveryAttempt = 0,
                    createdAtEpochMs = createdAtEpochMs
                )
                publishedByIdempotencyKey[dedupeKey] = created
                created
            }
            sendToKafka(deduped)
            eventLog.add(deduped)
            return deduped
        }
        val record = newRecord(
            topic = topic,
            key = key,
            payload = payload,
            schemaVersion = schemaVersion,
            headers = normalizedHeaders,
            deliveryAttempt = 0,
            createdAtEpochMs = createdAtEpochMs
        )
        sendToKafka(record)
        eventLog.add(record)
        return record
    }

    /**
     * 订阅事件
     *
     * Subscribe to events
     *
     * 创建一个后台线程订阅指定 Kafka topic 的事件，并将事件传递给处理函数。
     * 每个订阅有独立的消费者实例和线程，支持多订阅并行消费。
     *
     * Creates a background thread to subscribe to events of specified Kafka topic,
     * and passes events to handler function. Each subscription has independent
     * consumer instance and thread, supporting parallel consumption of multiple subscriptions.
     *
     * @param topic 要订阅的事件主题。
     *               Event topic to subscribe.
     * @param consumerGroup 消费者组标识，同一组内的消费者共享消费进度。
     *                       Consumer group identifier, consumers in same group share consumption progress.
     * @param handler 事件处理函数，接收事件记录并处理。处理成功后应调用 [ack]。
     *                 Event handler function, receives event record and processes it.
     *                 Should call [ack] after successful processing.
     * @return 订阅标识，用于后续取消订阅。
     *         Subscription identifier for later unsubscribe.
     */
    override suspend fun subscribe(
        topic: String,
        consumerGroup: String,
        handler: suspend (EventRecord) -> Unit
    ): String {
        val subscriptionId = idGenerator.newId("sub")
        val running = AtomicBoolean(true)
        val worker = Thread(
            {
                val consumer = createConsumer(consumerGroup = consumerGroup, subscriptionId = subscriptionId)
                try {
                    consumer.subscribe(listOf(topic))
                    while (running.get()) {
                        val records = consumer.poll(Duration.ofMillis(consumerPollIntervalMs.coerceAtLeast(50L)))
                        for (record in records) {
                            if (!running.get()) {
                                break
                            }
                            val event = fromKafkaRecord(record)
                            if (acknowledgedByEventId[event.eventId] == true) {
                                continue
                            }
                            try {
                                runSuspendBlocking {
                                    handler(event)
                                }
                            } catch (_: Exception) {
                            }
                        }
                    }
                } finally {
                    consumer.close()
                }
            },
            "kafka-event-sub-$subscriptionId"
        )
        worker.isDaemon = true
        val subscription = Subscription(
            id = subscriptionId,
            topic = topic,
            consumerGroup = consumerGroup,
            handler = handler,
            running = running,
            worker = worker
        )
        subscriptions[subscriptionId] = subscription
        worker.start()
        return subscriptionId
    }

    /**
     * 取消订阅
     *
     * Unsubscribe
     *
     * 停止指定订阅的消费线程并清理资源。线程会在当前轮询周期结束后优雅退出。
     * Stops consumption thread of specified subscription and cleans up resources.
     * Thread gracefully exits after current poll cycle ends.
     *
     * @param subscriptionId 订阅标识，由 [subscribe] 返回。
     *                         Subscription identifier returned by [subscribe].
     * @return 是否成功取消订阅。如果订阅不存在则返回 false。
     *         Whether unsubscribe was successful. Returns false if subscription doesn't exist.
     */
    override suspend fun unsubscribe(subscriptionId: String): Boolean {
        val subscription = subscriptions.remove(subscriptionId) ?: return false
        subscription.running.set(false)
        subscription.worker.join(consumerPollIntervalMs.coerceAtLeast(50L) + 200L)
        return true
    }

    /**
     * 确认事件处理成功
     *
     * Acknowledge event processing success
     *
     * 将事件标记为已确认，防止后续重复处理。已确认的事件会被后续轮询跳过。
     * Marks event as acknowledged, preventing subsequent duplicate processing.
     * Acknowledged events are skipped in subsequent polls.
     *
     * @param record 已成功处理的事件记录。
     *               Successfully processed event record.
     */
    override suspend fun ack(record: EventRecord) {
        acknowledgedByEventId[record.eventId] = true
    }

    /**
     * 否定确认事件，请求重新处理
     *
     * Negatively acknowledge event, request reprocessing
     *
     * 将事件重新发布到 Kafka 以触发重新处理。可以指定延迟时间，
     * 支持延迟重试场景。事件头中会增加投递尝试次数计数。
     *
     * Republishes event to Kafka to trigger reprocessing. Can specify delay time,
     * supporting delayed retry scenario. Delivery attempt count is incremented in event headers.
     *
     * @param record 需要重新处理的事件记录。
     *               Event record that needs reprocessing.
     * @param retryAtEpochMs 重试时间戳（毫秒），null 表示立即重试。
     *                        Retry timestamp in milliseconds, null means immediate retry.
     */
    override suspend fun nack(record: EventRecord, retryAtEpochMs: Long?) {
        val retryAttempt = record.deliveryAttempt + 1
        val headers = record.headers + mapOf("deliveryAttempt" to retryAttempt.toString())
        val retryRecord = record.copy(
            deliveryAttempt = retryAttempt,
            headers = headers
        )
        val delayMs = retryAtEpochMs?.let { it - clock.nowEpochMs() } ?: 0L
        if (delayMs <= 0L) {
            sendToKafka(retryRecord)
            return
        }
        retryScheduler.schedule(
            { sendToKafka(retryRecord) },
            delayMs,
            TimeUnit.MILLISECONDS
        )
    }

    /**
     * 查询任务相关的事件历史
     *
     * Query task-related event history
     *
     * 从本地事件日志和 Kafka 中查询与指定任务相关的所有事件，支持时间范围过滤
     * 和数量限制。用于事件溯源、时间线重构和问题诊断。
     *
     * Queries all events related to specified task from local event log and Kafka,
     * supporting time range filtering and quantity limit. Used for event sourcing,
     * timeline reconstruction and problem diagnosis.
     *
     * @param taskId 任务 ID。
     *               Task ID.
     * @param fromEpochMs 查询起始时间戳（毫秒），null 表示无限制。
     *                     Query start timestamp in milliseconds, null means unlimited.
     * @param toEpochMs 查询结束时间戳（毫秒），null 表示无限制。
     *                   Query end timestamp in milliseconds, null means unlimited.
     * @param limit 返回的最大事件数量。
     *               Maximum number of events to return.
     * @return 事件记录列表，按时间升序排列。
     *         Event record list sorted by time ascending.
     */
    override suspend fun queryTaskEvents(
        taskId: String,
        fromEpochMs: Long?,
        toEpochMs: Long?,
        limit: Int
    ): List<EventRecord> {
        val normalizedTaskId = taskId.trim()
        if (normalizedTaskId.isEmpty()) {
            return emptyList()
        }
        val from = fromEpochMs ?: Long.MIN_VALUE
        val to = toEpochMs ?: Long.MAX_VALUE
        val safeLimit = limit.coerceAtLeast(1)
        val local = eventLog.asSequence()
            .filter { it.createdAtEpochMs in from..to }
            .filter { record -> matchesTask(record, normalizedTaskId) }
            .sortedBy { it.createdAtEpochMs }
            .toList()
        val fromKafka = runCatching {
            queryFromKafka(normalizedTaskId, from, to, safeLimit)
        }.getOrElse { emptyList() }
        return (local + fromKafka)
            .distinctBy { it.eventId }
            .sortedBy { it.createdAtEpochMs }
            .let { if (it.size <= safeLimit) it else it.takeLast(safeLimit) }
    }

    /**
     * 创建 Kafka 消费者
     *
     * Create Kafka consumer
     *
     * 创建配置好的 Kafka 消费者实例，用于订阅消费。
     * Creates configured Kafka consumer instance for subscription consumption.
     *
     * @param consumerGroup 消费者组标识。
     *                       Consumer group identifier.
     * @param subscriptionId 订阅标识，用于构建客户端 ID。
     *                        Subscription identifier, for building client ID.
     * @return Kafka 消费者实例。
     *         Kafka consumer instance.
     */
    private fun createConsumer(consumerGroup: String, subscriptionId: String): KafkaConsumer<String, ByteArray> {
        val properties = Properties()
        properties[ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG] = bootstrapServers
        properties[ConsumerConfig.GROUP_ID_CONFIG] = consumerGroup
        properties[ConsumerConfig.CLIENT_ID_CONFIG] = "remote-solver-event-consumer-$subscriptionId"
        properties[ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG] = StringDeserializer::class.java.name
        properties[ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG] = ByteArrayDeserializer::class.java.name
        properties[ConsumerConfig.AUTO_OFFSET_RESET_CONFIG] = "earliest"
        properties[ConsumerConfig.ENABLE_AUTO_COMMIT_CONFIG] = "true"
        return KafkaConsumer(properties)
    }

    /**
     * 创建新的事件记录
     *
     * Create new event record
     *
     * 构建包含完整信息的 [EventRecord] 实例。
     * Builds [EventRecord] instance with complete information.
     *
     * @param topic 事件主题。
     *               Event topic.
     * @param key 事件键。
     *             Event key.
     * @param payload 事件数据。
     *                 Event data.
     * @param schemaVersion Schema 版本号。
     *                       Schema version number.
     * @param headers 规范化后的事件头。
     *                 Normalized event headers.
     * @param deliveryAttempt 投递尝试次数。
     *                         Delivery attempt count.
     * @param createdAtEpochMs 创建时间戳。
     *                          Creation timestamp.
     * @return 新创建的事件记录。
     *         Newly created event record.
     */
    private fun newRecord(
        topic: String,
        key: String,
        payload: ByteArray,
        schemaVersion: Int,
        headers: Map<String, String>,
        deliveryAttempt: Int,
        createdAtEpochMs: Long = clock.nowEpochMs()
    ): EventRecord =
        EventRecord(
            eventId = idGenerator.newId("evt"),
            topic = topic,
            key = key,
            payload = payload,
            createdAtEpochMs = createdAtEpochMs,
            schemaVersion = schemaVersion,
            deliveryAttempt = deliveryAttempt,
            headers = headers
        )

    /**
     * 发送事件到 Kafka
     *
     * Send event to Kafka
     *
     * 将事件记录封装为 Kafka ProducerRecord 并发送。所有事件头都会转换为
     * Kafka record headers。使用同步发送确保消息送达。
     *
     * Wraps event record as Kafka ProducerRecord and sends. All event headers
     * are converted to Kafka record headers. Uses synchronous send to ensure delivery.
     *
     * @param record 要发送的事件记录。
     *               Event record to send.
     */
    private fun sendToKafka(record: EventRecord) {
        val producerRecord = ProducerRecord<String, ByteArray>(record.topic, record.key, record.payload)
        producerRecord.headers().add(RecordHeader("eventId", record.eventId.toByteArray(StandardCharsets.UTF_8)))
        producerRecord.headers().add(RecordHeader("createdAtEpochMs", record.createdAtEpochMs.toString().toByteArray(StandardCharsets.UTF_8)))
        producerRecord.headers().add(RecordHeader("deliveryAttempt", record.deliveryAttempt.toString().toByteArray(StandardCharsets.UTF_8)))
        record.headers.forEach { (name, value) ->
            producerRecord.headers().add(RecordHeader(name, value.toByteArray(StandardCharsets.UTF_8)))
        }
        producer.send(producerRecord).get()
    }

    /**
     * 从 Kafka 记录解析事件
     *
     * Parse event from Kafka record
     *
     * 将 Kafka ConsumerRecord 转换为 [EventRecord] 实例，解析 headers 并进行
     * Schema 验证。
     *
     * Converts Kafka ConsumerRecord to [EventRecord] instance,
     * parsing headers and performing schema validation.
     *
     * @param record Kafka 消费记录。
     *               Kafka consumer record.
     * @return 解析后的事件记录。
     *         Parsed event record.
     */
    private fun fromKafkaRecord(record: ConsumerRecord<String, ByteArray>): EventRecord {
        val headers = mutableMapOf<String, String>()
        record.headers().forEach { header ->
            headers[header.key()] = String(header.value(), StandardCharsets.UTF_8)
        }
        val createdAtEpochMs = headers["createdAtEpochMs"]?.toLongOrNull() ?: clock.nowEpochMs()
        val normalizedHeaders = EventSchema.normalizeEnvelopeHeaders(
            headers = EventSchema.normalizeHeaders(headers),
            topic = record.topic(),
            occurredAtEpochMs = createdAtEpochMs,
            defaultProducer = producerName
        )
        val schemaVersion = EventSchema.resolveSchemaVersion(normalizedHeaders)
        schemaRegistry.validateForConsume(
            topic = record.topic(),
            headers = normalizedHeaders,
            schemaVersion = schemaVersion,
            mode = schemaValidationMode
        )
        val deliveryAttempt = normalizedHeaders["deliveryAttempt"]?.toIntOrNull() ?: 0
        val eventId = normalizedHeaders["eventId"] ?: idGenerator.newId("evt")
        return EventRecord(
            eventId = eventId,
            topic = record.topic(),
            key = record.key() ?: "",
            payload = record.value() ?: ByteArray(0),
            createdAtEpochMs = createdAtEpochMs,
            schemaVersion = schemaVersion,
            deliveryAttempt = deliveryAttempt,
            headers = normalizedHeaders
        )
    }

    /**
     * 在阻塞上下文中运行挂起函数
     *
     * Run suspend function in blocking context
     *
     * 用于在非挂起上下文（如线程）中调用挂起函数，通过 Continuation 机制桥接。
     * Used for calling suspend functions in non-suspend context (like thread),
     * bridging via Continuation mechanism.
     *
     * @param block 要执行的挂起函数。
     *              Suspend function to execute.
     */
    private fun runSuspendBlocking(block: suspend () -> Unit) {
        var failure: Throwable? = null
        val latch = CountDownLatch(1)
        block.startCoroutine(
            object : Continuation<Unit> {
                override val context = EmptyCoroutineContext

                override fun resumeWith(result: Result<Unit>) {
                    failure = result.exceptionOrNull()
                    latch.countDown()
                }
            }
        )
        latch.await()
        if (failure != null) {
            throw failure as Throwable
        }
    }

    /**
     * 检查事件是否与任务相关
     *
     * Check if event is related to task
     *
     * 通过检查事件键、事件头中的 taskId 和 payload 内容来判断事件是否属于指定任务。
     * Determines if event belongs to specified task by checking event key,
     * taskId in event headers and payload content.
     *
     * @param record 事件记录。
     *               Event record.
     * @param taskId 任务 ID。
     *               Task ID.
     * @return 是否与任务相关。
     *         Whether related to task.
     */
    private fun matchesTask(record: EventRecord, taskId: String): Boolean {
        if (record.key == taskId) {
            return true
        }
        if (record.headers["taskId"] == taskId) {
            return true
        }
        val payloadText = runCatching { String(record.payload, StandardCharsets.UTF_8) }.getOrNull()
        return payloadText?.contains("taskId=$taskId") == true
    }

    /**
     * 从 Kafka 查询任务相关事件
     *
     * Query task-related events from Kafka
     *
     * 创建临时消费者从头扫描指定主题，查找与任务相关的所有事件。
     * Creates temporary consumer to scan specified topics from beginning,
     * finding all events related to task.
     *
     * @param taskId 任务 ID。
     *               Task ID.
     * @param fromEpochMs 起始时间戳。
     *                    Start timestamp.
     * @param toEpochMs 结束时间戳。
     *                  End timestamp.
     * @param limit 最大数量限制。
     *              Maximum quantity limit.
     * @return 事件记录列表。
     *         Event record list.
     */
    private fun queryFromKafka(
        taskId: String,
        fromEpochMs: Long,
        toEpochMs: Long,
        limit: Int
    ): List<EventRecord> {
        val queryConsumer = createQueryConsumer()
        return try {
            val topics = if (queryTopics.isEmpty()) defaultQueryTopics() else queryTopics
            val partitions = topics.flatMap { topic ->
                runCatching {
                    queryConsumer.partitionsFor(
                        topic,
                        Duration.ofMillis(queryPollTimeoutMs.coerceIn(50L, 500L))
                    )
                }.getOrElse { emptyList() }
                    .map { partitionInfo -> TopicPartition(topic, partitionInfo.partition()) }
            }
            if (partitions.isEmpty()) {
                return emptyList()
            }
            queryConsumer.assign(partitions)
            queryConsumer.seekToBeginning(partitions)

            val collected = mutableListOf<EventRecord>()
            var rounds = 0
            while (rounds < queryMaxPollRounds) {
                val records = queryConsumer.poll(Duration.ofMillis(queryPollTimeoutMs.coerceAtLeast(50L)))
                if (records.isEmpty) {
                    rounds += 1
                    continue
                }
                records.forEach { raw ->
                    val parsed = fromKafkaRecord(raw)
                    if (parsed.createdAtEpochMs in fromEpochMs..toEpochMs && matchesTask(parsed, taskId)) {
                        collected.add(parsed)
                    }
                }
            }
            collected
                .distinctBy { it.eventId }
                .sortedBy { it.createdAtEpochMs }
                .let { if (it.size <= limit) it else it.takeLast(limit) }
        } finally {
            queryConsumer.close()
        }
    }

    /**
     * 创建查询专用消费者
     *
     * Create query-specific consumer
     *
     * 创建配置了短超时和唯一组 ID 的临时消费者，用于事件溯源查询。
     * Creates temporary consumer configured with short timeout and unique group ID,
     * for event sourcing query.
     *
     * @return Kafka 消费者实例。
     *         Kafka consumer instance.
     */
    private fun createQueryConsumer(): KafkaConsumer<String, ByteArray> {
        val properties = Properties()
        properties[ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG] = bootstrapServers
        properties[ConsumerConfig.GROUP_ID_CONFIG] = "remote-solver-event-query-${idGenerator.newId("query")}"
        properties[ConsumerConfig.CLIENT_ID_CONFIG] = "remote-solver-event-query-client-${idGenerator.newId("client")}"
        properties[ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG] = StringDeserializer::class.java.name
        properties[ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG] = ByteArrayDeserializer::class.java.name
        properties[ConsumerConfig.AUTO_OFFSET_RESET_CONFIG] = "earliest"
        properties[ConsumerConfig.ENABLE_AUTO_COMMIT_CONFIG] = "false"
        properties[ConsumerConfig.SESSION_TIMEOUT_MS_CONFIG] = "6000"
        properties[ConsumerConfig.REQUEST_TIMEOUT_MS_CONFIG] = "8000"
        return KafkaConsumer(properties)
    }

    /**
     * 默认查询主题集合
     *
     * Default query topics set
     *
     * 返回系统定义的标准事件主题列表，用于事件溯源查询的默认范围。
     * Returns system-defined standard event topic list, for default scope of event sourcing query.
     *
     * @return 默认查询主题集合。
     *         Default query topics set.
     */
    companion object {
        fun defaultQueryTopics(): Set<String> =
            linkedSetOf(
                EventTopics.SOLVING_REQUEST,
                EventTopics.SOLVING_CONTROL,
                EventTopics.TASK_DISPATCH,
                EventTopics.SOLVER_HEARTBEAT,
                EventTopics.SLICE_LIFECYCLE,
                EventTopics.TASK_RESULT,
                EventTopics.COST_AND_CAPABILITY_UPDATE,
                EventTopics.MONITOR_ALERT
            )
    }

}