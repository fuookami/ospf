@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * 远程求解器 Worker 启动入口
 * Remote solver worker bootstrap entry
 *
 * 用于独立进程模式的求解器 Worker，接收命令行参数执行求解任务。
 * Used for standalone process mode solver worker, receives command line arguments to execute solve tasks.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import java.nio.file.Files
import java.nio.file.Path
import kotlin.math.max
import kotlin.math.min
import kotlin.time.Instant
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import fuookami.ospf.framework.remote_solver.adapter.localfs.LocalFsObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfCpSnapshotExecutor
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverConfig
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort

/**
 * 远程求解器 Worker 主程序
 * Remote solver worker main
 *
 * 执行外部求解器进程，支持 CP snapshot 的真实 SCIP 求解和非 CP 兼容进度模式。
 * Executes the external solver process, supporting real SCIP CP solving for snapshots and a
 * compatibility progress mode for non-CP payloads.
 *
 * 命令行参数 / Command line arguments:
 * --task         任务 ID / Task ID
 * --slice        切片 ID / Slice ID
 * --model        模型标识 / Model identifier
 * --model-format 模型格式 / Model format
 * --quantum-ms   时间切片时长（毫秒） / Time slice duration in milliseconds
 * --tenant-id    租户 ID / Tenant identifier
 * --config-json  内联 SolverConfig JSON / Inline SolverConfig JSON
 * --total-runtime-ms 总运行时间（毫秒） / Total runtime in milliseconds
 * --checkpoint-in    输入检查点路径 / Input checkpoint path
 * --state-dir    状态目录 / State directory
 */
object RemoteSolverWorkerMain {
    private val json = Json {
        encodeDefaults = true
        ignoreUnknownKeys = false
    }

    @JvmStatic
    fun main(args: Array<String>) {
        val options = parseArgs(args)
        val taskId = options["task"]?.trim().takeUnless { it.isNullOrEmpty() } ?: "task-unknown"
        val sliceId = options["slice"]?.trim().takeUnless { it.isNullOrEmpty() } ?: "slice-unknown"
        val model = options["model"]?.trim().takeUnless { it.isNullOrEmpty() } ?: "model-unknown"
        val quantumMs = options["quantum-ms"]?.toLongOrNull()?.coerceAtLeast(1L) ?: 1000L
        val totalRuntimeMs = options["total-runtime-ms"]?.toLongOrNull()?.coerceAtLeast(1L) ?: 12_000L
        val checkpointIn = options["checkpoint-in"]?.trim()?.takeUnless { it.isEmpty() }
        val stateDir = Path.of(
            options["state-dir"]?.trim().takeUnless { it.isNullOrEmpty() } ?: "target/remote-solver-worker-state"
        ).toAbsolutePath().normalize()
        Files.createDirectories(stateDir)

        if (options["model-format"] == "ospf-cp-snapshot-json") {
            runCpSnapshot(options, taskId, sliceId, quantumMs, stateDir)
            return
        }

        runCompatibilityProgress(options, taskId, sliceId, model, quantumMs, totalRuntimeMs, checkpointIn, stateDir)
    }

