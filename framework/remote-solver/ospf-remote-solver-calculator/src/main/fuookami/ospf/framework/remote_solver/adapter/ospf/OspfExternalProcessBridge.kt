/*
 * OSPF 外部进程桥接器
 * OSPF External Process Bridge
 *
 * 通过调用外部进程执行求解任务。
 * Executes solving tasks by calling external processes.
 * 适用于需要使用独立求解器程序的场景。
 * Suitable for scenarios requiring standalone solver programs.
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import java.nio.file.Files
import java.nio.file.Path
import java.security.MessageDigest
import kotlin.math.max
import kotlin.time.DurationUnit
import kotlin.time.toDuration
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingConflict
import fuookami.ospf.framework.remote_solver.protocol.domain.SerializedSolution
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverConfig
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprint
import fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprintLegacyV1Candidates

/**
 * OSPF 外部进程桥接器
 * OSPF External Process Bridge
 *
 * 通过调用外部进程执行求解器任务。
 * Executes solver tasks by calling external processes.
 * 外部进程通过命令行参数接收任务信息，通过标准输出返回结果。
 * External process receives task info via command line args, returns results via stdout.
 *
 * 命令行参数:
 * Command line args:
 * --model {modelPath}      模型文件路径
 *                          Model file path
 * --task {taskId}          任务ID
 *                          Task ID
 * --slice {sliceId}        切片ID
 *                          Slice ID
 * --node {nodeId}          节点ID
 *                          Node ID
 * --quantum-ms {ms}        时间量子（毫秒）
 *                          Time quantum (ms)
 * --checkpoint-in {path}   输入检查点路径（可选）
 *                          Input checkpoint path (optional)
 *
 * 输出格式（key=value 行或 SerializedSolution JSON 文件）:
 * Output format (key=value lines):
 * completed=true/false     是否完成
 *                          Whether completed
 * feasible=true/false      是否可行
 *                          Whether feasible
 * objective={value}        目标函数值
 *                          Objective value
 * gap={value}              MIP Gap
 *                          MIP Gap
 * elapsedMs={ms}           耗时（毫秒）
 *                          Elapsed time (ms)
 * message={text}           结果消息
 *                          Result message
 * checkpointPath={path}    输出检查点路径（可选）
 *                          Output checkpoint path (optional)
 * resultPath={path}        结果文件路径（可选）
 *                          Result file path (optional)
 *
 * @param clock 时钟端口，用于生成时间戳
 *              Clock port for timestamp generation
 * @param idGenerator ID生成器，用于生成句柄ID
 *                    ID generator for handle ID generation
 * @param objectStoragePort 对象存储端口，用于存储检查点和结果
 *                          Object storage port for storing checkpoints and results
 * @param args 配置参数，必须包含 "command" 键
 *             Configuration args, must contain "command" key
 */
