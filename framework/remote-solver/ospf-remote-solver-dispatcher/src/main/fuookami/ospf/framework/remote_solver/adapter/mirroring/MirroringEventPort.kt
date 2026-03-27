/*
 * 镜像事件端口适配器
 *
 * Mirroring Event Port Adapter
 *
 * 该模块提供事件发布镜像功能，将事件同时发送到主端口和镜像端口，
 * 用于事件流的双写、灾备或调试场景。
 * This module provides event publishing mirroring functionality, sending events
 * to both primary and mirror ports simultaneously, for event stream dual-write,
 * disaster recovery or debugging scenarios.
 */

package fuookami.ospf.framework.remote_solver.adapter.mirroring

import fuookami.ospf.framework.remote_solver.domain.EventRecord
import fuookami.ospf.framework.remote_solver.port.EventPort

/**
 * 镜像事件端口
 *
 * Mirroring Event Port
 *
 * 该类实现了 [EventPort] 接口，包装了主端口和镜像端口，在发布事件时将事件
 * 同时发送到两个端口。订阅、取消订阅、确认和否定确认操作仅作用于主端口。
 *
 * This class implements [EventPort] interface, wrapping primary and mirror ports,
 * sending events to both ports simultaneously when publishing events.
 * Subscribe, unsubscribe, ack and nack operations only affect the primary port.
 *
 * 镜像模式适用于以下场景：
 * Mirroring mode is suitable for the following scenarios:
 *
 * - **灾备**: 将事件复制到备用消息系统，确保主系统故障时数据不丢失。
 *   Disaster recovery: Replicate events to backup messaging system, ensuring data not lost when primary fails.
 * - **调试**: 将事件镜像到调试系统进行实时监控和分析。
 *   Debugging: Mirror events to debug system for real-time monitoring and analysis.
 * - **双写迁移**: 在迁移消息系统期间同时写入新旧系统。
 *   Dual-write migration: Write to both old and new systems during messaging system migration.
 *
 * @param primary 主事件端口，所有订阅相关操作仅作用于该端口。
 *                 Primary event port, all subscription-related operations only affect this port.
 * @param mirror 镜像事件端口，事件发布时同时发送到该端口。
 *                Mirror event port, events are also sent to this port when publishing.
 * @param failOpen 是否在镜像端口失败时继续执行。true 表示镜像失败不影响主端口操作；
 *                  false 表示镜像失败会抛出异常。
 *                  Whether to continue execution when mirror port fails.
 *                  true means mirror failure doesn't affect primary port operation;
 *                  false means mirror failure throws exception.
 */
class MirroringEventPort(
    private val primary: EventPort,
    private val mirror: EventPort,
    private val failOpen: Boolean = true
) : EventPort {

    /**
     * 发布事件
     *
     * Publish event
     *
     * 将事件发送到主端口和镜像端口。返回的是主端口的发布结果。
     * 如果 [failOpen] 为 true，镜像端口失败时仅记录错误，不影响主端口操作；
     * 如果 [failOpen] 为 false，镜像端口失败会抛出异常。
     *
     * Sends event to both primary and mirror ports. Returns primary port's publish result.
     * If [failOpen] is true, mirror port failure only logs error, doesn't affect primary port operation;
     * If [failOpen] is false, mirror port failure throws exception.
     *
     * @param topic 事件主题。
     *               Event topic.
     * @param key 事件键，用于分区或路由。
     *             Event key, used for partitioning or routing.
     * @param payload 事件数据字节数组。
     *                 Event data byte array.
     * @param headers 事件头信息映射。
     *                 Event header information mapping.
     * @return 主端口发布的事件记录。
     *         Event record published by primary port.
     */
    override suspend fun publish(
        topic: String,
        key: String,
        payload: ByteArray,
        headers: Map<String, String>
    ): EventRecord {
        val primaryRecord = primary.publish(topic = topic, key = key, payload = payload, headers = headers)
        if (failOpen) {
            runCatching {
                mirror.publish(topic = topic, key = key, payload = payload, headers = headers)
            }
        } else {
            mirror.publish(topic = topic, key = key, payload = payload, headers = headers)
        }
        return primaryRecord
    }

    /**
     * 订阅事件
     *
     * Subscribe to events
     *
     * 仅在主端口订阅指定主题的事件。镜像端口不参与订阅操作。
     * Only subscribes to events of specified topic on primary port.
     * Mirror port doesn't participate in subscription operation.
     *
     * @param topic 要订阅的事件主题。
     *               Event topic to subscribe.
     * @param consumerGroup 消费者组标识，用于分组消费。
     *                       Consumer group identifier for group consumption.
     * @param handler 事件处理函数，接收事件记录并处理。
     *                 Event handler function, receives event record and processes it.
     * @return 订阅标识，用于后续取消订阅。
     *         Subscription identifier for later unsubscribe.
     */
    override suspend fun subscribe(
        topic: String,
        consumerGroup: String,
        handler: suspend (EventRecord) -> Unit
    ): String =
        primary.subscribe(topic = topic, consumerGroup = consumerGroup, handler = handler)

    /**
     * 取消订阅
     *
     * Unsubscribe
     *
     * 仅在主端口取消订阅。镜像端口不参与取消订阅操作。
     * Only unsubscribes on primary port.
     * Mirror port doesn't participate in unsubscribe operation.
     *
     * @param subscriptionId 订阅标识，由 [subscribe] 返回。
     *                         Subscription identifier returned by [subscribe].
     * @return 是否成功取消订阅。
     *         Whether unsubscribe was successful.
     */
    override suspend fun unsubscribe(subscriptionId: String): Boolean =
        primary.unsubscribe(subscriptionId)

    /**
     * 确认事件处理成功
     *
     * Acknowledge event processing success
     *
     * 仅在主端口确认事件。镜像端口不参与确认操作。
     * Only acknowledges event on primary port.
     * Mirror port doesn't participate in ack operation.
     *
     * @param record 已处理的事件记录。
     *               Processed event record.
     */
    override suspend fun ack(record: EventRecord) {
        primary.ack(record)
    }

    /**
     * 否定确认事件，请求重新处理
     *
     * Negatively acknowledge event, request reprocessing
     *
     * 仅在主端口否定确认事件。镜像端口不参与否定确认操作。
     * Only negatively acknowledges event on primary port.
     * Mirror port doesn't participate in nack operation.
     *
     * @param record 需要重新处理的事件记录。
     *               Event record that needs reprocessing.
     * @param retryAtEpochMs 重试时间戳（毫秒），null 表示立即重试。
     *                        Retry timestamp in milliseconds, null means immediate retry.
     */
    override suspend fun nack(record: EventRecord, retryAtEpochMs: Long?) {
        primary.nack(record, retryAtEpochMs)
    }
}