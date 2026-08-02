/*
 * OSPF 求解器执行端口
 * OSPF Solver Execution Port
 *
 * 基于 OSPF 框架的求解器执行端口实现。
 * Solver execution port implementation based on OSPF framework.
 * 支持进程内求解和外部进程求解两种模式。
 * Supports both in-process and external process solving modes.
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import kotlin.time.Duration
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverType
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort as ISolverExecutionPort

/**
 * OSPF 求解器执行端口
 * OSPF Solver Execution Port
 *
 * 基于 OSPF 框架实现的 SolverExecutionPort。
 * SolverExecutionPort implementation based on OSPF framework.
 * 通过 OspfExecutionBridge 执行实际的求解器调用。
 * Uses OspfExecutionBridge to perform actual solver invocation.
 *
 * 支持的桥接器类型:
 * Supported bridge types:
 * - OspfInProcessBridge: 在进程内调用 OSPF 求解器（Gurobi/SCIP/Heuristic）
 *                        In-process calls to OSPF solvers (Gurobi/SCIP/Heuristic)
 * - OspfExternalProcessBridge: 调用外部求解器进程
 *                               Calls external solver process
 *
 * **重要**: 必须提供桥接器。没有默认的 mock 实现。
 * **IMPORTANT**: A bridge must be provided. There is no default mock implementation.
 * 测试时请使用测试目录中的 MockSolverExecutionPort。
 * For testing, use MockSolverExecutionPort from the test directory.
 *
 * @param bridge 执行桥接器，用于执行实际求解。必须提供，无默认实现。
 *               Execution bridge for performing actual solving. Must be provided - no default implementation.
 */
