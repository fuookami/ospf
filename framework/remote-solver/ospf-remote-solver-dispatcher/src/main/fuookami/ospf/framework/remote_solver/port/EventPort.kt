/*
 * 事件端口接口
 *
 * Event Port Interface
 *
 * 该接口定义了事件发布和订阅的核心抽象，支持异步消息传递。
 * This interface defines the core abstraction for event publishing and subscription,
 * supporting asynchronous message passing.
 *
 * 事件端口是远程求解器系统的事件驱动架构基础，用于组件间的解耦通信。
 * The event port is the foundation of the event-driven architecture in the remote solver system,
 * enabling decoupled communication between components.
 *
 * 典型使用场景：
 * Typical use cases:
 * - 任务状态变更通知 / Task status change notifications
 * - 计算结果事件分发 / Computation result event distribution
 * - 系统监控事件发布 / System monitoring event publishing
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.EventRecord

/**
 * 事件端口接口
 *
 * Event Port Interface
 *
 * 提供事件发布和订阅功能的端口接口。
 * Port interface providing event publishing and subscription capabilities.
 */
interface EventPort {
    /**
     * 发布事件
     *
     * Publishes an event to a topic.
     *
     * 将事件发布到指定的主题，供订阅者消费。
     * Publishes an event to the specified topic for subscribers to consume.
     *
     * @param topic 主题名称 / Topic name
     * @param key 事件键（用于分区）/ Event key (for partitioning)
     * @param payload 事件负载数据 / Event payload data
     * @param headers 事件头信息（默认：空）/ Event headers (default: empty)
     * @return 已发布的事件记录 / Published event record
     */
    suspend fun publish(
        topic: String,
        key: String,
        payload: ByteArray,
        headers: Map<String, String> = emptyMap()
    ): EventRecord

    /**
     * 订阅主题
     *
     * Subscribes to a topic.
     *
     * 订阅指定主题的事件，并为每个事件调用处理函数。
     * Subscribes to events from the specified topic and invokes the handler for each event.
     *
     * @param topic 主题名称 / Topic name
     * @param consumerGroup 消费者组标识符 / Consumer group identifier
     * @param handler 事件处理函数 / Event handler function
     * @return 订阅标识符（用于取消订阅）/ Subscription identifier (for unsubscription)
     */
    suspend fun subscribe(
        topic: String,
        consumerGroup: String,
        handler: suspend (EventRecord) -> Unit
    ): String

    /**
     * 取消订阅
     *
     * Unsubscribes from a topic.
     *
     * 取消之前的订阅，停止接收事件。
     * Cancels a previous subscription and stops receiving events.
     *
     * @param subscriptionId 订阅标识符 / Subscription identifier
     * @return 取消成功返回 true，失败返回 false
     *         true if unsubscription succeeded, false otherwise
     */
    suspend fun unsubscribe(subscriptionId: String): Boolean

    /**
     * 确认事件处理完成
     *
     * Acknowledges successful event processing.
     *
     * 确认事件已成功处理，可以将其从队列中移除。
     * Confirms that an event has been successfully processed and can be removed from the queue.
     *
     * @param record 已处理的事件记录 / Processed event record
     */
    suspend fun ack(record: EventRecord)

    /**
     * 标记事件处理失败
     *
     * Marks event processing as failed.
     *
     * 标记事件处理失败，请求重新投递。可指定重试时间。
     * Marks an event as failed and requests redelivery. Can specify retry time.
     *
     * @param record 处理失败的事件记录 / Failed event record
     * @param retryAtEpochMs 重试时间的毫秒级时间戳（可选）/ Retry timestamp in milliseconds (optional)
     */
    suspend fun nack(record: EventRecord, retryAtEpochMs: Long? = null)
}