    private fun runCpSnapshot(
        options: Map<String, String>,
        taskId: String,
        sliceId: String,
        quantumMs: Long,
        stateDir: Path
    ) {
        val modelPath = options["model"]?.trim().takeUnless { it.isNullOrEmpty() }
        val tenantId = options["tenant-id"]?.trim().takeUnless { it.isNullOrEmpty() } ?: "default"
        val resultPath = stateDir.resolve("results").resolve(sanitize(taskId)).resolve("${sanitize(sliceId)}.json")
        val checkpointPath = stateDir.resolve("checkpoints").resolve(sanitize(taskId)).resolve("${sanitize(sliceId)}.json")
        runCatching {
            require(modelPath != null) { "CP model path is required" }
            val modelBytes = Files.readAllBytes(Path.of(modelPath))
            val config = options["config-json"]?.let { encoded ->
                json.decodeFromString(SolverConfig.serializer(), encoded)
            }
            val taskMeta = TaskMeta(
                timeLimitMs = options["task-time-limit-ms"]?.toLongOrNull(),
                solutionLimit = options["task-solution-limit"]?.toIntOrNull()
            )
            val payload = SolvePayload(
                modelData = ModelData.raw(modelBytes, "ospf-cp-snapshot-json"),
                config = config,
                taskMeta = taskMeta
            )
            val storage = LocalFsObjectStoragePort(
                rootPath = stateDir.resolve("objects"),
                clock = object : ClockPort {
                    override fun now(): Instant = Instant.fromEpochMilliseconds(System.currentTimeMillis())
                }
            )
            val checkpointInputPath = options["checkpoint-in"]?.trim().takeUnless { it.isNullOrEmpty() }
            val checkpoint = checkpointInputPath?.let { path ->
                val bytes = Files.readAllBytes(Path.of(path))
                PortableCheckpointCodec.decodeCompatibleOrNull(bytes.decodeToString())
                    ?: error("Invalid portable CP checkpoint: $path")
            }
            val executor = OspfCpSnapshotExecutor(storage)
            val result = runBlocking {
                executor.execute(
                    payload = payload,
                    tenantId = tenantId,
                    taskId = taskId,
                    sliceId = sliceId,
                    quantumMs = quantumMs,
                    checkpoint = checkpoint
                )
            }
            val resultBytes = result.resultRef?.let { ref -> runBlocking { storage.get(ref) } }
            if (resultBytes != null) {
                Files.createDirectories(resultPath.parent)
                Files.write(resultPath, resultBytes)
            }
            val checkpointRef = runBlocking {
                executor.exportCheckpoint(
                    payload = payload,
                    result = result,
                    tenantId = tenantId,
                    taskId = taskId,
                    sliceId = sliceId,
                    parentCheckpointId = checkpoint?.checkpointId
                )
            }
            val checkpointBytes = checkpointRef?.let { ref -> runBlocking { storage.get(ref) } }
            if (checkpointBytes != null) {
                Files.createDirectories(checkpointPath.parent)
                Files.write(checkpointPath, checkpointBytes)
            }
            printCpResult(result, resultPath.takeIf { resultBytes != null }, checkpointPath.takeIf { checkpointBytes != null })
        }.onFailure { error ->
            println("completed=false")
            println("feasible=false")
            println("problemStatus=UNKNOWN")
            println("solutionPresence=NONE")
            println("proofStatus=NONE")
            println("schemaVersion=2.0")
            println("runId=$taskId")
            println("attemptId=$sliceId")
            println("message=${error.message ?: error::class.simpleName}")
            println("terminationReason=${RemoteTerminationReason.BACKEND_FAILURE}")
        }
    }

    private fun printCpResult(result: SolveResult, resultPath: Path?, checkpointPath: Path?) {
        val completed = result.terminationReason !in setOf(
            RemoteTerminationReason.TIME_LIMIT,
            RemoteTerminationReason.NODE_LIMIT,
            RemoteTerminationReason.ITERATION_LIMIT,
            RemoteTerminationReason.BACKEND_FAILURE
        )
        println("completed=$completed")
        println("feasible=${result.feasible}")
        println("problemStatus=${result.problemStatus}")
        println("terminationReason=${result.terminationReason}")
        println("schemaVersion=${result.schemaVersion}")
        result.objectiveValueInt64?.let { println("objectiveInt64=$it") }
        result.objectiveValue?.let { println("objective=${it.toDouble()}") }
        result.gap?.let { println("gap=${it.toDouble()}") }
        println("elapsedMs=${result.elapsedMs}")
        result.message?.let { println("message=${it.replace('\n', ' ')}") }
        resultPath?.let { println("resultPath=${it.toAbsolutePath()}") }
        checkpointPath?.let { println("checkpointPath=${it.toAbsolutePath()}") }
        resultPath?.let { path ->
            if (Files.isRegularFile(path)) {
                println(Files.readString(path))
            }
        }
    }