class OspfSolverExecutionPort(
    private val bridge: OspfExecutionBridge
) : fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort {

    /**
     * 从任务元数据的 targetType 推断规范化模型类型
     * Infer normalized model type from task meta targetType
     *
     * 映射规则:
     * Mapping:
     * - "milp", "lp", "linear" -> "linear"
     * - "miqp", "qp", "quadratic", "miqcp", "qcp" -> "quadratic"
     * - "cp", "constraint-programming", "constraint_programming" -> "cp"
     * - 其他 -> null
     * - default -> null
     *
     * @param targetType 任务目标类型字符串
     *                    Task target type string
     * @return 规范化的模型类型字符串，无法推断返回 null
     *         Normalized model type string, or null if cannot infer
     */
    private fun inferNormalizedModelType(targetType: String?): String? {
        return when (targetType?.lowercase()) {
            "milp", "lp", "linear" -> "linear"
            "miqp", "qp", "quadratic", "miqcp", "qcp" -> "quadratic"
            "cp", "constraint-programming", "constraint_programming" -> "cp"
            else -> null
        }
    }

    /**
     * 将规范化模型类型注入载荷扩展字段
     * Inject normalized model type into payload extension
     *
     * 如果载荷扩展中已存在 normalizedModelType，则不覆盖。
     * If normalizedModelType already exists in payload extension, does not override.
     *
     * @param payload 原始求解载荷
     *                 Original solve payload
     * @return 包含规范化模型类型的载荷
     *         Payload containing normalized model type
     */
    private fun injectNormalizedModelType(payload: SolvePayload): SolvePayload {
        // 首先检查 modelData 是否有显式的 modelType
        // First check if modelData has explicit modelType
        val modelType = payload.modelData.modelType
        val normalizedType = when (modelType) {
            NormalizedModelType.LINEAR -> "linear"
            NormalizedModelType.QUADRATIC -> "quadratic"
            NormalizedModelType.CP -> "cp"
            NormalizedModelType.UNKNOWN -> inferNormalizedModelType(payload.taskMeta.targetType?.value)
        }

        if (normalizedType == null || payload.extension.containsKey("normalizedModelType")) {
            return payload
        }

        return payload.copy(
            extension = payload.extension + ("normalizedModelType" to normalizedType)
        )
    }

    /**
     * 启动求解任务
     * Start a solving task
     *
     * 注入规范化模型类型后调用桥接器启动任务。
     * Injects normalized model type then calls bridge to start task.
     *
     * @param payload 求解载荷
     *                Solve payload
     * @param taskId 任务ID
     *                Task ID
     * @param sliceId 切片ID
     *                 Slice ID
     * @param nodeId 节点ID
     *                Node ID
     * @param tenantId 租户ID
     *                  Tenant ID
     * @return 执行句柄
     *         Execution handle
     */
    override suspend fun start(
        payload: SolvePayload,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId
    ): ExecutionHandle {
        val enrichedPayload = injectNormalizedModelType(payload)
        return bridge.start(enrichedPayload, taskId.value, sliceId.value, nodeId.value, tenantId.value)
    }

    /**
     * 从检查点恢复求解任务
     * Resume a solving task from checkpoint
     *
     * 注入规范化模型类型后调用桥接器恢复任务。
     * Injects normalized model type then calls bridge to resume task.
     *
     * @param payload 求解载荷
     *                Solve payload
     * @param checkpoint 检查点引用
     *                   Checkpoint reference
     * @param taskId 任务ID
     *                Task ID
     * @param sliceId 切片ID
     *                 Slice ID
     * @param nodeId 节点ID
     *                Node ID
     * @param tenantId 租户ID
     *                  Tenant ID
     * @return 执行句柄
     *         Execution handle
     */
    override suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId
    ): ExecutionHandle {
        val enrichedPayload = injectNormalizedModelType(payload)
        return bridge.resume(enrichedPayload, checkpoint, taskId.value, sliceId.value, nodeId.value, tenantId.value)
    }

    /**
     * 等待切片执行结束
     * Await slice execution end
     *
     * @param handle 执行句柄
     *                Execution handle
     * @param quantumMs 时间量子（毫秒）
     *                   Time quantum (milliseconds)
     * @return 切片结果
     *         Slice result
     */
    override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult =
        bridge.awaitSliceEnd(handle, quantum.inWholeMilliseconds)

    /**
     * 导出检查点
     * Export checkpoint
     *
     * @param handle 执行句柄
     *                Execution handle
     * @return 检查点引用，不支持时返回 null
     *         Checkpoint reference, or null if unsupported
     */
    override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? =
        bridge.exportCheckpoint(handle)

    /**
     * 获取最终求解结果
     * Fetch final solving result
     *
     * @param handle 执行句柄
     *                Execution handle
     * @return 最终求解结果，未完成时返回 null
     *         Final solving result, or null if not completed
     */
    override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? =
        bridge.fetchFinalResult(handle)

    /**
     * 停止求解执行
     * Stop solving execution
     *
     * @param handle 执行句柄
     *                Execution handle
     * @return 是否成功停止
     *         Whether successfully stopped
     */
    override suspend fun stop(handle: ExecutionHandle): Boolean =
        bridge.stop(handle)

    companion object {
        /**
         * 使用 OspfInProcessBridge 创建 OspfSolverExecutionPort
         * Create OspfSolverExecutionPort with OspfInProcessBridge
         *
         * 工厂方法，用于便捷创建进程内求解配置。
         * Factory method for convenient in-process solving configuration.
         *
         * @param clock 时钟端口，用于计时
         *              Clock port for timing
         * @param idGenerator ID生成器，用于生成句柄ID
         *                    ID generator for handle ID generation
         * @param objectStoragePort 对象存储端口，用于存储结果
         *                          Object storage port for storing results
         * @param solverType 默认求解器类型（SCIP, GUROBI, HEURISTIC, AUTO）
         *                   Default solver type (SCIP, GUROBI, HEURISTIC, AUTO)
         * @return 配置好的 OspfSolverExecutionPort 实例
         *         Configured OspfSolverExecutionPort instance
         */
        fun withInProcessBridge(
            clock: fuookami.ospf.framework.remote_solver.protocol.port.ClockPort,
            idGenerator: fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort,
            objectStoragePort: fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort,
            solverType: SolverType = SolverType.AUTO
        ): OspfSolverExecutionPort {
            val bridge = OspfInProcessBridge(
                clock = clock,
                idGenerator = idGenerator,
                objectStoragePort = objectStoragePort,
                defaultSolverType = solverType
            )
            return OspfSolverExecutionPort(bridge)
        }

        /**
         * 使用 OspfExternalProcessBridge 创建 OspfSolverExecutionPort
         * Create OspfSolverExecutionPort with OspfExternalProcessBridge
         *
         * 工厂方法，用于便捷创建外部进程求解配置。
         * Factory method for convenient external process solving configuration.
         *
         * @param clock 时钟端口，用于计时
         *              Clock port for timing
         * @param idGenerator ID生成器，用于生成句柄ID
         *                    ID generator for handle ID generation
         * @param objectStoragePort 对象存储端口，用于存储结果
         *                          Object storage port for storing results
         * @param command 外部求解器调用命令
         *                External solver invocation command
         * @param workdir 外部进程的工作目录（可选）
         *                Working directory for external process (optional)
         * @return 配置好的 OspfSolverExecutionPort 实例
         *         Configured OspfSolverExecutionPort instance
         */
        fun withExternalProcessBridge(
            clock: fuookami.ospf.framework.remote_solver.protocol.port.ClockPort,
            idGenerator: fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort,
            objectStoragePort: fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort,
            command: String,
            workdir: String? = null
        ): OspfSolverExecutionPort {
            val bridge = OspfExternalProcessBridge(
                clock = clock,
                idGenerator = idGenerator,
                objectStoragePort = objectStoragePort,
                args = buildMap {
                    put("command", command)
                    workdir?.let { put("workdir", it) }
                }
            )
            return OspfSolverExecutionPort(bridge)
        }
    }
}
