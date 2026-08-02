@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * Node Models
 * 节点模型
 *
 * This file defines the data structures for solver node management.
 * 本文件定义了求解器节点管理的数据结构。
 *
 * These models support:
 * 这些模型支持：
 * - Node capability tracking and selection
 * - 节点能力追踪和选择
 * - Node state monitoring
 * - 节点状态监控
 * - Scheduler decision making
 * - 调度器决策
 */
package fuookami.ospf.framework.remote_solver.domain

import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverTypeName
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import kotlin.time.Duration
import kotlin.time.DurationUnit
import kotlin.time.Instant
import kotlin.time.toDuration

/**
 * Node Capability Profile
 * 节点能力档案
 *
 * Describes the capabilities and pricing of a solver node.
 * 描述求解器节点的能力和定价。
 *
 * Used for node selection and cost estimation.
 * 用于节点选择和成本估算。
 *
 * @param nodeId Unique identifier for the node.
 *               节点的唯一标识符。
 * @param solverType Type of solver this node runs (e.g., "cplex", "gurobi").
 *                   此节点运行的求解器类型（如"cplex"、"gurobi"）。
 * @param performanceScore Performance score relative to other nodes (0.0-1.0).
 *                         相对于其他节点的性能评分（0.0-1.0）。
 * @param pricePerSecond Price per second of computation time.
 *                       每秒计算时间的价格。
 * @param minBillingUnit Minimum billing unit (e.g., 1 minute).
 *                       最小计费单位（如1分钟）。
 * @param supportsInterrupt Whether the node supports interrupt/resume.
 *                           节点是否支持中断/恢复。
 * @param supportsCheckpoint Whether the node supports checkpointing.
 *                            节点是否支持检查点。
 * @param supportsWarmStart Whether the node supports warm start from checkpoint.
 *                           节点是否支持从检查点热启动。
 * @param parallelUnits Number of parallel solving units available.
 *                       可用的并行求解单元数。
 * @param licenseCostPerSlice License cost per slice execution (optional, default 0.0).
 *                             每Slice执行的许可证成本（可选，默认0.0）。
 * @param supportedModelTypes Supported normalized model types.
 *                            支持的规范化模型类型。
 */
data class NodeCapabilityProfile(
    val nodeId: NodeId,
    val solverType: SolverTypeName,
    val performanceScore: Flt64,
    val pricePerSecond: Flt64,
    val minBillingUnit: Duration,
    val supportsInterrupt: Boolean,
    val supportsCheckpoint: Boolean,
    val supportsWarmStart: Boolean,
    val parallelUnits: Int,
    val licenseCostPerSlice: Flt64 = Flt64.zero,
    val supportedModelTypes: Set<NormalizedModelType> = setOf(
        NormalizedModelType.LINEAR,
        NormalizedModelType.QUADRATIC
    )
) {
    val nodeIdValue: String get() = nodeId.value
    val solverTypeValue: String get() = solverType.value
    val performanceScoreValue: Double get() = performanceScore.toDouble()
    val pricePerSecondValue: Double get() = pricePerSecond.toDouble()
    val minBillingUnitSeconds: Long get() = minBillingUnit.inWholeSeconds
    val licenseCostPerSliceValue: Double get() = licenseCostPerSlice.toDouble()

}

@Deprecated(
    message = "Use the semantic NodeCapabilityProfile constructor with NodeId, SolverTypeName, Flt64, and Duration."
)
fun NodeCapabilityProfile(
    nodeId: String,
    solverType: String,
    performanceScore: Double,
    pricePerSecond: Double,
    minBillingUnitSeconds: Long,
    supportsInterrupt: Boolean,
    supportsCheckpoint: Boolean,
    supportsWarmStart: Boolean,
    parallelUnits: Int,
    licenseCostPerSlice: Double = 0.0
): NodeCapabilityProfile = NodeCapabilityProfile(
    nodeId = NodeId.of(nodeId),
    solverType = SolverTypeName.of(solverType),
    performanceScore = Flt64(performanceScore),
    pricePerSecond = Flt64(pricePerSecond),
    minBillingUnit = minBillingUnitSeconds.toDuration(DurationUnit.SECONDS),
    supportsInterrupt = supportsInterrupt,
    supportsCheckpoint = supportsCheckpoint,
    supportsWarmStart = supportsWarmStart,
    parallelUnits = parallelUnits,
    licenseCostPerSlice = Flt64(licenseCostPerSlice)
)

/**
 * Node State
 * 节点状态
 *
 * Current state of a solver node including availability.
 * 求解器节点的当前状态，包括可用性。
 *
 * Used for real-time scheduling decisions.
 * 用于实时调度决策。
 *
 * @param nodeId Unique identifier for the node.
 *               节点的唯一标识符。
 * @param profile The node's capability profile.
 *                节点的能力档案。
 * @param availableUnits Number of parallel units currently available.
 *                        当前可用的并行单元数。
 * @param lastHeartbeat Last heartbeat timestamp.
 *                      最后心跳时间。
 * @param online Whether the node is currently online.
 *               节点当前是否在线。
 */
data class NodeState(
    val nodeId: NodeId,
    val profile: NodeCapabilityProfile,
    val availableUnits: Int,
    val lastHeartbeat: Instant,
    val online: Boolean = true
) {
    val nodeIdValue: String get() = nodeId.value
    val lastHeartbeatEpochMs: Long get() = lastHeartbeat.toEpochMilliseconds()

}

@Deprecated(
    message = "Use the semantic NodeState constructor with NodeId and Instant."
)
fun NodeState(
    nodeId: String,
    profile: NodeCapabilityProfile,
    availableUnits: Int,
    lastHeartbeatEpochMs: Long,
    online: Boolean = true
): NodeState = NodeState(
    nodeId = NodeId.of(nodeId),
    profile = profile,
    availableUnits = availableUnits,
    lastHeartbeat = Instant.fromEpochMilliseconds(lastHeartbeatEpochMs),
    online = online
)
