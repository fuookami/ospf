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

/**
 * 远程求解器 Worker 主程序
 * Remote solver worker main
 *
 * 模拟求解器执行，支持检查点恢复和进度跟踪。
 * Simulates solver execution, supports checkpoint recovery and progress tracking.
 *
 * 命令行参数 / Command line arguments:
 * --task         任务 ID / Task ID
 * --slice        切片 ID / Slice ID
 * --model        模型标识 / Model identifier
 * --quantum-ms   时间切片时长（毫秒） / Time slice duration in milliseconds
 * --total-runtime-ms 总运行时间（毫秒） / Total runtime in milliseconds
 * --checkpoint-in    输入检查点路径 / Input checkpoint path
 * --state-dir    状态目录 / State directory
 */
object RemoteSolverWorkerMain {
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