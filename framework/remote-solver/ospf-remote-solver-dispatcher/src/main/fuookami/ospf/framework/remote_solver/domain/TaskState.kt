@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * Task State
 * 任务状态
 *
 * This file defines the core task and slice state data structures.
 * 本文件定义了核心任务和Slice状态数据结构。
 *
 * These models represent the runtime state of tasks in the dispatcher.
 * 这些模型表示调度器中任务的运行时状态。
 */
package fuookami.ospf.framework.remote_solver.domain

import kotlin.time.Duration
import kotlin.time.DurationUnit
import kotlin.time.Instant
import kotlin.time.toDuration
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.DispatchId
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/**
 * Task State
 * 任务状态
 *
 * Represents the runtime state of a solving task in the dispatcher.
 * 表示调度器中求解任务的运行时状态。
 *
 * Contains all information needed for task lifecycle management.
 * 包含任务生命周期管理所需的所有信息。
 *
 * @param taskId Unique identifier for the task.
 *                任务的唯一标识符。
 * @param requestId The original request identifier.
 *                   原始请求标识符。
 * @param tenantId The tenant that owns this task (default "default").
 *                  拥有此任务的租户（默认"default"）。
 * @param status Current task status.
 *               当前任务状态。
 * @param complexity Task complexity classification.
 *                    任务复杂度分类。
 * @param timeSensitivity Time sensitivity classification.
 *                         时间敏感度分类。
 * @param priority Task priority level (higher = more urgent).
 *                 任务优先级级别（越高 = 更紧急）。
 * @param deadline Deadline instant (optional).
 *                 截止时间（可选）。
 * @param payload The solving payload to be executed.
 *                 要执行的求解载荷。
 * @param latestResult The latest solving result (optional).
 *                     最新求解结果（可选）。
 * @param latestSnapshotRef Reference to the latest snapshot/checkpoint object (optional).
 *                           最新快照/检查点对象引用（可选）。
 * @param assignedNodeId Currently assigned solver node (optional).
 *                        当前分配的求解器节点（可选）。
 * @param createdAt Creation instant.
 *                  创建时间。
 * @param updatedAt Last update instant.
 *                  最后更新时间。
 * @param budgetScope The budget scope for cost tracking.
 *                    成本追踪的预算范围。
 * @param budgetLimit Maximum budget limit (optional).
 *                     最大预算限制（可选）。
 * @param consumedCost Cost already consumed by this task.
 *                     此任务已消耗的成本。
 */
data class TaskState(
    val taskId: TaskId,
    val requestId: RequestId,
    val tenantId: TenantId = TenantId.of("default"),
    val status: TaskStatus,
    val complexity: TaskComplexity,
    val timeSensitivity: TimeSensitivity,
    val priority: Int,
    val deadline: Instant?,
    val payload: SolvePayload,
    val latestResult: SolveResult? = null,
    val latestSnapshotRef: ObjectRef? = null,
    val assignedNodeId: NodeId? = null,
    val createdAt: Instant,
    val updatedAt: Instant,
    val budgetScope: BudgetScopeId,
    val budgetLimit: Flt64? = null,
    val consumedCost: Flt64 = Flt64.zero
) {
    val deadlineEpochMs: Long? get() = deadline?.toEpochMilliseconds()
    val createdAtEpochMs: Long get() = createdAt.toEpochMilliseconds()
    val updatedAtEpochMs: Long get() = updatedAt.toEpochMilliseconds()

    constructor(
        taskId: String,
        requestId: String,
        tenantId: String = "default",
        status: TaskStatus,
        complexity: TaskComplexity,
        timeSensitivity: TimeSensitivity,
        priority: Int,
        deadline: Long?,
        payload: SolvePayload,
        latestResult: SolveResult? = null,
        latestSnapshotRef: ObjectRef? = null,
        assignedNodeId: String? = null,
        createdAt: Long,
        updatedAt: Long,
        budgetScope: String,
        budgetLimit: Double? = null,
        consumedCost: Double = 0.0
    ) : this(
        taskId = TaskId.of(taskId),
        requestId = RequestId.of(requestId),
        tenantId = TenantId.of(tenantId),
        status = status,
        complexity = complexity,
        timeSensitivity = timeSensitivity,
        priority = priority,
        deadline = deadline?.let { Instant.fromEpochMilliseconds(it) },
        payload = payload,
        latestResult = latestResult,
        latestSnapshotRef = latestSnapshotRef,
        assignedNodeId = assignedNodeId?.let { NodeId.of(it) },
        createdAt = Instant.fromEpochMilliseconds(createdAt),
        updatedAt = Instant.fromEpochMilliseconds(updatedAt),
        budgetScope = BudgetScopeId.of(budgetScope),
        budgetLimit = budgetLimit?.let { Flt64(it) },
        consumedCost = Flt64(consumedCost)
    )
}

