/**
 * RemoteSolverReplayMain - 远程求解器任务回放主入口
 *
 * RemoteSolverReplayMain - Remote solver task replay main entry point.
 *
 * 远程求解器任务时间线回放和调试工具的启动入口点。
 * 用于分析任务执行历史、诊断问题并生成时间线报告。
 *
 * Bootstrap entry point for remote solver task timeline replay and debugging tool.
 * Used to analyze task execution history, diagnose issues, and generate timeline reports.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import fuookami.ospf.framework.remote_solver.application.TaskTimelineReplayer
import kotlinx.coroutines.runBlocking
import java.nio.file.Files
import java.nio.file.Path

/**
 * 远程求解器任务回放主入口对象
 *
 * Remote solver task replay main entry object.
 *
 * 执行任务时间线回放，从数据库加载事件并生成可视化报告。
 * 支持文本和 JSON 格式输出。
 *
 * Executes task timeline replay, loads events from database,
 * and generates visualization reports.
 * Supports text and JSON format output.
 */
object RemoteSolverReplayMain {
    /**
     * 主入口方法
     *
     * Main entry method.
     *
     * 执行任务时间线回放的主方法。
     * 加载任务事件历史、生成时间线报告并输出。
     *
     * Main method for executing task timeline replay.
     * Loads task event history, generates timeline report, and outputs.
     *
     * @param args 命令行参数，必须包含 --task <taskId>
     *             CLI arguments, must include --task <taskId>
     * @throws IllegalArgumentException 缺少必要参数时抛出
     *                                  Thrown when required arguments are missing
     */
    @JvmStatic
    fun main(args: Array<String>) = runBlocking {
        val parsed = BootstrapCliSupport.parseArgs(args)
        val configPath = BootstrapCliSupport.resolveConfigPath(parsed["config"])
        val taskId = parsed["task"]?.trim()
        if (taskId.isNullOrEmpty()) {
            throw IllegalArgumentException("Missing required argument: --task <taskId>")
        }
        val format = parsed["format"]?.trim()?.lowercase() ?: "text"
        val limit = parsed["limit"]?.trim()?.toIntOrNull()?.coerceAtLeast(1) ?: 500
        val outputPath = parsed["output"]?.trim()?.takeIf { it.isNotEmpty() }?.let {
            Path.of(it).toAbsolutePath().normalize()
        }

        val properties = BootstrapCliSupport.loadProperties(configPath)
        val runtime = RemoteSolverBootstrapFactory.create(properties = properties)
        val replayer = TaskTimelineReplayer(
            taskStatePort = runtime.service.taskStatePort(),
            checkpointPort = runtime.service.checkpointPort(),
            costLedgerPort = runtime.service.costLedgerPort(),
            clock = runtime.service.clockPort()
        )
        val report = replayer.replay(taskId = taskId, limitEvents = limit)
        val rendered = when (format) {
            "json" -> renderJson(report)
            "text" -> replayer.renderText(report)
            else -> throw IllegalArgumentException("Unsupported format '$format'. Supported: text|json")
        }

        if (outputPath == null) {
            println(rendered)
        } else {
            Files.createDirectories(outputPath.parent ?: outputPath.toAbsolutePath().parent)
            Files.writeString(outputPath, rendered)
            println("task timeline exported")
            println("taskId=${report.taskId}")
            println("format=$format")
            println("output=$outputPath")
        }
    }

    /**
     * 渲染 JSON 格式报告
     *
     * Renders JSON format report.
     *
     * 将任务时间线报告转换为 JSON 字符串格式。
     *
     * Converts task timeline report to JSON string format.
     *
     * @param report 任务时间线报告
     *                Task timeline report
     * @return JSON 格式的报告字符串
     *         Report string in JSON format
     */
    private fun renderJson(report: fuookami.ospf.framework.remote_solver.application.TaskTimelineReport): String {
        val eventsJson = report.events.joinToString(",") { event ->
            """{"atEpochMs":${event.atEpochMs},"type":"${escapeJson(event.type)}","summary":"${escapeJson(event.summary)}","attributes":${mapToJson(event.attributes)}}"""
        }
        val gapsJson = report.gaps.joinToString(",") { gap ->
            """{"severity":"${escapeJson(gap.severity)}","message":"${escapeJson(gap.message)}","relatedIds":${mapToJson(gap.relatedIds)}}"""
        }
        return """{"taskId":"${escapeJson(report.taskId)}","generatedAtEpochMs":${report.generatedAtEpochMs},"status":"${escapeJson(report.status)}","events":[${eventsJson}],"gaps":[${gapsJson}]}"""
    }

    /**
     * 将映射转换为 JSON 对象字符串
     *
     * Converts map to JSON object string.
     *
     * 将字符串键值对映射转换为 JSON 对象格式。
     *
     * Converts string key-value map to JSON object format.
     *
     * @param input 输入映射
     *               Input map
     * @return JSON 对象字符串
     *         JSON object string
     */
    private fun mapToJson(input: Map<String, String>): String {
        val body = input.entries.joinToString(",") { entry ->
            """"${escapeJson(entry.key)}":"${escapeJson(entry.value)}""""
        }
        return "{$body}"
    }

    /**
     * 转义 JSON 特殊字符
     *
     * Escapes JSON special characters.
     *
     * 将字符串中的特殊字符转换为 JSON 安全的转义序列。
     *
     * Converts special characters in string to JSON-safe escape sequences.
     *
     * @param raw 原始字符串
     *            Raw string
     * @return 转义后的字符串
     *         Escaped string
     */
    private fun escapeJson(raw: String): String =
        raw
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
}