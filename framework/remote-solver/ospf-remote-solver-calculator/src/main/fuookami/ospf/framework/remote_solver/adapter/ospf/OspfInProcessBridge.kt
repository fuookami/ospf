/*
 * OSPF 进程内桥接器
 * OSPF in-process bridge
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverType
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

/**
 * OSPF 1.1.0 进程内求解桥占位实现。
 * Placeholder in-process bridge for OSPF 1.1.0.
 *
 * OSPF 1.1.0 的进程内求解 API 已从旧 backend/frontend 包迁移，当前服务端保留外部进程桥作为
 * 可用生产边界，避免继续依赖不稳定的内部 API。
 *
 * OSPF 1.1.0 moved the in-process solve API away from the previous backend/frontend packages.
 * The server keeps the external-process bridge as the supported production boundary to avoid
 * depending on unstable internal APIs.
 */
class OspfInProcessBridge(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    @Suppress("UNUSED_PARAMETER")
    objectStoragePort: ObjectStoragePort,
    @Suppress("UNUSED_PARAMETER")
    private val defaultSolverType: SolverType = SolverType.AUTO,
    @Suppress("UNUSED_PARAMETER")
    private val defaultConfig: OspfSolverConfig = OspfSolverConfig()
) : OspfExecutionBridge {
    override suspend fun start(
        payload: SolvePayload,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle {
        return ExecutionHandle(
            handleId = idGenerator.newId("handle"),
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            startedAtEpochMs = clock.nowEpochMs()
        )
    }

    override suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle {
        return start(payload, taskId, sliceId, nodeId, tenantId)
    }

    override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult {
        return SliceResult(
            sliceId = handle.sliceId.value,
            completed = true,
            feasible = false,
            objectiveValue = null,
            gap = null,
            elapsedMs = 0L,
            message = unsupportedMessage
        )
    }

    override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? = null

    override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? {
        return SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsedMs = 0L,
            message = unsupportedMessage
        )
    }

    override suspend fun stop(handle: ExecutionHandle): Boolean = true

    private companion object {
        const val unsupportedMessage =
            "OSPF in-process bridge is not supported on OSPF 1.1.0; use ospf-external or inmemory adapter."
    }
}
