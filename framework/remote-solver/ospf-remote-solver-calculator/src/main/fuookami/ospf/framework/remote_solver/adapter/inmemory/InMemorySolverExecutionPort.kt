/**
 * 内存求解器执行端口
 * In-memory solver execution port
 *
 * 用于测试和开发的 SolverExecutionPort 内存实现。
 * In-memory implementation of SolverExecutionPort for testing and development.
 *
 * 此实现模拟求解器执行，不调用实际 OSPF 求解器。
 * This implementation simulates solver execution without calling actual OSPF solvers.
 * 它在可配置的模拟运行时间内跟踪进度从 0.0 到 1.0。
 * It tracks progress from 0.0 to 1.0 over a configurable simulated runtime.
 *
 * **注意**：主要用于测试。生产环境请使用 OspfSolverExecutionPort。
 * **NOTE**: This is primarily for testing. For production, use OspfSolverExecutionPort.
 *
 * @param clock 时钟端口 / Clock port for timing
 * @param idGenerator ID 生成器 / ID generator for handles
 * @param objectStoragePort 对象存储端口 / Object storage for results
 * @param simulatedTotalRuntimeMs 模拟总运行时间（毫秒） / Simulated total runtime in milliseconds
 * @param simulatedObjectiveValue 模拟目标值 / Simulated objective value to report
 * @param alwaysFeasible 是否总是报告可行解 / Whether to always report feasible solutions
 * @param alwaysOptimal 是否总是报告最优解 / Whether to always report optimal solutions
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import kotlin.time.Duration
import fuookami.ospf.framework.remote_solver.protocol.domain.*
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import kotlin.math.roundToLong

class InMemorySolverExecutionPort(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val objectStoragePort: ObjectStoragePort,
    private val simulatedTotalRuntimeMs: Long = 12000L,
    private val simulatedObjectiveValue: Double = 0.0,
    private val alwaysFeasible: Boolean = true,
    private val alwaysOptimal: Boolean = true
) : SolverExecutionPort {

    /**
     * 执行上下文
     * Execution context
     */
    private data class ExecutionContext(
        val tenantId: String,
        val taskId: String,
        val sliceId: String,
        val nodeId: String,
        val payload: SolvePayload,
        var progress: Double,
        var elapsedMs: Long,
        var checkpoint: ObjectRef? = null
    )

    private val contextsByHandle = linkedMapOf<String, ExecutionContext>()
    private val progressByCheckpoint = linkedMapOf<String, Double>()

    /** 是否报告可行解 / Whether to report feasible */
    var reportFeasible: Boolean = alwaysFeasible

    /** 是否报告最优解 / Whether to report optimal */
    var reportOptimal: Boolean = alwaysOptimal

    /** 自定义目标值 / Custom objective value */
    var customObjectiveValue: Double? = null

    /**
     * 启动新求解任务
     * Start a new solve task
     */
    override suspend fun start(
        payload: SolvePayload,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId
    ): ExecutionHandle {
        val handle = newHandle(taskId.value, sliceId.value, nodeId.value)
        contextsByHandle[handle.handleId.value] = ExecutionContext(
            tenantId = tenantId.value,
            taskId = taskId.value,
            sliceId = sliceId.value,
            nodeId = nodeId.value,
            payload = payload,
            progress = 0.0,
            elapsedMs = 0L
        )
        return handle
    }

    /**
     * 从检查点恢复求解
     * Resume solve from checkpoint
     */
    override suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId
    ): ExecutionHandle {
        val restoredProgress = restoreProgress(checkpoint)
        val handle = newHandle(taskId.value, sliceId.value, nodeId.value)
        contextsByHandle[handle.handleId.value] = ExecutionContext(
            tenantId = tenantId.value,
            taskId = taskId.value,
            sliceId = sliceId.value,
            nodeId = nodeId.value,
            payload = payload,
            progress = restoredProgress,
            elapsedMs = (simulatedTotalRuntimeMs * restoredProgress).roundToLong(),
            checkpoint = checkpoint
        )
        return handle
    }

    /**
     * 等待时间切片结束
     * Wait for time slice to end
     */
    override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult {
        val context = contextsByHandle[handle.handleId.value]
            ?: return SliceResult(
                sliceId = handle.sliceId.value,
                completed = false,
                feasible = false,
                objectiveValue = null,
                gap = null,
                elapsedMs = 0L,
                message = "Execution handle not found"
            )

        val safeQuantumMs = quantum.inWholeMilliseconds.coerceAtLeast(1L)
        context.elapsedMs += safeQuantumMs
        context.progress = (context.progress + safeQuantumMs.toDouble() / simulatedTotalRuntimeMs.toDouble())
            .coerceAtMost(1.0)

        val completed = context.progress >= 1.0
        val objectiveValue = customObjectiveValue
            ?: (simulatedObjectiveValue * (1.0 - context.progress)).coerceAtLeast(0.0)
        val gap = if (completed && reportOptimal) 0.0 else (1.0 - context.progress)

        return SliceResult(
            sliceId = handle.sliceId.value,
            completed = completed,
            feasible = reportFeasible,
            objectiveValue = objectiveValue,
            gap = gap,
            elapsedMs = safeQuantumMs,
            message = if (completed) "In-memory solver completed" else "In-memory solver suspended for next slice"
        )
    }

    /**
     * 导出检查点
     * Export checkpoint
     */
    override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? {
        val context = contextsByHandle[handle.handleId.value] ?: return null
        val checkpointRef = objectStoragePort.put(
            path = "${context.tenantId}/checkpoint/${context.taskId}/${context.sliceId}",
            bytes = context.progress.toString().toByteArray(),
            metadata = mapOf(
                "taskId" to context.taskId,
                "sliceId" to context.sliceId,
                "tenantId" to context.tenantId,
                "progress" to context.progress.toString()
            )
        )
        context.checkpoint = checkpointRef
        progressByCheckpoint[checkpointKey(checkpointRef)] = context.progress
        return checkpointRef
    }

    /**
     * 获取最终结果
     * Fetch final result
     */
    override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? {
        val context = contextsByHandle[handle.handleId.value] ?: return null
        if (context.progress < 1.0) {
            return null
        }
        return SolveResult(
            feasible = reportFeasible,
            optimal = reportFeasible && reportOptimal,
            objectiveValue = customObjectiveValue ?: simulatedObjectiveValue,
            gap = if (reportOptimal) 0.0 else null,
            elapsedMs = context.elapsedMs,
            checkpointRef = context.checkpoint,
            message = "In-memory solver result"
        )
    }

    /**
     * 停止执行
     * Stop execution
     */
    override suspend fun stop(handle: ExecutionHandle): Boolean =
        contextsByHandle.remove(handle.handleId.value) != null

    /**
     * 清理所有上下文
     * Clear all contexts
     */
    fun clear() {
        contextsByHandle.clear()
        progressByCheckpoint.clear()
    }

    /**
     * 获取执行进度
     * Get execution progress
     */
    fun getProgress(handle: ExecutionHandle): Double? =
        contextsByHandle[handle.handleId.value]?.progress

    private suspend fun restoreProgress(checkpoint: ObjectRef): Double {
        progressByCheckpoint[checkpointKey(checkpoint)]?.let { return it }
        val bytes = objectStoragePort.get(checkpoint) ?: return 0.0
        return bytes.decodeToString().toDoubleOrNull()?.coerceIn(0.0, 1.0) ?: 0.0
    }

    private fun newHandle(taskId: String, sliceId: String, nodeId: String): ExecutionHandle =
        ExecutionHandle(
            handleId = idGenerator.newId("handle"),
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            startedAtEpochMs = clock.nowEpochMs()
        )

    private fun checkpointKey(ref: ObjectRef): String = "${ref.path.value}@${ref.version?.value ?: "latest"}"
}

/**
 * 创建默认配置的 InMemorySolverExecutionPort
 * Creates an InMemorySolverExecutionPort with default settings
 */
fun inMemorySolverExecutionPort(
    clock: ClockPort,
    idGenerator: IdGeneratorPort,
    objectStoragePort: ObjectStoragePort,
    simulatedRuntimeMs: Long = 12000L
): InMemorySolverExecutionPort = InMemorySolverExecutionPort(
    clock = clock,
    idGenerator = idGenerator,
    objectStoragePort = objectStoragePort,
    simulatedTotalRuntimeMs = simulatedRuntimeMs
)
