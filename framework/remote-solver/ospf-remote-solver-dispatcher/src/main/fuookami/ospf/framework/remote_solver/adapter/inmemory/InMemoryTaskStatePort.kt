@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * 内存任务状态端口适配器
 *
 * 提供基于内存的任务状态管理实现，用于测试和非持久化场景。
 * 支持任务状态存储、查询、CAS更新和切片状态管理。
 *
 * In-memory task state port adapter.
 *
 * Provides memory-based task state management implementation for testing and non-persistent scenarios.
 * Supports task state storage, querying, CAS updates, and slice state management.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import kotlin.time.Instant
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList

/**
 * 内存任务状态端口实现
 *
 * 使用内存存储任务状态和切片状态，支持多索引查询和并发更新。
 *
 * In-memory task state port implementation.
 *
 * Uses in-memory storage for task states and slice states, supporting multi-index queries
 * and concurrent updates.
 */
class InMemoryTaskStatePort : TaskStatePort {
    /**
     * 任务状态存储映射
     *
     * 键为任务ID，值为任务状态。
     *
     * Task state storage map.
     *
     * Keyed by task ID, valued by task state.
     */
    private val tasks = ConcurrentHashMap<String, TaskState>()

    /**
     * 请求ID到任务ID映射（遗留全局索引）
     *
     * 用于向后兼容的全局请求ID索引。
     *
     * Request ID to task ID map (legacy global index).
     *
     * Global request ID index for backward compatibility.
     */
    private val taskIdByRequestId = ConcurrentHashMap<String, String>()

    /**
     * 租户+请求ID到任务ID映射
     *
     * 键为租户和请求ID的组合，值为任务ID。
     *
     * Tenant+request ID to task ID map.
     *
     * Keyed by tenant and request ID combination, valued by task ID.
     */
    private val taskIdByTenantAndRequestId = ConcurrentHashMap<String, String>()

    /**
     * 切片状态存储映射
     *
     * 键为任务ID，值为该任务的切片状态列表。
     *
     * Slice state storage map.
     *
     * Keyed by task ID, valued by slice state list for that task.
     */
    private val slicesByTask = ConcurrentHashMap<String, CopyOnWriteArrayList<SliceState>>()

    /**
     * 获取任务状态
     *
     * 根据任务ID获取任务状态。
     *
     * Gets task state.
     *
     * Retrieves task state by task ID.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 任务状态，如果不存在则返回null
     *         Task state, returns null if not found
     */
    override suspend fun getTask(taskId: TaskId): TaskState? = tasks[taskId.value]

    /**
     * 根据请求ID获取任务状态（遗留方法）
     *
     * 使用全局请求ID索引查找任务状态。
     *
     * Gets task state by request ID (legacy method).
     *
     * Finds task state using global request ID index.
     *
     * @param requestId 请求ID
     *                   Request ID
     * @return 任务状态，如果不存在则返回null
     *         Task state, returns null if not found
     */
    override suspend fun getTaskByRequestId(requestId: RequestId): TaskState? {
        val taskId = taskIdByRequestId[requestId.value] ?: return null
        return tasks[taskId]
    }

    /**
     * 根据租户和请求ID获取任务状态
     *
     * 使用租户范围的请求ID索引查找任务状态。
     *
     * Gets task state by tenant and request ID.
     *
     * Finds task state using tenant-scoped request ID index.
     *
     * @param tenantId 租户ID
     *                 Tenant ID
     * @param requestId 请求ID
     *                   Request ID
     * @return 任务状态，如果不存在则返回null
     *         Task state, returns null if not found
     */
    override suspend fun getTaskByRequestId(tenantId: TenantId, requestId: RequestId): TaskState? {
        val key = tenantRequestIdKey(tenantId.value, requestId.value)
        val taskId = taskIdByTenantAndRequestId[key] ?: return null
        return tasks[taskId]
    }

    /**
     * 更新或插入任务状态
     *
     * 将任务状态存储到内存中，同时更新相关索引。
     *
     * Upserts task state.
     *
     * Stores task state in memory and updates related indexes.
     *
     * @param task 任务状态
     *             Task state
     */
    override suspend fun upsertTask(task: TaskState) {
        synchronized(tasks) {
            val previous = tasks[task.taskId.value]
            tasks[task.taskId.value] = task

            // 更新遗留全局请求ID索引（向后兼容）
            // Update legacy global requestId index (for backward compatibility)
            if (previous != null && previous.requestId != task.requestId) {
                taskIdByRequestId.remove(previous.requestId.value, task.taskId.value)
            }
            taskIdByRequestId[task.requestId.value] = task.taskId.value

            // 更新租户范围的请求ID索引
            // Update tenant-scoped requestId index
            if (previous != null && previous.tenantId == task.tenantId && previous.requestId != task.requestId) {
                taskIdByTenantAndRequestId.remove(
                    tenantRequestIdKey(previous.tenantId.value, previous.requestId.value),
                    task.taskId.value
                )
            }
            if (previous != null && previous.tenantId != task.tenantId) {
                taskIdByTenantAndRequestId.remove(
                    tenantRequestIdKey(previous.tenantId.value, task.requestId.value),
                    task.taskId.value
                )
            }
            taskIdByTenantAndRequestId[tenantRequestIdKey(task.tenantId.value, task.requestId.value)] = task.taskId.value
        }
    }

