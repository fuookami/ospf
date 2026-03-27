/*
 * Event Payloads
 * 事件载荷
 *
 * This file defines the serializable payload structures for different event types.
 * 本文件定义了不同事件类型的可序列化载荷结构。
 *
 * These payloads are used for:
 * 这些载荷用于：
 * - Task dispatch commands to solver nodes
 * - 向求解器节点发送任务分发命令
 * - Slice lifecycle tracking
 * - Slice生命周期追踪
 * - Task result reporting
 * - 任务结果报告
 * - Solving control commands
 * - 求解控制命令
 * - Cost and heartbeat updates
 * - 成本和心跳更新
 */
package fuookami.ospf.framework.remote_solver.domain

import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * JSON serializer configuration for event payloads.
 * 事件载荷的JSON序列化器配置。
 *
 * Uses ignoreUnknownKeys for forward compatibility and encodeDefaults=false for compact output.
 * 使用ignoreUnknownKeys实现向前兼容，encodeDefaults=false实现紧凑输出。
 */
private val eventJson = Json {
    ignoreUnknownKeys = true
    encodeDefaults = false
}

/**
 * Task Dispatch Payload
 * 任务分发载荷
 *
 * Payload for dispatching a task slice to a solver node.
 * 将任务Slice分发到求解器节点的载荷。
 *
 * @param dispatchId Unique identifier for this dispatch operation.
 *                    本次分发操作的唯一标识符。
 * @param taskId The task being dispatched.
 *               正在分发的任务ID。
 * @param sliceId The slice being dispatched.
 *                正在分发的Slice ID。
 * @param nodeId Target solver node identifier.
 *               目标求解器节点标识符。
 * @param quantumMs Time quantum allocated for this slice in milliseconds.
 *                  为此Slice分配的时间量子（毫秒）。
 * @param checkpointInRef Reference to checkpoint input object (for warm start).
 *                         Checkpoint输入对象引用（用于热启动）。
 */
@Serializable
data class TaskDispatchPayload(
    val dispatchId: String,
    val taskId: String,
    val sliceId: String,
    val nodeId: String,
    val quantumMs: Long,
    val checkpointInRef: String? = null
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}

/**
 * Slice Lifecycle Payload
 * Slice生命周期载荷
 *
 * Payload for tracking slice lifecycle events (start, stop, complete, etc.).
 * 追踪Slice生命周期事件（开始、停止、完成等）的载荷。
 *
 * @param taskId The task this slice belongs to.
 *               此Slice所属的任务ID。
 * @param sliceId The slice identifier.
 *                Slice标识符。
 * @param dispatchId The dispatch operation that created this slice.
 *                    创建此Slice的分发操作ID。
 * @param action The lifecycle action (e.g., "start", "stop", "complete").
 *               生命周期动作（如"start"、"stop"、"complete"）。
 * @param status Current slice status.
 *               当前Slice状态。
 * @param taskStatus Current task status.
 *                   当前任务状态。
 * @param nodeId Node processing this slice (optional).
 *               处理此Slice的节点（可选）。
 * @param quantumMs Time quantum in milliseconds (optional).
 *                   时间量子（毫秒，可选）。
 * @param reason Reason for the lifecycle event (optional).
 *               生命周期事件的原因（可选）。
 */
@Serializable
data class SliceLifecyclePayload(
    val taskId: String,
    val sliceId: String,
    val dispatchId: String,
    val action: String,
    val status: String,
    val taskStatus: String,
    val nodeId: String? = null,
    val quantumMs: Long? = null,
    val reason: String? = null
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}

/**
 * Task Result Payload
 * 任务结果载荷
 *
 * Payload for reporting task execution results.
 * 报告任务执行结果的载荷。
 *
 * @param taskId The task identifier.
 *               任务标识符。
 * @param status Final task status (e.g., "SUCCESS", "FAILED").
 *               最终任务状态（如"SUCCESS"、"FAILED"）。
 * @param reasonCode Reason code for the result (optional).
 *                   结果的原因代码（可选）。
 * @param message Human-readable result message (optional).
 *                 人类可读的结果消息（可选）。
 */
