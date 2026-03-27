@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 任务状态端口接口
 *
 * Task State Port Interface
 *
 * 该接口定义了任务状态持久化的核心抽象，支持任务和切片状态的 CRUD 操作。
 * This interface defines the core abstraction for task state persistence,
 * supporting CRUD operations for task and slice states.
 *
 * 任务状态端口是远程求解器系统的核心组件，用于持久化任务执行状态。
 * The task state port is a core component of the remote solver system,
 * used to persist task execution states.
 *
 * 功能特性：
 * Features:
 * - 任务状态的创建、查询和更新 / Task state creation, query, and update
 * - 原子性条件更新 / Atomic conditional update
 * - 切片状态管理 / Slice state management
 */
package fuookami.ospf.framework.remote_solver.port

import kotlin.time.Instant
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId

/**
 * 任务状态端口接口
 *
 * Task State Port Interface
 *
 * 提供任务状态持久化功能的端口接口。
 * Port interface providing task state persistence capabilities.
 */
interface TaskStatePort {
    /**
     * 根据 ID 获取任务
     *
     * Gets a task by ID.
     *
     * 根据任务唯一标识符获取任务状态。
     * Retrieves task state by unique task identifier.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @return 任务状态，如不存在返回 null
     *         Task state, or null if not found
     */
    suspend fun getTask(taskId: String): TaskState? = getTask(TaskId.of(taskId))

    suspend fun getTask(taskId: TaskId): TaskState?

    /**
     * 根据请求 ID 获取任务
     *
     * Gets a task by request ID.
     *
     * 根据请求唯一标识符获取任务状态。
     * Retrieves task state by unique request identifier.
     *
     * @param requestId 请求唯一标识符 / Unique request identifier
     * @return 任务状态，如不存在返回 null
     *         Task state, or null if not found
     */
    suspend fun getTaskByRequestId(requestId: String): TaskState? = getTaskByRequestId(RequestId.of(requestId))

    suspend fun getTaskByRequestId(requestId: RequestId): TaskState?

    /**
     * 根据租户 ID 和请求 ID 获取任务
     *
     * Gets a task by tenant ID and request ID.
     *
     * 根据租户和请求唯一标识符组合获取任务状态，支持多租户隔离。
     * Retrieves task state by tenant and request identifier combination,
     * supporting multi-tenant isolation.
     *
     * @param tenantId 租户唯一标识符 / Unique tenant identifier
     * @param requestId 请求唯一标识符 / Unique request identifier
     * @return 任务状态，如不存在返回 null
     *         Task state, or null if not found
     */
    suspend fun getTaskByRequestId(tenantId: String?, requestId: String): TaskState? =
        if (tenantId == null) {
            getTaskByRequestId(RequestId.of(requestId))
        } else {
            getTaskByRequestId(TenantId.of(tenantId), RequestId.of(requestId))
        }

    suspend fun getTaskByRequestId(tenantId: TenantId, requestId: RequestId): TaskState?

    /**
     * 创建或更新任务
     *
     * Creates or updates a task.
     *
     * 创建新任务或更新现有任务状态。
     * Creates a new task or updates an existing task state.
     *
     * @param task 任务状态 / Task state
     */
    suspend fun upsertTask(task: TaskState)

    /**
     * 原子性条件更新任务状态
     *
     * Atomically updates task status with condition.
     *
     * 仅当任务当前状态在指定状态集合中时，才更新为新状态。此操作是原子的。
     * Updates task status only if current status is in the specified set.
     * This operation is atomic.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @param from 允许转换的源状态集合 / Set of allowed source statuses
     * @param to 目标状态 / Target status
     * @param updatedAtEpochMs 更新时间的毫秒级时间戳 / Update timestamp in milliseconds
     * @return 更新成功返回 true，条件不满足返回 false
     *         true if update succeeded, false if condition not met
     */
    suspend fun compareAndSet(
        taskId: String,
        from: Set<TaskStatus>,
        to: TaskStatus,
        updatedAtEpochMs: Long
    ): Boolean = compareAndSet(
        taskId = TaskId.of(taskId),
        from = from,
        to = to,
        updatedAt = Instant.fromEpochMilliseconds(updatedAtEpochMs)
    )

    suspend fun compareAndSet(
        taskId: TaskId,
        from: Set<TaskStatus>,
        to: TaskStatus,
        updatedAt: Instant
    ): Boolean

    /**
     * 按状态列出任务
     *
     * Lists tasks by status.
     *
     * 获取指定状态集合中的任务列表。
     * Retrieves list of tasks in the specified status set.
     *
     * @param statuses 状态集合 / Set of statuses
     * @param limit 最大返回数量（默认：100）/ Maximum number to return (default: 100)
     * @return 任务状态列表 / List of task states
     */
    suspend fun listTasks(statuses: Set<TaskStatus>, limit: Int = 100): List<TaskState>

    /**
     * 追加切片状态
     *
     * Appends a slice state.
     *
     * 将切片状态追加到任务中。
     * Appends a slice state to a task.
     *
     * @param slice 切片状态 / Slice state
     */
    suspend fun appendSlice(slice: SliceState)

    /**
     * 更新切片状态
     *
     * Updates a slice state.
     *
     * 更新现有切片的状态信息。
     * Updates an existing slice's state information.
     *
     * @param slice 切片状态 / Slice state
     */
    suspend fun updateSlice(slice: SliceState)

    /**
     * 获取任务的所有切片
     *
     * Gets all slices for a task.
     *
     * 获取指定任务的所有切片状态列表。
     * Retrieves all slice states for the specified task.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @return 切片状态列表 / List of slice states
     */
    suspend fun getSlices(taskId: String): List<SliceState> = getSlices(TaskId.of(taskId))

    suspend fun getSlices(taskId: TaskId): List<SliceState>
}
