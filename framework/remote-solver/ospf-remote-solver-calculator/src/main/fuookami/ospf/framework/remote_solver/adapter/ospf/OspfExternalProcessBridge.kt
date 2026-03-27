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

import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import kotlin.math.max

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
 * 输出格式（key=value 行）:
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
        var objectiveValue: Double? = null,
        var gap: Double? = null,
        var elapsedMs: Long = 0L,
        var checkpointRef: ObjectRef? = null,
        var resultRef: ObjectRef? = null,
        var message: String? = null
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
            ?: return SliceResult(
                sliceId = handle.sliceId.value,
                completed = false,
                feasible = false,
                objectiveValue = null,
                gap = null,
                elapsedMs = 0L,
                message = "Execution handle not found"
            )

        val command = args["command"]?.trim()
            ?: return SliceResult(
                sliceId = handle.sliceId.value,
                completed = false,
                feasible = false,
                objectiveValue = null,
                gap = null,
                elapsedMs = 0L,
                message = "Bridge arg 'command' is required for OspfExternalProcessBridge"
            )
        val commandArgs = mutableListOf<String>().apply {
            addAll(splitCommand(command))
            add("--model")
            context.payload.modelRef?.path?.let { add(it.value) }
            add("--task")
            add(context.taskId)
            add("--slice")
            add(context.sliceId)
            add("--node")
            add(context.nodeId)
            add("--quantum-ms")
            add(max(1L, quantumMs).toString())
            context.payload.snapshotRef?.path?.let {
                add("--checkpoint-in")
                add(it.value)
            }
        }

        val processBuilder = ProcessBuilder(commandArgs)
        args["workdir"]?.takeIf { it.isNotBlank() }?.let { processBuilder.directory(java.io.File(it)) }
        val process = processBuilder.start()
        val stdout = process.inputStream.bufferedReader().use { it.readText() }
        val stderr = process.errorStream.bufferedReader().use { it.readText() }
        val exitCode = process.waitFor()
        if (exitCode != 0) {
            return SliceResult(
                sliceId = handle.sliceId.value,
                completed = false,
                feasible = false,
                objectiveValue = null,
                gap = null,
                elapsedMs = 0L,
                message = "Process failed: exitCode=$exitCode, stderr=${stderr.take(512)}"
            )
        }
        val parsed = parseKeyValues(stdout)
        val completed = parsed["completed"]?.toBooleanStrictOrNull() ?: false
        val feasible = parsed["feasible"]?.toBooleanStrictOrNull() ?: completed
        val objectiveValue = parsed["objective"]?.toDoubleOrNull()
        val gap = parsed["gap"]?.toDoubleOrNull()
        val elapsedMs = parsed["elapsedMs"]?.toLongOrNull() ?: max(1L, quantumMs)
        val message = parsed["message"] ?: "External process completed"
        val checkpointPath = parsed["checkpointPath"]?.takeIf { it.isNotBlank() }
        val checkpointRef = checkpointPath?.let {
            val bytes = it.toByteArray()
            objectStoragePort.put(
                path = "${context.tenantId}/checkpoint/${context.taskId}/${context.sliceId}",
                bytes = bytes,
                metadata = mapOf("externalCheckpointPath" to it, "tenantId" to context.tenantId)
            )
        }
        val resultPath = parsed["resultPath"]?.takeIf { it.isNotBlank() }
        val resultRef = resultPath?.let {
            objectStoragePort.put(
                path = "${context.tenantId}/result/${context.taskId}/${context.sliceId}",
                bytes = it.toByteArray(),
                metadata = mapOf("externalResultPath" to it, "tenantId" to context.tenantId)
            )
        }

        context.completed = completed
        context.feasible = feasible
        context.objectiveValue = objectiveValue
        context.gap = gap
        context.elapsedMs += elapsedMs
        context.checkpointRef = checkpointRef ?: context.checkpointRef
        context.resultRef = resultRef ?: context.resultRef
        context.message = message

        return SliceResult(
            sliceId = handle.sliceId.value,
            completed = completed,
            feasible = feasible,
            objectiveValue = objectiveValue,
            gap = gap,
            elapsedMs = elapsedMs,
            message = message
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
            optimal = context.gap?.let { it <= 0.0 } ?: context.feasible,
            objectiveValue = context.objectiveValue,
            gap = context.gap,
            elapsedMs = context.elapsedMs,
            checkpointRef = context.checkpointRef,
            resultRef = context.resultRef,
            message = context.message
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
}