    /**
     * 比较并设置任务状态
     *
     * 仅当当前状态在预期状态集合中时，才更新为新状态（CAS操作）。
     *
     * Compares and sets task status.
     *
     * Updates to new status only if current status is in expected status set (CAS operation).
     *
     * @param taskId 任务ID
     *               Task ID
     * @param from 预期的当前状态集合
     *             Expected current status set
     * @param to 目标状态
     *           Target status
     * @param updatedAtEpochMs 更新时间戳（毫秒）
     *                          Update timestamp in milliseconds
     * @return 是否成功更新
     *         Whether update was successful
     */
    override suspend fun compareAndSet(
        taskId: TaskId,
        from: Set<TaskStatus>,
        to: TaskStatus,
        updatedAt: Instant
    ): Boolean {
        synchronized(tasks) {
            val current = tasks[taskId.value] ?: return false
            if (current.status !in from) {
                return false
            }
            tasks[taskId.value] = current.copy(status = to, updatedAt = updatedAt)
            return true
        }
    }

    /**
     * 列出任务
     *
     * 根据状态过滤任务，按优先级和创建时间排序，返回指定数量的结果。
     *
     * Lists tasks.
     *
     * Filters tasks by status, sorts by priority and creation time, returns specified count.
     *
     * @param statuses 状态过滤集合
     *                 Status filter set
     * @param limit 最大返回数量
     *              Maximum return count
     * @return 任务状态列表
     *         Task state list
     */
    override suspend fun listTasks(statuses: Set<TaskStatus>, limit: Int): List<TaskState> =
        tasks.values
            .asSequence()
            .filter { it.status in statuses }
            .sortedWith(
                compareByDescending<TaskState> { it.priority }
                    .thenBy { it.createdAtEpochMs }
            )
            .take(limit)
            .toList()

    /**
     * 追加切片状态
     *
     * 将切片状态添加到对应任务的切片列表中，避免重复添加。
     *
     * Appends slice state.
     *
     * Adds slice state to the corresponding task's slice list, avoiding duplicates.
     *
     * @param slice 切片状态
     *              Slice state
     */
    override suspend fun appendSlice(slice: SliceState) {
        val slices = slicesByTask.computeIfAbsent(slice.taskId.value) { CopyOnWriteArrayList() }
        synchronized(slices) {
            val exists = slices.any { it.sliceId == slice.sliceId || it.dispatchId == slice.dispatchId }
            if (!exists) {
                slices.add(slice)
            }
        }
    }

    /**
     * 更新切片状态
     *
     * 更新现有切片状态，如果不存在则添加。
     *
     * Updates slice state.
     *
     * Updates existing slice state, adds if not found.
     *
     * @param slice 切片状态
     *              Slice state
     */
    override suspend fun updateSlice(slice: SliceState) {
        val slices = slicesByTask.computeIfAbsent(slice.taskId.value) { CopyOnWriteArrayList() }
        val index = slices.indexOfFirst { it.sliceId == slice.sliceId }
        if (index >= 0) {
            slices[index] = slice
        } else {
            slices.add(slice)
        }
    }

    /**
     * 获取任务的切片列表
     *
     * 返回指定任务的所有切片状态。
     *
     * Gets slice list for task.
     *
     * Returns all slice states for the specified task.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 切片状态列表
     *         Slice state list
     */
    override suspend fun getSlices(taskId: TaskId): List<SliceState> =
        slicesByTask[taskId.value]?.toList() ?: emptyList()

    /**
     * 生成租户请求ID组合键
     *
     * 将租户ID和请求ID组合成唯一键。
     *
     * Generates tenant request ID combination key.
     *
     * Combines tenant ID and request ID into a unique key.
     *
     * @param tenantId 租户ID
     *                 Tenant ID
     * @param requestId 请求ID
     *                   Request ID
     * @return 组合键字符串
     *         Combination key string
     */
    private fun tenantRequestIdKey(tenantId: String, requestId: String): String =
        "$tenantId|$requestId"
}
