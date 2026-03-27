/**
 * 内存事件端口适配器
 *
 * 提供基于内存的事件发布订阅实现，用于测试和非持久化场景。
 * 支持幂等发布、消费者组、重试机制、Schema验证和任务事件查询。
 *
 * In-memory event port adapter.
 *
 * Provides memory-based event pub/sub implementation for testing and non-persistent scenarios.
 * Supports idempotent publishing, consumer groups, retry mechanism, schema validation,
 * and task event querying.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.domain.EventRecord
import fuookami.ospf.framework.remote_solver.domain.EventSchema
import fuookami.ospf.framework.remote_solver.domain.EventSchemaRegistry
import fuookami.ospf.framework.remote_solver.domain.EventSchemaValidationMode
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.EventPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.port.TaskEventQueryPort
import java.nio.charset.StandardCharsets
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlin.coroutines.Continuation
import kotlin.coroutines.EmptyCoroutineContext
import kotlin.coroutines.startCoroutine

/**
 * 内存事件端口实现
 *
 * 使用内存存储事件记录和订阅关系，支持幂等发布、消费者组轮询分发和延迟重试。
 *
 * In-memory event port implementation.
 *
 * Uses in-memory storage for event records and subscriptions, supporting idempotent publishing,
 * consumer group round-robin dispatching, and delayed retries.
 *
 * @param clock 时钟端口，用于获取当前时间戳
 *              Clock port for obtaining current timestamps
 * @param idGenerator ID生成器端口，用于生成事件和订阅ID
 *                    ID generator port for generating event and subscription IDs
 * @param retryPolicy 重试策略配置
 *                    Retry policy configuration
 * @param producerName 生产者名称，用于默认Header
 *                     Producer name for default header
 * @param schemaRegistry Schema注册表，用于事件验证
 *                        Schema registry for event validation
 * @param schemaValidationMode Schema验证模式
 *                              Schema validation mode
 */
