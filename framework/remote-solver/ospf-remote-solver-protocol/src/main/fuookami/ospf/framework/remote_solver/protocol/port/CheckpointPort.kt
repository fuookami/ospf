/**
 * 检查点端口
 * Checkpoint port
 *
 * 提供检查点持久化的抽象接口，用于求解任务的暂停恢复。
 * Provides abstract interface for checkpoint persistence, used for solve task suspend/resume.
 */
package fuookami.ospf.framework.remote_solver.protocol.port

import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId

/**
 * 检查点端口接口
 * Checkpoint port interface
 */
interface CheckpointPort {
    /**
     * 保存检查点元数据
     * Save checkpoint metadata
     *
     * @param metadata 检查点元数据 / Checkpoint metadata
     */
    suspend fun save(metadata: CheckpointMetadata)

    /**
     * 获取任务最新的检查点
     * Get latest checkpoint for task
     *
     * @param taskId 任务 ID / Task ID
     * @return 最新检查点元数据，不存在则返回 null / Latest checkpoint metadata, or null if not exists
     */
    suspend fun latest(taskId: String): CheckpointMetadata? = latest(TaskId.of(taskId))

    suspend fun latest(taskId: TaskId): CheckpointMetadata?

    /**
     * 列出任务的所有检查点
     * List all checkpoints for task
     *
     * @param taskId 任务 ID / Task ID
     * @return 检查点元数据列表 / List of checkpoint metadata
     */
    suspend fun list(taskId: String): List<CheckpointMetadata> = list(TaskId.of(taskId))

    suspend fun list(taskId: TaskId): List<CheckpointMetadata>
}