@Serializable
data class TaskResultPayload(
    val taskId: String,
    val status: String,
    val reasonCode: String? = null,
    val message: String? = null
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}

/**
 * Solving Control Payload
 * 求解控制载荷
 *
 * Payload for solving control commands (stop, resume, cancel, etc.).
 * 求解控制命令（停止、恢复、取消等）的载荷。
 *
 * @param taskId The task to control.
 *               要控制的任务ID。
 * @param action The control action (e.g., "stop", "resume", "cancel").
 *               控制动作（如"stop"、"resume"、"cancel"）。
 * @param status Status after the action (optional).
 *               动作后的状态（可选）。
 * @param dispatchId Related dispatch identifier (optional).
 *                    相关的分发操作ID（可选）。
 * @param nodeId Node identifier (optional).
 *               节点标识符（可选）。
 * @param dispatcherId Dispatcher that initiated the action (optional).
 *                      发起动作的调度器ID（可选）。
 * @param from Previous state (optional).
 *             之前的状态（可选）。
 * @param to Target state (optional).
 *           目标状态（可选）。
 * @param reason Reason for the control action (optional).
 *               控制动作的原因（可选）。
 * @param operator Operator who initiated the action (optional).
 *                 发起动作的操作者（可选）。
 * @param source Source system that initiated the action (optional).
 *               发起动作的来源系统（可选）。
 */
@Serializable
data class SolvingControlPayload(
    val taskId: String,
    val action: String,
    val status: String? = null,
    val dispatchId: String? = null,
    val nodeId: String? = null,
    val dispatcherId: String? = null,
    val from: String? = null,
    val to: String? = null,
    val reason: String? = null,
    val operator: String? = null,
    val source: String? = null
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}

/**
 * Cost Update Payload
 * 成本更新载荷
 *
 * Payload for updating cost and performance metrics.
 * 更新成本和性能指标的载荷。
 *
 * @param taskId Task identifier (optional).
 *               任务标识符（可选）。
 * @param sliceId Slice identifier (optional).
 *                Slice标识符（可选）。
 * @param nodeId Node identifier (optional).
 *               节点标识符（可选）。
 * @param totalCost Total cost incurred (optional).
 *                  已产生的总成本（可选）。
 * @param runtimeMs Runtime in milliseconds (optional).
 *                  运行时间（毫秒，可选）。
 * @param performanceScore Performance score of the node (optional).
 *                         节点的性能评分（可选）。
 * @param online Node online status (optional).
 *               节点在线状态（可选）。
 */
@Serializable
data class CostUpdatePayload(
    val taskId: String? = null,
    val sliceId: String? = null,
    val nodeId: String? = null,
    val totalCost: Double? = null,
    val runtimeMs: Long? = null,
    val performanceScore: Double? = null,
    val online: Boolean? = null
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}

/**
 * Heartbeat Payload
 * 心跳载荷
 *
 * Payload for solver node heartbeat events.
 * 求解器节点心跳事件的载荷。
 *
 * @param nodeId Node sending the heartbeat.
 *               发送心跳的节点标识符。
 * @param heartbeatAt Timestamp of the heartbeat in epoch milliseconds.
 *                     心跳时间戳（epoch毫秒）。
 */
@Serializable
data class HeartbeatPayload(
    val nodeId: String,
    val heartbeatAt: Long
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}

/**
 * Task Summary Payload
 * 任务摘要载荷
 *
 * Payload for task summary updates in monitoring.
 * 监控中任务摘要更新的载荷。
 *
 * @param taskId Task identifier.
 *               任务标识符。
 * @param status Current task status.
 *               当前任务状态。
 * @param priority Task priority level.
 *                 任务优先级。
 */
@Serializable
data class TaskSummaryPayload(
    val taskId: String,
    val status: String,
    val priority: Int
) {
    /**
     * Serializes this payload to a byte array.
     * 将此载荷序列化为字节数组。
     *
     * @return Serialized JSON bytes.
     *         序列化的JSON字节。
     */
    fun toByteArray(): ByteArray = eventJson.encodeToString(this).toByteArray()
}