class InMemoryEventPort(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val retryPolicy: InMemoryEventRetryPolicy = InMemoryEventRetryPolicy(),
    private val producerName: String = "inmemory-event-port",
    private val schemaRegistry: EventSchemaRegistry = EventSchemaRegistry.default(),
    private val schemaValidationMode: EventSchemaValidationMode = EventSchemaValidationMode.LENIENT
) : EventPort, TaskEventQueryPort {
    companion object {
        /**
         * 重试调度器
         *
         * 单线程后台调度器，用于延迟执行重试任务。
         *
         * Retry scheduler.
         *
         * Single-threaded background scheduler for delayed retry task execution.
         */
        private val retryScheduler = Executors.newSingleThreadScheduledExecutor { runnable ->
            Thread(runnable, "inmemory-event-port-retry").apply {
                isDaemon = true
            }
        }
    }

    /**
     * 订阅信息
     *
     * 存储订阅的完整信息，包括订阅ID、主题、消费者组和处理函数。
     *
     * Subscription information.
     *
     * Stores complete subscription information including subscription ID, topic, consumer group, and handler.
     *
     * @param id 订阅ID
     *            Subscription ID
     * @param topic 主题名称
     *               Topic name
     * @param consumerGroup 消费者组名称
     *                       Consumer group name
     * @param handler 事件处理函数
     *                 Event handler function
     */
    private data class Subscription(
        val id: String,
        val topic: String,
        val consumerGroup: String,
        val handler: suspend (EventRecord) -> Unit
    )

    /**
     * 订阅ID到订阅信息映射
     *
     * 用于快速查找订阅信息。
     *
     * Subscription ID to subscription info map.
     *
     * Used for quick subscription info lookup.
     */
    private val subscriptionById = ConcurrentHashMap<String, Subscription>()

    /**
     * 主题订阅映射
     *
     * 键为主题，值为消费者组到订阅列表的映射。
     *
     * Topic subscription map.
     *
     * Keyed by topic, valued by consumer group to subscription list map.
     */
    private val subscriptionsByTopic =
        ConcurrentHashMap<String, ConcurrentHashMap<String, CopyOnWriteArrayList<Subscription>>>()

    /**
     * 轮询计数器映射
     *
     * 用于消费者组内的轮询分发。
     *
     * Round-robin counter map.
     *
     * Used for round-robin dispatching within consumer groups.
     */
    private val roundRobinByTopicAndGroup = ConcurrentHashMap<String, AtomicInteger>()

    /**
     * 已发布事件的幂等键映射
     *
     * 用于幂等发布去重。
     *
     * Published event idempotency key map.
     *
     * Used for idempotent publishing deduplication.
     */
    private val publishedByIdempotencyKey = ConcurrentHashMap<String, EventRecord>()

    /**
     * 事件日志
     *
     * 存储所有已发布的事件记录。
     *
     * Event log.
     *
     * Stores all published event records.
     */
    private val eventLog = CopyOnWriteArrayList<EventRecord>()

    /**
     * 发布事件
     *
     * 将事件发布到指定主题，支持幂等发布和Schema验证。
     *
     * Publishes event.
     *
     * Publishes an event to the specified topic, supporting idempotent publishing and schema validation.
     *
     * @param topic 主题名称
     *               Topic name
     * @param key 事件键
     *             Event key
     * @param payload 事件数据
     *                 Event payload
     * @param headers 事件头信息
     *                 Event headers
     * @return 事件记录
     *         Event record
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
            val dedupeKey = dedupeKey(topic, key, idempotencyKey)
            val created = synchronized(publishedByIdempotencyKey) {
                publishedByIdempotencyKey[dedupeKey]?.let { return it }
                val record = EventRecord(
                    eventId = idGenerator.newId("evt"),
                    topic = topic,
                    key = key,
                    payload = payload,
                    createdAtEpochMs = createdAtEpochMs,
                    schemaVersion = schemaVersion,
                    headers = normalizedHeaders
                )
                publishedByIdempotencyKey[dedupeKey] = record
                eventLog.add(record)
                record
            }
            deliver(created)
            return created
        }

        val record = EventRecord(
            eventId = idGenerator.newId("evt"),
            topic = topic,
            key = key,
            payload = payload,
            createdAtEpochMs = createdAtEpochMs,
            schemaVersion = schemaVersion,
            headers = normalizedHeaders
        )
        eventLog.add(record)
        deliver(record)
        return record
    }

    /**
     * 订阅主题
     *
     * 注册事件处理函数到指定主题和消费者组。
     *
     * Subscribes to topic.
     *
     * Registers an event handler to the specified topic and consumer group.
     *
     * @param topic 主题名称
     *               Topic name
     * @param consumerGroup 消费者组名称
     *                       Consumer group name
     * @param handler 事件处理函数
     *                 Event handler function
     * @return 订阅ID
     *         Subscription ID
     */
    override suspend fun subscribe(
        topic: String,
        consumerGroup: String,
        handler: suspend (EventRecord) -> Unit
    ): String {
        val id = idGenerator.newId("sub")
        val subscription = Subscription(id, topic, consumerGroup, handler)
        subscriptionById[id] = subscription
        subscriptionsByTopic
            .computeIfAbsent(topic) { ConcurrentHashMap() }
            .computeIfAbsent(consumerGroup) { CopyOnWriteArrayList() }
            .add(subscription)
        return id
    }

    /**
     * 取消订阅
     *
     * 根据订阅ID取消订阅，清理相关映射。
     *
     * Unsubscribes.
     *
     * Cancels subscription by subscription ID and cleans up related mappings.
     *
     * @param subscriptionId 订阅ID
     *                        Subscription ID
     * @return 是否成功取消
     *         Whether unsubscription was successful
     */
    override suspend fun unsubscribe(subscriptionId: String): Boolean {
        val subscription = subscriptionById.remove(subscriptionId) ?: return false
        val groupSubscriptions = subscriptionsByTopic[subscription.topic]?.get(subscription.consumerGroup)
        groupSubscriptions?.removeIf { it.id == subscription.id }
        if (groupSubscriptions != null && groupSubscriptions.isEmpty()) {
            subscriptionsByTopic[subscription.topic]?.remove(subscription.consumerGroup, groupSubscriptions)
            roundRobinByTopicAndGroup.remove(roundRobinKey(subscription.topic, subscription.consumerGroup))
        }
        val topicSubscriptions = subscriptionsByTopic[subscription.topic]
        if (topicSubscriptions != null && topicSubscriptions.isEmpty()) {
            subscriptionsByTopic.remove(subscription.topic, topicSubscriptions)
        }
        return true
    }

    /**
     * 确认事件处理成功
     *
     * 在内存实现中不执行实际操作。
     *
     * Acknowledges successful event handling.
     *
     * Performs no actual operation in memory implementation.
     *
     * @param record 事件记录
     *                Event record
     */
    override suspend fun ack(record: EventRecord) {
    }

    /**
     * 标记事件处理失败并触发重试
     *
     * 根据重试策略计算延迟时间并调度重新投递。
     *
     * Marks event handling failure and triggers retry.
     *
     * Calculates delay based on retry policy and schedules redelivery.
     *
     * @param record 事件记录
     *                Event record
     * @param retryAtEpochMs 可选的自定义重试时间戳，null时使用策略计算
     *                        Optional custom retry timestamp, uses policy calculation when null
     */
    override suspend fun nack(record: EventRecord, retryAtEpochMs: Long?) {
        val retryRecord = record.copy(deliveryAttempt = record.deliveryAttempt + 1)
        val delayMs = if (retryAtEpochMs == null) {
            retryPolicy.computeDelayMs(retryRecord.deliveryAttempt)
        } else {
            retryAtEpochMs - clock.nowEpochMs()
        }
        if (delayMs <= 0L) {
            deliver(retryRecord)
            return
        }
        retryScheduler.schedule(
            {
                runSuspendBlocking {
                    deliver(retryRecord)
                }
            },
            delayMs,
            TimeUnit.MILLISECONDS
        )
    }

    /**
     * 查询任务相关事件
     *
     * 根据任务ID和时间范围查询相关事件记录。
     *
     * Queries task-related events.
     *
     * Queries event records related to the task by task ID and time range.
     *
     * @param taskId 任务ID
     *               Task ID
     * @param fromEpochMs 起始时间戳（可选）
     *                     Start timestamp (optional)
     * @param toEpochMs 结束时间戳（可选）
     *                   End timestamp (optional)
     * @param limit 最大返回数量
     *               Maximum return count
     * @return 事件记录列表
     *         Event record list
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
        return eventLog.asSequence()
            .filter { it.createdAtEpochMs in from..to }
            .filter { record -> matchesTask(record, normalizedTaskId) }
            .sortedBy { it.createdAtEpochMs }
            .toList()
            .let { if (it.size <= safeLimit) it else it.takeLast(safeLimit) }
    }

    /**
     * 投递事件
     *
     * 将事件分发给所有订阅该主题的消费者组，使用轮询方式选择消费者。
     *
     * Delivers event.
     *
     * Dispatches event to all consumer groups subscribed to the topic, using round-robin consumer selection.
     *
     * @param record 事件记录
     *                Event record
     */
    private suspend fun deliver(record: EventRecord) {
        schemaRegistry.validateForConsume(
            topic = record.topic,
            headers = record.headers,
            schemaVersion = record.schemaVersion,
            mode = schemaValidationMode
        )
        val groupedSubscriptions = subscriptionsByTopic[record.topic] ?: return
        groupedSubscriptions.forEach { (consumerGroup, subscriptions) ->
            val target = chooseTarget(record.topic, consumerGroup, subscriptions) ?: return@forEach
            target.handler(record)
        }
    }

    /**
     * 生成去重键
     *
     * 组合主题、键和幂等键生成唯一去重标识。
     *
     * Generates deduplication key.
     *
     * Combines topic, key, and idempotency key into a unique deduplication identifier.
     *
     * @param topic 主题名称
     *               Topic name
     * @param key 事件键
     *             Event key
     * @param idempotencyKey 幂等键
     *                        Idempotency key
     * @return 去重键字符串
     *         Deduplication key string
     */
    private fun dedupeKey(topic: String, key: String, idempotencyKey: String): String =
        "$topic|$key|$idempotencyKey"

    /**
     * 生成轮询键
     *
     * 组合主题和消费者组生成轮询计数器键。
     *
     * Generates round-robin key.
     *
     * Combines topic and consumer group into a round-robin counter key.
     *
     * @param topic 主题名称
     *               Topic name
     * @param consumerGroup 消费者组名称
     *                       Consumer group name
     * @return 轮询键字符串
     *         Round-robin key string
     */
    private fun roundRobinKey(topic: String, consumerGroup: String): String =
        "$topic|$consumerGroup"

    /**
     * 选择目标订阅
     *
     * 使用轮询算法从订阅列表中选择一个消费者。
     *
     * Chooses target subscription.
     *
     * Selects one consumer from subscription list using round-robin algorithm.
     *
     * @param topic 主题名称
     *               Topic name
     * @param consumerGroup 消费者组名称
     *                       Consumer group name
     * @param subscriptions 订阅列表
     *                      Subscription list
     * @return 目标订阅，如果列表为空则返回null
     *         Target subscription, returns null if list is empty
     */
    private fun chooseTarget(
        topic: String,
        consumerGroup: String,
        subscriptions: CopyOnWriteArrayList<Subscription>
    ): Subscription? {
        if (subscriptions.isEmpty()) {
            return null
        }
        val cursor = roundRobinByTopicAndGroup.computeIfAbsent(roundRobinKey(topic, consumerGroup)) {
            AtomicInteger(0)
        }
        val index = cursor.getAndIncrement()
        val position = Math.floorMod(index, subscriptions.size)
        return subscriptions[position]
    }

    /**
     * 判断事件是否与任务相关
     *
     * 通过事件键、Header或Payload中的taskId字段进行匹配。
     *
     * Determines if event is related to task.
     *
     * Matches by event key, header, or taskId field in payload.
     *
     * @param record 事件记录
     *                Event record
     * @param taskId 任务ID
     *               Task ID
     * @return 是否相关
     *         Whether related
     */
    private fun matchesTask(record: EventRecord, taskId: String): Boolean {
        // 匹配事件键
        // Match by event key
        if (record.key == taskId) {
            return true
        }
        // 匹配taskId Header
        // Match by taskId header
        if (record.headers["taskId"] == taskId) {
            return true
        }
        // 匹配JSON Payload中的taskId
        // Match by taskId in JSON payload
        val payloadText = runCatching { String(record.payload, StandardCharsets.UTF_8) }.getOrNull()
        if (payloadText != null) {
            // 新格式: JSON "taskId":"..."
            // New format: JSON "taskId":"..."
            if (payloadText.contains(""""taskId":"$taskId"""") ||
                payloadText.contains(""""taskId": "$taskId"""")) {
                return true
            }
            // 遗留格式: taskId=...
            // Legacy format: taskId=...
            if (payloadText.contains("taskId=$taskId")) {
                return true
            }
        }
        return false
    }

    /**
     * 阻塞式执行挂起函数
     *
     * 在非挂起上下文中阻塞等待挂起函数执行完成。
     *
     * Executes suspend function in blocking manner.
     *
     * Blocks and waits for suspend function execution completion in non-suspend context.
     *
     * @param block 要执行的挂起函数
     *              Suspend function to execute
     */
    private fun runSuspendBlocking(block: suspend () -> Unit) {
        val latch = CountDownLatch(1)
        val failureRef = AtomicReference<Throwable?>(null)
        block.startCoroutine(
            object : Continuation<Unit> {
                override val context = EmptyCoroutineContext

                override fun resumeWith(result: Result<Unit>) {
                    failureRef.set(result.exceptionOrNull())
                    latch.countDown()
                }
            }
        )
        latch.await()
        val failure = failureRef.get()
        if (failure != null) {
            throw failure
        }
    }
}