class OspfExternalProcessBridge(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val objectStoragePort: ObjectStoragePort,
    private val args: Map<String, String> = emptyMap()
) : OspfExecutionBridge {
    /**
     * 执行上下文
     * Execution context
     *
     * 存储单个求解任务的所有执行状态信息。
     * Stores all execution state info for a single solving task.
     */
    private data class Context(
        val payload: SolvePayload,
        val tenantId: String,
        val taskId: String,
        val sliceId: String,
        val nodeId: String,
        var completed: Boolean = false,
        var feasible: Boolean = false,
        var optimal: Boolean = false,
        var objectiveValue: Double? = null,
        var gap: Double? = null,
        var elapsedMs: Long = 0L,
        var checkpointRef: ObjectRef? = null,
        var resultRef: ObjectRef? = null,
        var message: String? = null,
        var schemaVersion: String = "1.0",
        var problemStatus: RemoteProblemStatus = RemoteProblemStatus.UNKNOWN,
        var terminationReason: RemoteTerminationReason = RemoteTerminationReason.COMPLETED,
        var solutionPresence: RemoteSolutionPresence = RemoteSolutionPresence.NONE,
        var proofStatus: RemoteProofStatus = RemoteProofStatus.NONE,
        var provenance: Map<String, String> = emptyMap(),
        var fingerprints: Map<String, String> = emptyMap(),
        var fingerprintSchemas: Map<String, String> = emptyMap(),
        var statistics: Map<String, String> = emptyMap(),
        var diagnostics: Map<String, String> = emptyMap(),
        var runId: String? = null,
        var attemptId: String? = null,
        var artifactDigest: String? = null,
        var objectiveValueInt64: Long? = null,
        var modelFingerprint: String? = null,
        var snapshotJson: String? = null,
        var inputCheckpointId: String? = null
    )

    private val contexts = linkedMapOf<String, Context>()

    /**
     * 启动求解任务
     * Start a solving task
     */
    override suspend fun start(
        payload: SolvePayload,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle {
        val handle = newHandle(taskId, sliceId, nodeId)
        contexts[handle.handleId.value] = Context(
            payload = payload,
            tenantId = tenantId,
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId
        )
        return handle
    }

    /**
     * 从检查点恢复求解任务
     * Resume a solving task from checkpoint
     */
    override suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: String,
        sliceId: String,
        nodeId: String,
        tenantId: String
    ): ExecutionHandle {
        val handle = newHandle(taskId, sliceId, nodeId)
        contexts[handle.handleId.value] = Context(
            payload = payload.copy(snapshotRef = checkpoint),
            tenantId = tenantId,
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            checkpointRef = checkpoint
        )
        return handle
    }

    /**
     * 等待切片执行结束
     * Await slice execution end
     *
     * 构建命令行参数并调用外部进程。
     * Builds command line args and invokes external process.
     * 解析进程输出并更新执行上下文。
     * Parses process output and updates execution context.
     */
    override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult {
        val context = contexts[handle.handleId.value]
            ?: return backendFailureSliceResult(
                sliceId = handle.sliceId.value,
                message = "Execution handle not found"
            )
        // A resumed input is only a source for this slice.  It must never be re-exported
        // when the worker does not produce a new checkpoint. / 恢复输入只属于本切片的起点；
        // worker 未生成新 checkpoint 时不得再次导出旧引用。
        context.checkpointRef = null

        val command = args["command"]?.trim()
            ?: return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "Bridge arg 'command' is required for OspfExternalProcessBridge"
            )
        val inputDir = prepareInputDirectory(context, handle)
        val modelPath = materializeModel(context, inputDir)
        if (context.payload.modelData.format == "ospf-cp-snapshot-json" && context.payload.snapshotRef != null) {
            val checkpointError = validateInputCheckpoint(context)
            if (checkpointError != null) {
                return backendFailureSliceResult(
                    context = context,
                    sliceId = handle.sliceId.value,
                    message = checkpointError
                )
            }
        }
        val checkpointInputPath = materializeCheckpoint(context, inputDir)
        val commandArgs = mutableListOf<String>().apply {
            addAll(splitCommand(command))
            modelPath?.let {
                add("--model")
                add(it)
            }
            context.payload.modelData.format?.let {
                add("--model-format")
                add(it)
            }
            add("--task")
            add(context.taskId)
            add("--slice")
            add(context.sliceId)
            add("--node")
            add(context.nodeId)
            add("--quantum-ms")
            add(max(1L, quantumMs).toString())
            add("--tenant-id")
            add(context.tenantId)
            context.payload.config?.let {
                add("--config-json")
                add(Json { encodeDefaults = true }.encodeToString(SolverConfig.serializer(), it))
            }
            context.payload.taskMeta.timeLimitMs?.let {
                add("--task-time-limit-ms")
                add(it.toString())
            }
            context.payload.taskMeta.solutionLimit?.let {
                add("--task-solution-limit")
                add(it.toString())
            }
            checkpointInputPath?.let {
                add("--checkpoint-in")
                add(it)
            }
        }

        val processBuilder = ProcessBuilder(commandArgs)
        args["workdir"]?.takeIf { it.isNotBlank() }?.let { processBuilder.directory(java.io.File(it)) }
        val process = processBuilder.start()
        val stdout = process.inputStream.bufferedReader().use { it.readText() }
        val stderr = process.errorStream.bufferedReader().use { it.readText() }
        val exitCode = process.waitFor()
        if (exitCode != 0) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "Process failed: exitCode=$exitCode, stderr=${stderr.take(512)}"
            )
        }
        val parsed = parseKeyValues(stdout)
        val parsedCompleted = parsed["completed"]?.toBooleanStrictOrNull() ?: false
        val feasible = parsed["feasible"]?.toBooleanStrictOrNull() ?: parsedCompleted
        val objectiveValue = parsed["objective"]?.toDoubleOrNull()
        val gap = parsed["gap"]?.toDoubleOrNull()
        val elapsedMs = parsed["elapsedMs"]?.toLongOrNull() ?: max(1L, quantumMs)
        val message = parsed["message"] ?: "External process completed"
        val strictCp = context.payload.modelData.format == "ospf-cp-snapshot-json"
        val checkpointOutputPath = parsed["checkpointPath"]?.takeIf { it.isNotBlank() }
        var checkpointEnvelope: PortableCheckpointEnvelope? = null
        var pendingCheckpointBytes: ByteArray? = null
        var pendingCheckpointPath: String? = null
        checkpointOutputPath?.let { path ->
            val file = java.io.File(path)
            if (!file.isFile) {
                if (strictCp) {
                    return backendFailureSliceResult(
                        context = context,
                        sliceId = handle.sliceId.value,
                        message = "External CP checkpointPath does not point to a readable file"
                    )
                }
                null
            } else {
                val bytes = file.readBytes()
                val checkpoint = if (strictCp) {
                    PortableCheckpointCodec.decodeCompatibleOrNull(bytes.decodeToString())
                } else {
                    null
                }
                if (strictCp && (checkpoint == null || !validateStrictCheckpoint(checkpoint, context, output = true))) {
                    return backendFailureSliceResult(
                        context = context,
                        sliceId = handle.sliceId.value,
                        message = "External CP checkpoint failed integrity or schema validation"
                    )
                }
                checkpointEnvelope = checkpoint
                pendingCheckpointBytes = bytes
                pendingCheckpointPath = path
            }
        }
        val resultPath = parsed["resultPath"]?.takeIf { it.isNotBlank() }
        var artifactSolution: SerializedSolution? = null
        var pendingResultBytes: ByteArray? = null
        var pendingResultPath: String? = null
        resultPath?.let { path ->
            val resultFile = java.io.File(path)
            if (!resultFile.isFile) {
                if (strictCp) {
                    return backendFailureSliceResult(
                        context = context,
                        sliceId = handle.sliceId.value,
                        message = "External CP resultPath does not point to a readable file"
                    )
                }
                null
            } else {
                val resultBytes = resultFile.readBytes()
                val solution = runCatching {
                    Json { ignoreUnknownKeys = !strictCp }.decodeFromString(
                        SerializedSolution.serializer(),
                        resultBytes.decodeToString()
                    )
                }.getOrNull()
                if (solution == null || (strictCp && !validateStrictSolution(resultBytes.decodeToString(), solution, context))) {
                    return backendFailureSliceResult(
                        context = context,
                        sliceId = handle.sliceId.value,
                        message = "External CP result artifact failed strict audit validation"
                    )
                } else {
                    artifactSolution = solution
                    pendingResultBytes = resultBytes
                    pendingResultPath = path
                }
            }
        }

        val stdoutJson = stdout.lineSequence()
            .map(String::trim)
            .firstOrNull { it.startsWith("{") && it.endsWith("}") }
        val stdoutSolution = stdoutJson?.let {
            runCatching {
                Json { ignoreUnknownKeys = !strictCp }.decodeFromString(
                    SerializedSolution.serializer(),
                    it
                )
            }.getOrNull()
        }

        if (strictCp && stdoutSolution != null &&
            !validateStrictSolution(stdoutJson!!, stdoutSolution, context)
        ) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "External CP SerializedSolution failed strict audit validation"
            )
        }
        if (strictCp && artifactSolution != null && stdoutSolution != null && artifactSolution != stdoutSolution) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "External CP stdout and result artifact disagree"
            )
        }
        if (strictCp && stdoutSolution == null && artifactSolution == null) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "External CP process did not return a strict SerializedSolution artifact"
            )
        }

        if (pendingResultBytes == null && stdoutSolution != null) {
            pendingResultBytes = stdoutJson!!.toByteArray()
            pendingResultPath = "stdout"
        }

        val canonicalSolution = if (strictCp) artifactSolution ?: stdoutSolution else stdoutSolution
        if (strictCp && checkpointEnvelope != null && canonicalSolution != null &&
            !checkpointMatchesSolution(checkpointEnvelope!!, canonicalSolution)
        ) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "External CP checkpoint and result artifact disagree"
            )
        }
        val acceptedCompleted = canonicalSolution?.let { it.terminationReason !in nonTerminalReasons }
            ?: parsed["completed"]?.toBooleanStrictOrNull()
            ?: parsedCompleted
        context.completed = acceptedCompleted
        context.feasible = canonicalSolution?.feasible ?: feasible
        context.optimal = canonicalSolution?.optimal ?: (parsed["optimal"]?.toBooleanStrictOrNull() ?: false)
        context.objectiveValue = canonicalSolution?.objectiveValue?.toDouble() ?: objectiveValue
        context.objectiveValueInt64 = canonicalSolution?.objectiveValueInt64
        context.gap = canonicalSolution?.gap?.toDouble() ?: gap
        val canonicalElapsedMs = canonicalSolution?.elapsed?.inWholeMilliseconds ?: elapsedMs
        if (strictCp && canonicalSolution != null && canonicalElapsedMs != elapsedMs) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "External CP raw and artifact elapsedMs disagree"
            )
        }
        // A non-terminal strict CP slice is resumable only when it produced a fresh checkpoint.
        // 在严格 CP 模式下，非终态切片只有产出新 checkpoint 才允许持久化和挂起。
        if (strictCp && !acceptedCompleted && pendingCheckpointBytes == null) {
            return backendFailureSliceResult(
                context = context,
                sliceId = handle.sliceId.value,
                message = "External CP process did not produce a new checkpoint",
                incumbent = canonicalSolution
            )
        }
        val checkpointRef = pendingCheckpointBytes?.let { bytes ->
            objectStoragePort.put(
                path = "${context.tenantId}/checkpoint/${context.taskId}/${context.sliceId}",
                bytes = bytes,
                metadata = mapOf(
                    "externalCheckpointPath" to (pendingCheckpointPath ?: "unknown"),
                    "tenantId" to context.tenantId,
                    "taskId" to context.taskId,
                    "sliceId" to context.sliceId,
                    "contentType" to "application/json",
                    "schemaVersion" to (checkpointEnvelope?.schemaVersion ?: ""),
                    "sourceFormat" to (checkpointEnvelope?.sourceFormat ?: ""),
                    "modelFingerprint" to (checkpointEnvelope?.modelFingerprint ?: ""),
                    "configurationFingerprint" to (checkpointEnvelope?.configurationFingerprint ?: ""),
                    "solverFingerprint" to (checkpointEnvelope?.solverFingerprint ?: ""),
                    "runId" to (checkpointEnvelope?.runId ?: ""),
                    "attemptId" to (checkpointEnvelope?.attemptId ?: ""),
                    "integritySha256" to (checkpointEnvelope?.integritySha256 ?: "")
                )
            )
        }
        val effectiveResultRef = pendingResultBytes?.let { bytes ->
            objectStoragePort.put(
                path = "${context.tenantId}/result/${context.taskId}/${context.sliceId}",
                bytes = bytes,
                metadata = mapOf(
                    "externalResultPath" to (pendingResultPath ?: "unknown"),
                    "tenantId" to context.tenantId,
                    "taskId" to context.taskId,
                    "sliceId" to context.sliceId,
                    "contentType" to "application/json",
                    "schemaVersion" to (canonicalSolution?.schemaVersion ?: ""),
                    "modelFingerprint" to (canonicalSolution?.fingerprints?.get("model") ?: ""),
                    "configurationFingerprint" to (canonicalSolution?.fingerprints?.get("configuration") ?: ""),
                    "solverFingerprint" to (canonicalSolution?.fingerprints?.get("solver") ?: ""),
                    "runId" to (canonicalSolution?.runId ?: ""),
                    "attemptId" to (canonicalSolution?.attemptId ?: ""),
                    "artifactDigest" to (canonicalSolution?.artifactDigest ?: "")
                )
            )
        }
        context.elapsedMs += canonicalElapsedMs
        context.checkpointRef = checkpointRef
        context.resultRef = effectiveResultRef ?: context.resultRef
        context.message = message
        context.schemaVersion = canonicalSolution?.schemaVersion ?: "1.0"
        context.problemStatus = canonicalSolution?.problemStatus ?: if (context.feasible) {
            RemoteProblemStatus.FEASIBLE
        } else {
            RemoteProblemStatus.UNKNOWN
        }
        context.terminationReason = canonicalSolution?.terminationReason
            ?: if (context.completed) RemoteTerminationReason.COMPLETED else RemoteTerminationReason.BACKEND_FAILURE
        context.solutionPresence = canonicalSolution?.solutionPresence ?: if (context.feasible) {
            RemoteSolutionPresence.INCUMBENT
        } else {
            RemoteSolutionPresence.NONE
        }
        context.proofStatus = canonicalSolution?.proofStatus ?: RemoteProofStatus.NONE
        context.provenance = canonicalSolution?.provenance ?: emptyMap()
        context.fingerprints = canonicalSolution?.fingerprints ?: emptyMap()
        context.fingerprintSchemas = canonicalSolution?.fingerprintSchemas ?: emptyMap()
        context.statistics = canonicalSolution?.statistics ?: emptyMap()
        context.diagnostics = canonicalSolution?.diagnostics ?: emptyMap()
        context.runId = canonicalSolution?.runId
        context.attemptId = canonicalSolution?.attemptId
        context.artifactDigest = canonicalSolution?.artifactDigest

        return SliceResult(
            sliceId = handle.sliceId,
            completed = context.completed,
            feasible = context.feasible,
            objectiveValue = context.objectiveValue?.let(::Flt64),
            objectiveValueInt64 = context.objectiveValueInt64,
            gap = context.gap?.let(::Flt64),
            elapsed = canonicalElapsedMs.toDuration(DurationUnit.MILLISECONDS),
            message = canonicalSolution?.message ?: message,
            schemaVersion = context.schemaVersion,
            problemStatus = context.problemStatus,
            terminationReason = context.terminationReason,
            solutionPresence = context.solutionPresence,
            proofStatus = context.proofStatus,
            resultRef = context.resultRef,
            provenance = context.provenance,
            fingerprints = context.fingerprints,
            fingerprintSchemas = context.fingerprintSchemas,
            statistics = context.statistics,
            diagnostics = context.diagnostics,
            runId = context.runId,
            attemptId = context.attemptId,
            artifactDigest = context.artifactDigest
        )
    }

    /**
     * 导出检查点
     * Export checkpoint
     *
     * 返回上下文中保存的检查点引用。
     * Returns checkpoint reference saved in context.
     */
    override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? =
        contexts[handle.handleId.value]?.checkpointRef

    /**
     * 获取最终求解结果
     * Fetch final solving result
     *
     * 如果求解已完成，返回包含所有结果信息的 SolveResult。
     * If solving is complete, returns SolveResult with all result info.
     */
    override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? {
        val context = contexts[handle.handleId.value] ?: return null
        if (!context.completed) {
            return null
        }
        return SolveResult(
            feasible = context.feasible,
            optimal = context.optimal,
            objectiveValue = context.objectiveValue?.let(::Flt64),
            objectiveValueInt64 = context.objectiveValueInt64,
            gap = context.gap?.let(::Flt64),
            elapsed = context.elapsedMs.toDuration(DurationUnit.MILLISECONDS),
            checkpointRef = context.checkpointRef,
            resultRef = context.resultRef,
            message = context.message,
            schemaVersion = context.schemaVersion,
            problemStatus = context.problemStatus,
            terminationReason = context.terminationReason,
            solutionPresence = context.solutionPresence,
            proofStatus = context.proofStatus,
            provenance = context.provenance,
            fingerprints = context.fingerprints,
            fingerprintSchemas = context.fingerprintSchemas,
            statistics = context.statistics,
            diagnostics = context.diagnostics,
            runId = context.runId,
            attemptId = context.attemptId,
            artifactDigest = context.artifactDigest
        )
    }

    /**
     * 停止求解执行
     * Stop solving execution
     *
     * 从上下文映射中移除对应的执行状态。
     * Removes corresponding execution state from context map.
     */
    override suspend fun stop(handle: ExecutionHandle): Boolean =
        contexts.remove(handle.handleId.value) != null

    private fun backendFailureSliceResult(
        context: Context? = null,
        sliceId: String,
        message: String,
        incumbent: SerializedSolution? = null
    ): SliceResult {
        val retainedIncumbent = incumbent?.takeIf {
            it.feasible && it.solutionPresence == RemoteSolutionPresence.INCUMBENT
        }
        val retainedElapsed = retainedIncumbent?.elapsed ?: 0L.toDuration(DurationUnit.MILLISECONDS)
        context?.apply {
            completed = true
            feasible = retainedIncumbent != null
            optimal = false
            objectiveValue = retainedIncumbent?.objectiveValue?.toDouble()
            objectiveValueInt64 = retainedIncumbent?.objectiveValueInt64
            gap = retainedIncumbent?.gap?.toDouble()
            elapsedMs = retainedElapsed.inWholeMilliseconds
            checkpointRef = null
            resultRef = null
            this.message = message
            schemaVersion = "2.0"
            problemStatus = if (retainedIncumbent != null) {
                RemoteProblemStatus.FEASIBLE
            } else {
                RemoteProblemStatus.UNKNOWN
            }
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE
            solutionPresence = if (retainedIncumbent != null) {
                RemoteSolutionPresence.INCUMBENT
            } else {
                RemoteSolutionPresence.NONE
            }
            proofStatus = RemoteProofStatus.NONE
            provenance = retainedIncumbent?.provenance ?: emptyMap()
            fingerprints = retainedIncumbent?.fingerprints ?: emptyMap()
            fingerprintSchemas = retainedIncumbent?.fingerprintSchemas ?: emptyMap()
            statistics = retainedIncumbent?.statistics ?: emptyMap()
            diagnostics = retainedIncumbent?.diagnostics ?: emptyMap()
            runId = retainedIncumbent?.runId ?: taskId
            attemptId = retainedIncumbent?.attemptId ?: sliceId
            artifactDigest = null
        }
        return SliceResult(
            sliceId = SliceId.of(sliceId),
            completed = true,
            feasible = retainedIncumbent != null,
            objectiveValue = retainedIncumbent?.objectiveValue,
            objectiveValueInt64 = retainedIncumbent?.objectiveValueInt64,
            gap = retainedIncumbent?.gap,
            elapsed = retainedElapsed,
            message = message,
            schemaVersion = "2.0",
            problemStatus = if (retainedIncumbent != null) {
                RemoteProblemStatus.FEASIBLE
            } else {
                RemoteProblemStatus.UNKNOWN
            },
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
            solutionPresence = if (retainedIncumbent != null) {
                RemoteSolutionPresence.INCUMBENT
            } else {
                RemoteSolutionPresence.NONE
            },
            proofStatus = RemoteProofStatus.NONE,
            provenance = retainedIncumbent?.provenance ?: emptyMap(),
            fingerprints = retainedIncumbent?.fingerprints ?: emptyMap(),
            fingerprintSchemas = retainedIncumbent?.fingerprintSchemas ?: emptyMap(),
            statistics = retainedIncumbent?.statistics ?: emptyMap(),
            diagnostics = retainedIncumbent?.diagnostics ?: emptyMap(),
            runId = retainedIncumbent?.runId ?: context?.taskId,
            attemptId = retainedIncumbent?.attemptId ?: context?.sliceId
        )
    }

    /**
     * Cross-check checkpoint state against the result artifact from the same slice.
     * 交叉校验同一切片 checkpoint 与结果 artifact 的状态。
     *
     * @param checkpoint Output checkpoint envelope. / 输出 checkpoint envelope。
     * @param solution Validated result artifact. / 已验证的结果 artifact。
     * @return Whether both artifacts describe the same incumbent and bounds. / 两个 artifact 是否描述相同 incumbent 与界限。
     */
    private fun checkpointMatchesSolution(
        checkpoint: PortableCheckpointEnvelope,
        solution: SerializedSolution
    ): Boolean {
        // Plain CP result artifacts do not carry Benders state; accepting it here would allow
        // unverifiable evidence to enter storage. / 普通 CP 结果不携带 Benders 状态；否则无法复验的证据可能进入存储。
        if (checkpoint.benders != null) {
            return false
        }
        val assumptions = decodeEvidenceList(solution.diagnostics["infeasibility.assumptionIds"]) ?: return false
        val members = decodeEvidenceList(solution.diagnostics["infeasibility.members"]) ?: return false
        val expectedConflicts = if (members.isEmpty()) {
            emptyList()
        } else {
            listOf(
                PortableConstraintProgrammingConflict(
                    validity = solution.diagnostics["infeasibility.validity"] ?: "Unknown",
                    minimality = solution.diagnostics["infeasibility.minimality"] ?: "NotChecked",
                    memberIds = members,
                    assumptionIds = assumptions,
                    provenance = solution.diagnostics
                        .filterKeys { it.startsWith("infeasibility.") }
                        .filterValues { it.isNotBlank() }
                )
            )
        }
        if (checkpoint.assumptions != assumptions || checkpoint.conflicts != expectedConflicts) {
            return false
        }
        if (checkpoint.bestBound != solution.statistics["bestBound"] ||
            checkpoint.gap != (solution.gap?.toString() ?: solution.statistics["gap"])
        ) {
            return false
        }
        val incumbent = checkpoint.incumbent
        if (!solution.feasible) {
            return incumbent == null && solution.objectiveValue == null && solution.objectiveValueInt64 == null
        }
        if (incumbent == null ||
            incumbent.valuesById != solution.variableValuesById ||
            incumbent.intervalsById.keys != solution.intervalValues.keys
        ) {
            return false
        }
        if (incumbent.intervalsById.any { (id, value) ->
                val supplied = solution.intervalValues[id]
                supplied == null ||
                    supplied.start != value.start ||
                    supplied.size != value.size ||
                    supplied.end != value.end ||
                    supplied.present != value.present
            }
        ) {
            return false
        }
        val expectedObjective = solution.objectiveValueInt64?.toString() ?: solution.objectiveValue?.toString()
        if (expectedObjective != null) {
            if (incumbent.objective != expectedObjective) {
                return false
            }
        } else if (incumbent.objective != null &&
            solution.terminationReason != RemoteTerminationReason.BACKEND_FAILURE
        ) {
            // A target-bearing model must agree on its exact objective. A target-free model
            // legitimately carries no objective in either artifact. / 含目标模型必须逐字一致；
            // 无目标模型允许两个 artifact 同时没有目标值。
            return false
        }
        return true
    }

    private fun decodeEvidenceList(encoded: String?): List<String>? {
        if (encoded.isNullOrBlank()) {
            return emptyList()
        }
        return runCatching {
            Json.decodeFromString<List<String>>(encoded)
        }.getOrNull()?.takeIf { values -> values.all(String::isNotBlank) }
    }

    /**
     * 创建新的执行句柄
     * Create new execution handle
     *
     * @param taskId 任务ID
     *                Task ID
     * @param sliceId 切片ID
     *                 Slice ID
     * @param nodeId 节点ID
     *                Node ID
     * @return 新的执行句柄
     *         New execution handle
     */
    private fun newHandle(taskId: String, sliceId: String, nodeId: String): ExecutionHandle =
        ExecutionHandle(
            handleId = idGenerator.newId("handle"),
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            startedAtEpochMs = clock.nowEpochMs()
        )

    /**
     * 分割命令字符串
     * Split command string
     *
     * 将带引号的命令字符串分割为参数列表。
     * Splits quoted command string into argument list.
     * 支持空格分隔和双引号包裹的参数。
     * Supports space-separated and double-quote-wrapped arguments.
     *
     * @param command 原始命令字符串
     *                 Original command string
     * @return 分割后的参数列表
     *         Split argument list
     */
    private fun splitCommand(command: String): List<String> {
        val tokens = mutableListOf<String>()
        val current = StringBuilder()
        var inQuotes = false
        var i = 0
        while (i < command.length) {
            val ch = command[i]
            when {
                ch == '"' -> {
                    inQuotes = !inQuotes
                }

                ch.isWhitespace() && !inQuotes -> {
                    if (current.isNotEmpty()) {
                        tokens += current.toString()
                        current.clear()
                    }
                }

                else -> current.append(ch)
            }
            i += 1
        }
        if (current.isNotEmpty()) {
            tokens += current.toString()
        }
        return tokens
    }

    private fun prepareInputDirectory(context: Context, handle: ExecutionHandle): Path {
        val root = args["workdir"]?.takeIf { it.isNotBlank() }?.let(Path::of)
            ?: Path.of(System.getProperty("java.io.tmpdir"), "ospf-remote-solver-input")
        val directory = root.resolve(
            "${safePathComponent(context.taskId)}-${safePathComponent(handle.sliceId.value)}"
        ).toAbsolutePath().normalize()
        Files.createDirectories(directory)
        return directory
    }

    private suspend fun materializeModel(context: Context, directory: Path): String? {
        val bytes = context.payload.modelData.rawBytes
            ?: context.payload.modelData.ref?.let { objectStoragePort.get(it) }
        if (bytes == null) {
            return context.payload.modelData.ref?.path?.value
        }
        context.modelFingerprint = sha256(bytes.decodeToString())
        if (context.payload.modelData.format == "ospf-cp-snapshot-json") {
            context.snapshotJson = bytes.decodeToString()
        }
        val path = directory.resolve("model.${context.payload.modelData.format ?: "bin"}")
        Files.write(path, bytes)
        return path.toString()
    }

    private suspend fun materializeCheckpoint(context: Context, directory: Path): String? {
        val reference = context.payload.snapshotRef ?: return null
        val bytes = objectStoragePort.get(reference) ?: return reference.path.value
        val path = directory.resolve("checkpoint.json")
        Files.write(path, bytes)
        return path.toString()
    }

    /**
     * Validate an input CP checkpoint before invoking an external process.
     * 在启动外部进程前校验输入 CP checkpoint。
     *
     * @param context Execution context containing the model and checkpoint reference.
     * 包含模型和 checkpoint 引用的执行上下文。
     * @return Null when valid, otherwise a structured backend-failure message.
     * 校验成功返回 null，否则返回结构化后端失败消息。
     */
    private suspend fun validateInputCheckpoint(context: Context): String? {
        val reference = context.payload.snapshotRef
            ?: return null
        val bytes = objectStoragePort.get(reference)
            ?: return "External CP checkpoint object does not exist"
        val checkpoint = PortableCheckpointCodec.decodeCompatibleOrNull(bytes.decodeToString())
            ?: return "External CP checkpoint failed integrity or schema validation"
        if (!validateStrictCheckpoint(checkpoint, context, output = false)) {
            return "External CP checkpoint does not match this task, model, configuration, or solver"
        }
        context.inputCheckpointId = checkpoint.checkpointId
        return null
    }

    /**
     * 解析键值对输出
     * Parse key-value output
     *
     * 将外部进程的标准输出解析为键值对映射。
     * Parses external process stdout into key-value map.
     * 输出格式为每行一个 "key=value" 条目。
     * Output format is one "key=value" entry per line.
     *
     * @param content 外部进程的输出内容
     *                External process output content
     * @return 解析后的键值对映射
     *         Parsed key-value map
     */
    private fun parseKeyValues(content: String): Map<String, String> =
        content.lineSequence()
            .map { it.trim() }
            .filter { it.isNotEmpty() }
            .mapNotNull { line ->
                val idx = line.indexOf('=')
                if (idx <= 0) {
                    null
                } else {
                    line.substring(0, idx).trim() to line.substring(idx + 1).trim()
                }
            }
            .toMap()

    private fun validateStrictSolution(
        encoded: String,
        solution: SerializedSolution,
        context: Context
    ): Boolean {
        val root = runCatching {
            Json { ignoreUnknownKeys = false }.parseToJsonElement(encoded).jsonObject
        }.getOrNull() ?: return false
        val required = setOf(
            "schemaVersion", "feasible", "optimal", "objectiveValue", "objectiveValueInt64",
            "elapsedMs", "message", "variableValuesById", "intervalValues", "problemStatus",
            "solutionPresence", "proofStatus", "terminationReason", "gap", "runId", "attemptId", "artifactDigest",
            "provenance", "fingerprints", "fingerprintSchemas", "statistics", "diagnostics"
        )
        if (solution.schemaVersion != "2.0" ||
            required.any { it !in root.keys } ||
            solution.runId != context.taskId ||
            solution.attemptId != context.sliceId ||
            solution.artifactDigest.isNullOrBlank() ||
            solution.problemStatus == null ||
            solution.solutionPresence == null ||
            solution.proofStatus == null ||
            solution.terminationReason == null ||
            solution.runId.isNullOrBlank() ||
            solution.attemptId.isNullOrBlank()
        ) {
            return false
        }
        if (!validateStatusCombination(solution)) {
            return false
        }
        val unsigned = solution.copy(artifactDigest = null)
        val canonical = Json { encodeDefaults = true; ignoreUnknownKeys = false }
            .encodeToString(SerializedSolution.serializer(), unsigned)
        val actual = MessageDigest.getInstance("SHA-256")
            .digest(canonical.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
        if (actual != solution.artifactDigest ||
            solution.fingerprints.keys != solution.fingerprintSchemas.keys ||
            solution.fingerprintSchemas.values.any(String::isBlank) ||
            solution.fingerprints.values.any(String::isBlank) ||
            solution.fingerprints["model"].isNullOrBlank() ||
            solution.fingerprints["configuration"].isNullOrBlank() ||
            solution.fingerprints["solver"].isNullOrBlank() ||
            (solution.fingerprints["solver"] != scipRuntimeFingerprint() &&
                solution.fingerprints["solver"] !in scipRuntimeFingerprintLegacyV1Candidates())
        ) {
            return false
        }
        if (context.modelFingerprint == null || solution.fingerprints["model"] != context.modelFingerprint) {
            return false
        }
        if (solution.fingerprints["configuration"] != configurationFingerprint(context.payload)) {
            return false
        }
        val snapshot = context.snapshotJson ?: return false
        return OspfCpSnapshotExecutor(objectStoragePort).validateExternalSolution(snapshot, solution)
    }

    private fun validateStrictCheckpoint(
        checkpoint: fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope,
        context: Context,
        output: Boolean
    ): Boolean {
        if (checkpoint.sourceFormat !in setOf("v2", "legacy-v1")) {
            return false
        }
        if (checkpoint.sourceFormat == "v2" && checkpoint.schemaVersion != "2.0") {
            return false
        }
        if (checkpoint.sourceFormat == "v2" && checkpoint.runId != context.taskId) {
            return false
        }
        if (checkpoint.sourceFormat == "v2" && checkpoint.attemptId.isNullOrBlank()) {
            return false
        }
        if (output && (checkpoint.sourceFormat != "v2" || checkpoint.migratedFromLegacy ||
                checkpoint.attemptId != context.sliceId || checkpoint.runId != context.taskId ||
                checkpoint.checkpointId == "legacy-v1" ||
                checkpoint.checkpointId == context.inputCheckpointId ||
                checkpoint.parentCheckpointId != context.inputCheckpointId)) {
            return false
        }
        if (checkpoint.sourceFormat == "legacy-v1") {
            val snapshot = context.snapshotJson ?: return false
            return checkpoint.modelFingerprint == sha256(snapshot) &&
                OspfCpSnapshotExecutor(objectStoragePort).validateExternalCheckpoint(snapshot, checkpoint)
        }
        if (checkpoint.modelFingerprint.isBlank() ||
            checkpoint.configurationFingerprint.isNullOrBlank() ||
            checkpoint.solverFingerprint.isNullOrBlank()
        ) {
            return false
        }
        if (checkpoint.solverFingerprint != scipRuntimeFingerprint() &&
            checkpoint.solverFingerprint !in scipRuntimeFingerprintLegacyV1Candidates()
        ) {
            return false
        }
        if (context.modelFingerprint == null || checkpoint.modelFingerprint != context.modelFingerprint) {
            return false
        }
        if (checkpoint.configurationFingerprint != configurationFingerprint(context.payload)) {
            return false
        }
        val snapshot = context.snapshotJson ?: return false
        return OspfCpSnapshotExecutor(objectStoragePort).validateExternalCheckpoint(snapshot, checkpoint)
    }

    private fun validateStatusCombination(solution: SerializedSolution): Boolean {
        return when (solution.problemStatus) {
            RemoteProblemStatus.FEASIBLE -> solution.feasible &&
                solution.solutionPresence != RemoteSolutionPresence.NONE &&
                solution.optimal == (solution.solutionPresence == RemoteSolutionPresence.OPTIMAL) &&
                (solution.solutionPresence != RemoteSolutionPresence.OPTIMAL ||
                    solution.proofStatus == RemoteProofStatus.VERIFIED)

            RemoteProblemStatus.INFEASIBLE,
            RemoteProblemStatus.UNBOUNDED,
            RemoteProblemStatus.INFEASIBLE_OR_UNBOUNDED -> !solution.feasible &&
                !solution.optimal &&
                solution.solutionPresence == RemoteSolutionPresence.NONE &&
                solution.objectiveValue == null &&
                solution.objectiveValueInt64 == null &&
                (solution.proofStatus == RemoteProofStatus.CLAIMED ||
                    solution.proofStatus == RemoteProofStatus.VERIFIED)

            RemoteProblemStatus.UNKNOWN -> !solution.feasible &&
                !solution.optimal &&
                solution.solutionPresence == RemoteSolutionPresence.NONE &&
                solution.objectiveValue == null &&
                solution.objectiveValueInt64 == null &&
                solution.proofStatus != RemoteProofStatus.VERIFIED

            null -> false
        }
    }

    private fun configurationFingerprint(payload: SolvePayload): String {
        val config = payload.config ?: SolverConfig(timeLimit = null)
        val effective = config.copy(
            timeLimit = payload.config?.timeLimit ?: payload.taskMeta.timeLimit,
            solutionLimit = payload.config?.solutionLimit ?: payload.taskMeta.solutionLimit,
            threads = config.threads ?: 8
        )
        val canonical = buildString {
            appendCanonical("timeLimitMs", effective.timeLimitMs?.toString())
            appendCanonical("solutionLimit", effective.solutionLimit?.toString())
            appendCanonical("mipGapTolerance", effective.mipGapTolerance?.toString())
            appendCanonical("threads", effective.threads?.toString())
            effective.solverParams.toSortedMap().forEach { (key, value) ->
                appendCanonical("solverParam.key", key)
                appendCanonical("solverParam.value", value)
            }
        }
        return sha256(canonical)
    }

    private fun StringBuilder.appendCanonical(name: String, value: String?) {
        append(name.length).append(':').append(name)
            .append(value?.length ?: -1).append(':').append(value ?: "")
    }

    private fun sha256(value: String): String {
        return MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }

    private fun safePathComponent(value: String): String =
        value.map { ch ->
            when {
                ch.isLetterOrDigit() || ch == '_' || ch == '-' || ch == '.' -> ch
                else -> '_'
            }
        }.joinToString("").ifEmpty { "unknown" }

    private companion object {
        val nonTerminalReasons = setOf(
            RemoteTerminationReason.TIME_LIMIT,
            RemoteTerminationReason.NODE_LIMIT,
            RemoteTerminationReason.ITERATION_LIMIT
        )
    }
}
