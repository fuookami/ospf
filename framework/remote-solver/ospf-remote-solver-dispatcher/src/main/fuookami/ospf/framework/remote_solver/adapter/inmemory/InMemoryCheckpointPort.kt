/**
 * 内存检查点端口适配器
 *
 * 提供基于内存的检查点存储实现，用于测试和非持久化场景。
 * 支持按任务保留最大数量的检查点，自动清理最旧的检查点。
 *
 * In-memory checkpoint port adapter.
 *
 * Provides memory-based checkpoint storage implementation for testing and non-persistent scenarios.
 * Supports retaining a maximum number of checkpoints per task, automatically cleaning up the oldest checkpoints.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList

/**
 * 内存检查点端口实现
 *
 * 使用内存存储检查点元数据，支持按任务分组管理和自动清理。
 *
 * In-memory checkpoint port implementation.
 *
 * Uses in-memory storage for checkpoint metadata, supports per-task grouping and automatic cleanup.
 *
 * @param maxRetainedPerTask 每个任务保留的最大检查点数量，0表示不限制
 *                           Maximum number of checkpoints retained per task, 0 means unlimited
 */
class InMemoryCheckpointPort(
    private val maxRetainedPerTask: Int = 0
) : CheckpointPort {
    private val checkpointsByTask = ConcurrentHashMap<String, CopyOnWriteArrayList<CheckpointMetadata>>()

    /**
     * 保存检查点元数据
     *
     * 将检查点元数据添加到对应任务的列表中，并根据配置自动清理超出限制的旧检查点。
     *
     * Saves checkpoint metadata.
     *
     * Adds checkpoint metadata to the corresponding task's list and automatically cleans up old checkpoints
     * that exceed the limit based on configuration.
     *
     * @param metadata 检查点元数据
     *                 Checkpoint metadata
     */
    override suspend fun save(metadata: CheckpointMetadata) {
        val checkpoints = checkpointsByTask.computeIfAbsent(metadata.taskId.value) { CopyOnWriteArrayList() }
        synchronized(checkpoints) {
            checkpoints.add(metadata)
            trimIfNeeded(checkpoints)
        }
    }

    /**
     * 获取指定任务的最新检查点
     *
     * 返回指定任务创建时间最新的检查点元数据。
     *
     * Gets the latest checkpoint for the specified task.
     *
     * Returns the checkpoint metadata with the most recent creation time for the specified task.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 最新检查点元数据，如果不存在则返回null
     *         Latest checkpoint metadata, or null if not found
     */
    override suspend fun latest(taskId: TaskId): CheckpointMetadata? =
        checkpointsByTask[taskId.value]
            ?.maxByOrNull { it.createdAtEpochMs }

    /**
     * 列出指定任务的所有检查点
     *
     * 返回指定任务的所有检查点元数据，按创建时间升序排列。
     *
     * Lists all checkpoints for the specified task.
     *
     * Returns all checkpoint metadata for the specified task, sorted by creation time in ascending order.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 检查点元数据列表
     *         List of checkpoint metadata
     */
    override suspend fun list(taskId: TaskId): List<CheckpointMetadata> =
        checkpointsByTask[taskId.value]
            ?.sortedBy { it.createdAtEpochMs }
            ?: emptyList()

    /**
     * 根据需要清理检查点
     *
     * 如果配置了最大保留数量且当前数量超过限制，则删除最旧的检查点直到数量在限制范围内。
     *
     * Trims checkpoints if needed.
     *
     * If a maximum retention count is configured and the current count exceeds the limit,
     * deletes the oldest checkpoints until the count is within the limit.
     *
     * @param checkpoints 检查点列表
     *                    Checkpoint list
     */
    private fun trimIfNeeded(checkpoints: CopyOnWriteArrayList<CheckpointMetadata>) {
        if (maxRetainedPerTask <= 0) {
            return
        }
        while (checkpoints.size > maxRetainedPerTask) {
            val oldest = checkpoints.minByOrNull { it.createdAtEpochMs } ?: return
            checkpoints.remove(oldest)
        }
    }
}