/**
 * Slice State
 * Slice状态
 *
 * Represents the runtime state of a task slice (execution segment).
 * 表示任务Slice（执行片段）的运行时状态。
 *
 * Slices are used for preemptive scheduling and checkpoint-based resumption.
 * Slice用于抢占式调度和基于检查点的恢复。
 *
 * @param sliceId Unique identifier for the slice.
 *                 Slice的唯一标识符。
 * @param taskId The parent task identifier.
 *               父任务标识符。
 * @param dispatchId The dispatch operation that created this slice.
 *                    创建此Slice的分发操作ID。
 * @param status Current slice status.
 *               当前Slice状态。
 * @param nodeId The solver node processing this slice (optional).
 *               处理此Slice的求解器节点（可选）。
 * @param quantum Time quantum allocated for this slice.
 *                为此Slice分配的时间量子。
 * @param checkpointRef Reference to checkpoint object for warm start (optional).
 *                       用于热启动的检查点对象引用（可选）。
 * @param resultRef Reference to the result object (optional).
 *                  结果对象引用（可选）。
 * @param startedAt Start instant (optional).
 *                  开始时间（可选）。
 * @param finishedAt Finish instant (optional).
 *                   完成时间（可选）。
 * @param error Error message if the slice failed (optional).
 *              Slice失败时的错误消息（可选）。
 */
data class SliceState(
    val sliceId: SliceId,
    val taskId: TaskId,
    val dispatchId: DispatchId,
    val status: SliceStatus,
    val nodeId: NodeId?,
    val quantum: Duration,
    val checkpointRef: ObjectRef? = null,
    val resultRef: ObjectRef? = null,
    val startedAt: Instant? = null,
    val finishedAt: Instant? = null,
    val error: String? = null
) {
    val quantumMs: Long get() = quantum.inWholeMilliseconds
    val startedAtEpochMs: Long? get() = startedAt?.toEpochMilliseconds()
    val finishedAtEpochMs: Long? get() = finishedAt?.toEpochMilliseconds()

    constructor(
        sliceId: String,
        taskId: String,
        dispatchId: String,
        status: SliceStatus,
        nodeId: String?,
        quantum: Long,
        checkpointRef: ObjectRef? = null,
        resultRef: ObjectRef? = null,
        startedAt: Long? = null,
        finishedAt: Long? = null,
        error: String? = null
    ) : this(
        sliceId = SliceId.of(sliceId),
        taskId = TaskId.of(taskId),
        dispatchId = DispatchId.of(dispatchId),
        status = status,
        nodeId = nodeId?.let { NodeId.of(it) },
        quantum = quantum.toDuration(DurationUnit.MILLISECONDS),
        checkpointRef = checkpointRef,
        resultRef = resultRef,
        startedAt = startedAt?.let { Instant.fromEpochMilliseconds(it) },
        finishedAt = finishedAt?.let { Instant.fromEpochMilliseconds(it) },
        error = error
    )

}
