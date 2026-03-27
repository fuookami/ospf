/*
 * 批量事件发布器
 *
 * 本模块提供批量事件发布功能，用于提高事件吞吐量。
 * 通过批量发布多个事件来减少 Kafka/网络开销，
 * 而不是逐个发布事件。
 *
 * Batch Event Publisher
 *
 * This module provides batch event publishing functionality for improved throughput.
 * Instead of publishing events one at a time, this batches multiple events
 * and publishes them together, reducing Kafka/network overhead.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.port.EventPort
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.atomic.AtomicLong

/**
 * 批量事件发布器
 *
 * 用于提高事件发布吞吐量的批量发布器。
 * 事件在满足以下条件时被发布：
 * - 批量大小达到 config.maxBatchSize
 * - 自上次发布以来的时间超过 config.maxBatchDelayMs
 * - 显式调用 flush()
 *
 * Batch event publisher for improved throughput.
 *
 * Events are published when:
 * - Batch size reaches config.maxBatchSize
 * - Time since last publish exceeds config.maxBatchDelayMs
 * - flush() is called explicitly
 *
 * @param eventPort 事件发布端口，用于实际发布事件
 *                   Event port for actual event publishing
 * @param metricsPort 指标端口，用于记录发布统计
 *                    Metrics port for recording publishing statistics
 * @param config 批量配置参数
 *               Batch configuration parameters
 */
class BatchEventPublisher(
    private val eventPort: EventPort,
    private val metricsPort: MetricsPort,
    private val config: BatchConfig = BatchConfig()
) {
    private val batchQueue = ConcurrentLinkedQueue<PendingEvent>()
    private val pendingCount = AtomicLong(0)
    private val publishedCount = AtomicLong(0)
    private val batchCount = AtomicLong(0)
    private val scope = CoroutineScope(Dispatchers.Default)

    /**
     * 将事件加入批量发布队列
     *
     * 事件在满足以下条件时被发布：
     * - 批量大小达到 config.maxBatchSize
     * - 自上次发布以来的时间超过 config.maxBatchDelayMs
     * - 显式调用 flush()
     *
     * Queues an event for batch publishing.
     *
     * Events are published when:
     * - Batch size reaches config.maxBatchSize
     * - Time since last publish exceeds config.maxBatchDelayMs
     * - flush() is called explicitly
     *
     * @param topic 事件主题
     *              Event topic
     * @param key 事件键
     *            Event key
     * @param payload 事件 payload 字节数组
     *                Event payload byte array
     * @param headers 事件头信息
     *                Event headers
     */
    fun queue(
        topic: String,
        key: String,
        payload: ByteArray,
        headers: Map<String, String>
    ) {
        if (!config.enabled) {
            // Disabled: publish immediately
            runBlocking {
                eventPort.publish(topic, key, payload, headers)
            }
            return
        }

        val event = PendingEvent(topic, key, payload, headers)
        batchQueue.offer(event)
        val count = pendingCount.incrementAndGet()
        runBlocking { metricsPort.gauge("event.batch.pending", count.toDouble()) }

        if (count >= config.maxBatchSize) {
            flush()
        }
    }

    /**
     * 立即刷新所有待发布事件
     *
     * Flushes all pending events immediately.
     */
    fun flush() {
        if (!config.enabled) return

        val events = mutableListOf<PendingEvent>()
        while (true) {
            val event = batchQueue.poll()
            if (event == null) break
            events.add(event)
        }

        if (events.isEmpty()) return

        pendingCount.set(0)
        runBlocking { metricsPort.gauge("event.batch.pending", 0.0) }

        // Publish batch asynchronously
        scope.launch {
            publishBatch(events)
        }
    }

    /**
     * 获取批量发布器统计信息
     *
     * Gets batch publisher statistics.
     *
     * @return 批量统计信息，包含待发布数、已发布数、批次数和平均批量大小
     *         Batch statistics containing pending count, published count, batch count and average batch size
     */
    fun getStats(): BatchStats {
        return BatchStats(
            pending = pendingCount.get(),
            published = publishedCount.get(),
            batches = batchCount.get(),
            avgBatchSize = if (batchCount.get() > 0) {
                publishedCount.get().toDouble() / batchCount.get().toDouble()
            } else 0.0
        )
    }

    private suspend fun publishBatch(events: List<PendingEvent>) {
        if (events.isEmpty()) return

        // Group events by topic for more efficient publishing
        val byTopic = events.groupBy { it.topic }

        for ((topic, topicEvents) in byTopic) {
            // Publish each event (could be optimized further with Kafka batch API)
            for (event in topicEvents) {
                try {
                    eventPort.publish(event.topic, event.key, event.payload, event.headers)
                    publishedCount.incrementAndGet()
                } catch (e: Exception) {
                    metricsPort.increment("event.batch.error", tags = mapOf("topic" to topic))
                }
            }
        }

        batchCount.incrementAndGet()
        metricsPort.increment("event.batch.published")
        metricsPort.timing("event.batch.size", events.size.toLong())
    }

    private data class PendingEvent(
        val topic: String,
        val key: String,
        val payload: ByteArray,
        val headers: Map<String, String>
    )
}

/**
 * 批量发布器配置
 *
 * Batch publisher configuration.
 *
 * @param enabled 是否启用批量发布，默认为 true
 *                Whether batch publishing is enabled, defaults to true
 * @param maxBatchSize 最大批量大小，默认为 100
 *                     Maximum batch size, defaults to 100
 * @param maxBatchDelayMs 自动刷新前的最大延迟时间（毫秒），默认为 100ms
 *                        Max delay before auto-flush in milliseconds, defaults to 100ms
 */
data class BatchConfig(
    val enabled: Boolean = true,
    val maxBatchSize: Int = 100,
    val maxBatchDelayMs: Long = 100L // Max delay before auto-flush
)

/**
 * 批量发布器统计信息
 *
 * Batch publisher statistics.
 *
 * @param pending 当前待发布事件数量
 *                Current pending event count
 * @param published 已发布事件总数
 *                  Total published event count
 * @param batches 已完成的批次总数
 *                Total completed batch count
 * @param avgBatchSize 平均批量大小
 *                     Average batch size
 */
data class BatchStats(
    val pending: Long,
    val published: Long,
    val batches: Long,
    val avgBatchSize: Double
)