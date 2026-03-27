/*
 * OSPF 执行桥接器接口
 * OSPF Execution Bridge Interface
 *
 * 定义求解器执行的核心抽象接口。
 * Defines core abstract interface for solver execution.
 * 支持求解任务的启动、恢复、等待和停止操作。
 * Supports start, resume, await, and stop operations for solving tasks.
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult

/**
 * OSPF 执行桥接器接口
 * OSPF Execution Bridge Interface
 *
 * 定义求解器执行的抽象接口，支持不同的执行方式。
 * Defines abstract interface for solver execution, supporting different execution modes.
 * 实现可以包括进程内调用（OspfInProcessBridge）或外部进程调用（OspfExternalProcessBridge）。
 * Implementations can include in-process calls (OspfInProcessBridge) or external process calls (OspfExternalProcessBridge).
 */
interface OspfExecutionBridge {
    /**
     * 启动求解任务
     * Start a solving task
     *
     * 创建新的求解执行上下文并返回执行句柄。
     * Creates new solving execution context and returns execution handle.
     *
     * @param payload 求解任务的载荷，包含模型数据和配置
     *                Solving task payload containing model data and configuration
     * @param taskId 任务ID
     *                Task ID
     * @param sliceId 切片ID
     *                 Slice ID
     * @param nodeId 计算节点ID
     *                Compute node ID
     * @param tenantId 租户ID
     *                  Tenant ID
     * @return 执行句柄，用于后续操作
     *         Execution handle for subsequent operations
     */
    suspend fun start(
        payload: SolvePayload,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle

    /**
     * 从检查点恢复求解任务
     * Resume a solving task from checkpoint
     *
     * 使用已有的检查点数据恢复求解执行。
     * Resumes solving execution using existing checkpoint data.
     *
     * @param payload 求解任务的载荷
     *                Solving task payload
     * @param checkpoint 检查点的对象引用
     *                   Checkpoint object reference
     * @param taskId 任务ID
     *                Task ID
     * @param sliceId 切片ID
     *                 Slice ID
     * @param nodeId 计算节点ID
     *                Compute node ID
     * @param tenantId 租户ID
     *                  Tenant ID
     * @return 执行句柄
     *         Execution handle
     */
    suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle

    /**
     * 等待切片执行结束
     * Await slice execution end
     *
     * 等待当前时间切片的求解完成，返回切片结果。
     * Waits for current time slice solving to complete, returns slice result.
     *
     * @param handle 执行句柄
     *                Execution handle
     * @param quantumMs 时间量子（毫秒），最大等待时间
     *                   Time quantum in milliseconds, max wait time
     * @return 切片执行结果
     *         Slice execution result
     */
    suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult

    /**
     * 导出检查点
     * Export checkpoint
     *
     * 将当前求解状态导出为检查点对象。
     * Exports current solving state as checkpoint object.
     *
     * @param handle 执行句柄
     *                Execution handle
     * @return 检查点的对象引用，不支持检查点时返回 null
     *         Checkpoint object reference, or null if checkpointing not supported
     */
    suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef?

    /**
     * 获取最终求解结果
     * Fetch final solving result
     *
     * 当求解完成后，获取最终的求解结果。
     * Gets final solving result when solving is complete.
     *
     * @param handle 执行句柄
     *                Execution handle
     * @return 最终求解结果，未完成时返回 null
     *         Final solving result, or null if not completed
     */
    suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult?

    /**
     * 停止求解执行
     * Stop solving execution
     *
     * 终止正在进行的求解任务。
     * Terminates ongoing solving task.
     *
     * @param handle 执行句柄
     *                Execution handle
     * @return 是否成功停止
     *         Whether successfully stopped
     */
    suspend fun stop(handle: ExecutionHandle): Boolean
}