    private fun runCompatibilityProgress(
        options: Map<String, String>,
        taskId: String,
        sliceId: String,
        model: String,
        quantumMs: Long,
        totalRuntimeMs: Long,
        checkpointIn: String?,
        stateDir: Path
    ) {

        // Restore progress from checkpoint / 从检查点恢复进度
        val restoredProgress = readCheckpointProgress(checkpointIn)
        val progressed = (restoredProgress + quantumMs.toDouble() / totalRuntimeMs.toDouble()).coerceIn(0.0, 1.0)
        val completed = progressed >= 1.0
        val feasible = true

        // Calculate objective value / 计算目标值
        val baseObjective = max(1.0, absHash(model).toDouble() % 1000.0)
        val objective = baseObjective * (1.0 - progressed)
        val gap = if (completed) 0.0 else min(1.0, 1.0 - progressed)

        // Write checkpoint / 写入检查点
        val checkpointPath = writeCheckpoint(
            stateDir = stateDir,
            taskId = taskId,
            sliceId = sliceId,
            progress = progressed
        )

        // Write result if completed / 完成时写入结果
        val resultPath = if (completed) {
            writeResult(
                stateDir = stateDir,
                taskId = taskId,
                sliceId = sliceId,
                objective = objective,
                gap = gap
            )
        } else {
            ""
        }
        val message = if (completed) "completed" else "suspended"

        // Output results / 输出结果
        println("completed=$completed")
        println("feasible=$feasible")
        println("objective=$objective")
        println("gap=$gap")
        println("elapsedMs=$quantumMs")
        println("message=$message")
        println("checkpointPath=$checkpointPath")
        println("resultPath=$resultPath")
    }

    /**
     * 从检查点文件读取进度
     * Read progress from checkpoint file
     */
    private fun readCheckpointProgress(checkpointIn: String?): Double {
        if (checkpointIn.isNullOrBlank()) {
            return 0.0
        }
        val path = runCatching { Path.of(checkpointIn).toAbsolutePath().normalize() }.getOrNull() ?: return 0.0
        if (!Files.exists(path) || !Files.isRegularFile(path)) {
            return 0.0
        }
        val content = runCatching { Files.readString(path) }.getOrDefault("").trim()
        return content.toDoubleOrNull()?.coerceIn(0.0, 1.0) ?: 0.0
    }

    /**
     * 写入检查点文件
     * Write checkpoint file
     */
    private fun writeCheckpoint(
        stateDir: Path,
        taskId: String,
        sliceId: String,
        progress: Double
    ): String {
        val taskDir = stateDir.resolve(sanitize(taskId))
        Files.createDirectories(taskDir)
        val checkpoint = taskDir.resolve("${sanitize(sliceId)}.chk")
        Files.writeString(checkpoint, progress.toString())
        return checkpoint.toString()
    }

    /**
     * 写入结果文件
     * Write result file
     */
    private fun writeResult(
        stateDir: Path,
        taskId: String,
        sliceId: String,
        objective: Double,
        gap: Double
    ): String {
        val taskDir = stateDir.resolve(sanitize(taskId))
        Files.createDirectories(taskDir)
        val result = taskDir.resolve("${sanitize(sliceId)}.result")
        val content = buildString {
            appendLine("objective=$objective")
            appendLine("gap=$gap")
        }
        Files.writeString(result, content)
        return result.toString()
    }

    /**
     * 清理文件名中的特殊字符
     * Sanitize special characters in filename
     */
    private fun sanitize(value: String): String =
        value.map { ch ->
            when {
                ch.isLetterOrDigit() || ch == '_' || ch == '-' || ch == '.' -> ch
                else -> '_'
            }
        }.joinToString("")

    /**
     * 计算绝对哈希值
     * Calculate absolute hash value
     */
    private fun absHash(value: String): Int = value.hashCode().let { if (it == Int.MIN_VALUE) 0 else kotlin.math.abs(it) }

    /**
     * 解析命令行参数
     * Parse command line arguments
     */
    private fun parseArgs(args: Array<String>): Map<String, String> {
        val result = linkedMapOf<String, String>()
        var i = 0
        while (i < args.size) {
            val token = args[i]
            if (token.startsWith("--")) {
                val key = token.removePrefix("--")
                if (i + 1 < args.size && !args[i + 1].startsWith("--")) {
                    result[key] = args[i + 1]
                    i += 1
                } else {
                    result[key] = "true"
                }
            }
            i += 1
        }
        return result
    }
}
