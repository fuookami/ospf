/*
 * OSPF 进程内桥接器
 * OSPF in-process bridge
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverType
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import fuookami.ospf.kotlin.core.solver.report.CancellationSource
import fuookami.ospf.kotlin.core.solver.report.SolveHandle
import java.util.concurrent.ConcurrentHashMap

/**
 * OSPF 1.1.0 进程内求解桥。
 * In-process OSPF 1.1.0 execution bridge.
 *
 * CP snapshot 通过服务端 calculator 直接重建并调用 SCIP；非 CP 模型仍按明确的 unsupported
 * 边界返回，不再使用模拟 worker 冒充真实求解。
 *
 * CP snapshots are rebuilt by the calculator and solved with SCIP. Non-CP models remain an
 * explicit unsupported boundary instead of being represented by a simulated worker.
 *
 * @property clock 时钟端口 / Clock port
 * @property idGenerator 句柄 ID 生成器 / Handle ID generator
 * @property objectStoragePort 结果与 checkpoint 对象存储 / Result and checkpoint object storage
 * @property defaultSolverType 默认求解器类型 / Default solver type
 * @property defaultConfig 默认求解器配置 / Default solver configuration
 */
class OspfInProcessBridge @JvmOverloads constructor(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val objectStoragePort: ObjectStoragePort,
    @Suppress("UNUSED_PARAMETER")
    private val defaultSolverType: SolverType = SolverType.AUTO,
    @Suppress("UNUSED_PARAMETER")
    private val defaultConfig: OspfSolverConfig = OspfSolverConfig()
) : OspfExecutionBridge {
    private data class Context(
        val payload: SolvePayload,
        val taskId: String,
        val tenantId: String,
        val cancellation: SolveHandle,
        var elapsedMs: Long = 0L,
        var elapsedRecorded: Boolean = false,
        var checkpoint: PortableCheckpointEnvelope? = null,
        var result: SolveResult? = null,
        var checkpointRef: ObjectRef? = null,
        var resumeFailure: String? = null
    )

    private val cpExecutor = OspfCpSnapshotExecutor(objectStoragePort)
    private val contexts = ConcurrentHashMap<String, Context>()
    private val taskElapsedMs = ConcurrentHashMap<String, Long>()

    override val capabilities: OspfExecutionCapabilities =
        OspfExecutionCapabilities.controlledReturn()

    override fun capabilitiesFor(payload: SolvePayload): OspfExecutionCapabilities =
        if (isConstraintProgramming(payload)) {
            capabilities
        } else {
            OspfExecutionCapabilities.nonPreemptible()
        }

    override suspend fun start(
        payload: SolvePayload,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle {
        taskElapsedMs.putIfAbsent(taskId, 0L)
        val handle = ExecutionHandle(
            handleId = idGenerator.newId("handle"),
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            startedAtEpochMs = clock.nowEpochMs()
        )
        contexts[handle.handleId.value] = Context(
            payload = payload,
            taskId = taskId,
            tenantId = tenantId,
            cancellation = SolveHandle.create(),
            elapsedMs = taskElapsedMs[taskId] ?: 0L
        )
        return handle
    }

    override suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle {
        taskElapsedMs.putIfAbsent(taskId, 0L)
        val decoded = runCatching {
            objectStoragePort.get(checkpoint)
                ?.decodeToString()
                ?.let(PortableCheckpointCodec::decodeCompatibleOrNull)
        }.getOrNull()
        val handle = start(payload, taskId, sliceId, nodeId, tenantId)
        contexts[handle.handleId.value]?.apply {
            elapsedMs = taskElapsedMs[taskId] ?: 0L
            this.checkpoint = decoded
            this.checkpointRef = checkpoint
            if (decoded == null) {
                this.resumeFailure = "CP checkpoint 解码或完整性校验失败 / CP checkpoint decoding or integrity verification failed"
            }
        }
        return handle
    }

    override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult {
        val context = contexts[handle.handleId.value]
        if (context != null && isConstraintProgramming(context.payload)) {
            context.resumeFailure?.let { message ->
                val result = SolveResult(
                    feasible = false,
                    optimal = false,
                    objectiveValue = null,
                    gap = null,
                    elapsed = kotlin.time.Duration.ZERO,
                    problemStatus = RemoteProblemStatus.UNKNOWN,
                    terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
                    solutionPresence = RemoteSolutionPresence.NONE,
                    message = message
                )
                context.result = result
                return result.toSliceResult(handle)
            }
            val result = cpExecutor.execute(
                payload = context.payload,
                tenantId = context.tenantId,
                taskId = handle.taskId.value,
                sliceId = handle.sliceId.value,
                quantumMs = quantumMs,
                cancellationToken = context.cancellation.token,
                checkpoint = context.checkpoint,
                totalTimeLimitMs = taskTimeLimitMs(context.payload),
                elapsedBeforeMs = context.elapsedMs
            ).also { context.result = it }
            if (!context.elapsedRecorded) {
                context.elapsedMs = safeAdd(context.elapsedMs, result.elapsed.inWholeMilliseconds)
                taskElapsedMs[context.taskId] = context.elapsedMs
                context.elapsedRecorded = true
            }
            return result.toSliceResult(handle)
        }
        return SliceResult(
            sliceId = handle.sliceId,
            completed = false,
            feasible = false,
            objectiveValue = null,
            gap = null,
            elapsed = kotlin.time.Duration.ZERO,
            message = unsupportedMessage,
            problemStatus = RemoteProblemStatus.UNKNOWN,
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
            solutionPresence = RemoteSolutionPresence.NONE,
            proofStatus = RemoteProofStatus.NONE
        )
    }

    override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? {
        val context = contexts[handle.handleId.value] ?: return null
        if (!isConstraintProgramming(context.payload)) {
            return null
        }
        val result = context.result ?: return null
        val checkpoint = cpExecutor.exportCheckpoint(
            payload = context.payload,
            result = result,
            tenantId = context.tenantId,
            taskId = handle.taskId.value,
            sliceId = handle.sliceId.value,
            parentCheckpointId = context.checkpoint?.checkpointId
        )
        context.checkpointRef = checkpoint
        context.checkpoint = checkpoint?.let { ref ->
            objectStoragePort.get(ref)
                ?.decodeToString()
                ?.let(PortableCheckpointCodec::decodeCompatibleOrNull)
        }
        return checkpoint
    }

    override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? {
        contexts[handle.handleId.value]?.let { context ->
            if (isConstraintProgramming(context.payload)) {
                return context.result?.takeIf {
                    it.isTerminal()
                }
            }
        }
        return SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsed = kotlin.time.Duration.ZERO,
            message = unsupportedMessage,
            problemStatus = RemoteProblemStatus.UNKNOWN,
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
            solutionPresence = RemoteSolutionPresence.NONE,
            proofStatus = RemoteProofStatus.NONE
        )
    }

    override suspend fun stop(handle: ExecutionHandle): Boolean {
        val context = contexts[handle.handleId.value] ?: return false
        context.cancellation.cancel(CancellationSource.Remote, "remote task stopped")
        contexts.remove(handle.handleId.value, context)
        if (contexts.values.none { it.taskId == context.taskId }) {
            taskElapsedMs.remove(context.taskId)
        }
        return true
    }

    private fun taskTimeLimitMs(payload: SolvePayload): Long? =
        (payload.config?.timeLimitMs ?: payload.taskMeta.timeLimitMs)?.coerceAtLeast(0L)

    private fun safeAdd(left: Long, right: Long): Long {
        if (right <= 0L) {
            return left
        }
        return if (Long.MAX_VALUE - left < right) Long.MAX_VALUE else left + right
    }

    private fun isConstraintProgramming(payload: SolvePayload): Boolean {
        return payload.modelData.modelType == NormalizedModelType.CP ||
            payload.extension["normalizedModelType"]?.equals("cp", ignoreCase = true) == true
    }

    private fun SolveResult.toSliceResult(handle: ExecutionHandle): SliceResult {
        return SliceResult(
            sliceId = handle.sliceId,
            completed = isTerminal(),
            feasible = feasible,
            objectiveValue = objectiveValue,
            objectiveValueInt64 = objectiveValueInt64,
            gap = gap,
            elapsed = elapsed,
            message = message,
            schemaVersion = schemaVersion,
            problemStatus = problemStatus,
            terminationReason = terminationReason,
            solutionPresence = solutionPresence,
            proofStatus = proofStatus,
            checkpointRef = checkpointRef,
            resultRef = resultRef,
            provenance = provenance,
            fingerprints = fingerprints,
            fingerprintSchemas = fingerprintSchemas,
            statistics = statistics,
            diagnostics = diagnostics,
            runId = runId,
            attemptId = attemptId,
            artifactDigest = artifactDigest,
            incumbentRef = incumbentRef,
            modelFingerprint = modelFingerprint ?: fingerprints["model"],
            scheduling = scheduling,
            outcome = outcome
        )
    }

    private fun SolveResult.isTerminal(): Boolean {
        return terminationReason !in setOf(
            RemoteTerminationReason.TIME_LIMIT,
            RemoteTerminationReason.NODE_LIMIT,
            RemoteTerminationReason.ITERATION_LIMIT
        )
    }

    private companion object {
        const val unsupportedMessage =
            "OSPF in-process bridge is not supported on OSPF 1.1.0; use ospf-external or inmemory adapter."
    }
}
