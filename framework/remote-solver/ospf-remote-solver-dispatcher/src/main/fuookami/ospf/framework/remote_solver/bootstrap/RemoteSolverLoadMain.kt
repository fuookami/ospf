@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * RemoteSolverLoadMain - 远程求解器负载测试主入口
 *
 * RemoteSolverLoadMain - Remote solver load testing main entry point.
 *
 * 远程求解器容量测试和负载测试的启动入口点。
 * 用于验证系统容量、吞吐量和 SLO 达标情况。
 *
 * Bootstrap entry point for remote solver capacity testing and load testing.
 * Used to validate system capacity, throughput, and SLO compliance.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverTypeName
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.nio.file.Files
import java.nio.file.Path
import java.util.UUID
import kotlin.time.DurationUnit
import kotlin.time.toDuration

/**
 * 远程求解器负载测试主入口对象
 *
 * Remote solver load testing main entry object.
 *
 * 执行容量测试，生成节点、提交任务并收集结果统计。
 * 支持 SLO 验证和多格式报告输出。
 *
 * Executes capacity testing, generates nodes, submits tasks,
 * and collects result statistics.
 * Supports SLO validation and multi-format report output.
 */
object RemoteSolverLoadMain {
    /**
     * 测试选项配置
     *
     * Test options configuration.
     *
     * 定义负载测试的所有配置参数。
     *
     * Defines all configuration parameters for load testing.
     *
     * @param configPath 配置文件路径
     *                    Config file path
     * @param totalTasks 总任务数量
     *                    Total task count
     * @param simpleRatio 简单任务比例
     *                     Simple task ratio
     * @param realtimeRatio 实时任务比例
     *                       Realtime task ratio
     * @param maxRounds 最大调度轮数
     *                   Maximum scheduling rounds
     * @param nodeCount 测试节点数量
     *                  Test node count
     * @param performanceTierSplitRatio 高性能节点比例
     *                                   High-performance node ratio
     * @param outputFormat 输出格式（TEXT/JSON/BOTH）
     *                      Output format (TEXT/JSON/BOTH)
     * @param outputPath 输出文件路径，可为 null
     *                    Output file path, can be null
     * @param sloSuccessRate SLO 成功率目标
     *                        SLO success rate target
     * @param sloThroughput SLO 吞吐量目标，可为 null
     *                       SLO throughput target, can be null
     */
    private data class Options(
        val configPath: Path,
        val totalTasks: Int,
        val simpleRatio: Double,
        val realtimeRatio: Double,
        val maxRounds: Int,
        val nodeCount: Int,
        val performanceTierSplitRatio: Double,
        val outputFormat: OutputFormat,
        val outputPath: Path?,
        val sloSuccessRate: Double,
        val sloThroughput: Double?
    )

    /**
     * 输出格式枚举
     *
     * Output format enumeration.
     *
     * 定义测试报告的输出格式类型。
     *
     * Defines output format types for test reports.
     */
    private enum class OutputFormat {
        TEXT, JSON, BOTH
    }

    /** JSON 序列化器配置 */
    private val json = Json { prettyPrint = true }

    /**
     * 主入口方法
     *
     * Main entry method.
     *
     * 执行容量测试的主方法。注册节点、提交任务、运行调度循环并生成报告。
     *
     * Main method for executing capacity testing.
     * Registers nodes, submits tasks, runs scheduling loop, and generates report.
     *
     * @param args 命令行参数
     *             CLI arguments
     */
    @JvmStatic
    fun main(args: Array<String>) = runBlocking {
        val options = parseOptions(args)
        val properties = BootstrapCliSupport.loadProperties(options.configPath)
        val runtime = RemoteSolverBootstrapFactory.create(properties = properties)
        val service = runtime.service

        val testId = "load-${UUID.randomUUID().toString().take(8)}"
        val startEpochMs = System.currentTimeMillis()

        // 注册测试节点
        repeat(options.nodeCount) { index ->
            val highPerformanceTier = (index + 1).toDouble() / options.nodeCount.toDouble() <= options.performanceTierSplitRatio
            val nodeId = "load-node-${index + 1}"
            val profile = if (highPerformanceTier) {
                NodeCapabilityProfile(
                    nodeId = NodeId.of(nodeId),
                    solverType = SolverTypeName.of("gurobi"),
                    performanceScore = Flt64(2.0),
                    pricePerSecond = Flt64(0.32),
                    minBillingUnit = 1L.toDuration(DurationUnit.SECONDS),
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 2
                )
            } else {
                NodeCapabilityProfile(
                    nodeId = NodeId.of(nodeId),
                    solverType = SolverTypeName.of("gurobi"),
                    performanceScore = Flt64(1.0),
                    pricePerSecond = Flt64(0.12),
                    minBillingUnit = 1L.toDuration(DurationUnit.SECONDS),
                    supportsInterrupt = false,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            }
            service.registerNode(profile)
        }

        // 提交测试任务
        val taskIds = mutableListOf<String>()
        repeat(options.totalTasks) { index ->
            val ratioCursor = index.toDouble() / options.totalTasks.toDouble()
            val complexity = if (ratioCursor < options.simpleRatio) TaskComplexity.SIMPLE else TaskComplexity.COMPLEX
            val timeSensitivity = if (ratioCursor < options.realtimeRatio) TimeSensitivity.REALTIME else TimeSensitivity.NON_REALTIME
            val task = service.submitTask(
                payload = SolvePayload(
                    modelRef = ObjectRef.of(path = "models/load-${index + 1}"),
                    extension = mapOf("solverType" to "gurobi")
                ),
                complexity = complexity,
                timeSensitivity = timeSensitivity,
                priority = if (timeSensitivity == TimeSensitivity.REALTIME) 10 else 0,
                budgetScope = BudgetScopeId.of("load-test")
            )
            taskIds.add(task.taskId.value)
        }

        // 运行调度循环直到任务完成或达到最大轮数
        var rounds = 0
        while (rounds < options.maxRounds) {
            var terminalCount = 0
            for (taskId in taskIds) {
                val task = service.getTask(taskId)
                if (task != null && task.status in TERMINAL_STATUSES) {
                    terminalCount += 1
                }
            }
            if (terminalCount == taskIds.size) {
                break
            }
            service.scheduleOnce()
            rounds += 1
        }

        // 收集结果统计
        val endEpochMs = System.currentTimeMillis()
        val tasks = taskIds.mapNotNull { service.getTask(it) }
        val completed = tasks.count { it.status == TaskStatus.COMPLETED }
        val failed = tasks.count { it.status == TaskStatus.FAILED }
        val stopped = tasks.count { it.status == TaskStatus.STOPPED }
        val nonTerminal = tasks.size - completed - failed - stopped
        val totalCost = tasks.sumOf { it.consumedCost.toDouble() }
        val avgCost = if (tasks.isEmpty()) 0.0 else totalCost / tasks.size.toDouble()
        val durationMs = endEpochMs - startEpochMs
        val throughput = if (durationMs > 0) tasks.size.toDouble() * 1000.0 / durationMs.toDouble() else 0.0
        val successRate = if (tasks.isNotEmpty()) completed.toDouble() / tasks.size.toDouble() else 0.0

        // 构建容量测试报告
        val report = CapacityReport(
            testId = testId,
            generatedAtEpochMs = endEpochMs,
            config = CapacityTestConfig(
                totalTasks = options.totalTasks,
                nodeCount = options.nodeCount,
                simpleRatio = options.simpleRatio,
                realtimeRatio = options.realtimeRatio,
                highTierRatio = options.performanceTierSplitRatio,
                maxRounds = options.maxRounds
            ),
            results = CapacityResults(
                totalTasks = tasks.size,
                completedTasks = completed,
                failedTasks = failed,
                stoppedTasks = stopped,
                nonTerminalTasks = nonTerminal,
                totalRounds = rounds,
                startEpochMs = startEpochMs,
                endEpochMs = endEpochMs,
                durationMs = durationMs,
                throughputTasksPerSecond = throughput,
                totalCost = totalCost,
                avgCostPerTask = avgCost,
                successRate = successRate
            ),
            sloValidation = SloValidationResult(
                successRateTarget = options.sloSuccessRate,
                successRateActual = successRate,
                successRatePassed = successRate >= options.sloSuccessRate,
                throughputTarget = options.sloThroughput,
                throughputActual = throughput,
                throughputPassed = options.sloThroughput?.let { throughput >= it },
                avgCostTarget = null,
                avgCostActual = avgCost,
                avgCostPassed = null,
                overallPassed = successRate >= options.sloSuccessRate &&
                    (options.sloThroughput == null || throughput >= options.sloThroughput)
            )
        )

        // 输出结果
        when (options.outputFormat) {
            OutputFormat.TEXT, OutputFormat.BOTH -> printTextReport(report)
            OutputFormat.JSON -> {}
        }

        if (options.outputFormat == OutputFormat.JSON || options.outputFormat == OutputFormat.BOTH) {
            val jsonOutput = json.encodeToString(report)
            options.outputPath?.let { path ->
                Files.writeString(path, jsonOutput)
                println("Report written to: $path")
            } ?: run {
                if (options.outputFormat == OutputFormat.JSON) {
                    println(jsonOutput)
                }
            }
        }
    }

    /**
     * 打印文本格式报告
     *
     * Prints text format report.
     *
     * 将容量测试报告以文本格式输出到控制台。
     *
     * Outputs capacity test report in text format to console.
     *
     * @param report 容量测试报告
     *                Capacity test report
     */
    private fun printTextReport(report: CapacityReport) {
        println()
        println("=== Capacity Test Report ===")
        println("Test ID: ${report.testId}")
        println("Generated at: ${java.time.Instant.ofEpochMilli(report.generatedAtEpochMs)}")
        println()
        println("Configuration:")
        println("  Total tasks: ${report.config.totalTasks}")
        println("  Node count: ${report.config.nodeCount}")
        println("  Simple/Complex ratio: ${report.config.simpleRatio}/${1.0 - report.config.simpleRatio}")
        println("  High/Low tier ratio: ${report.config.highTierRatio}/${1.0 - report.config.highTierRatio}")
        println()
        println("Results:")
        println("  Completed: ${report.results.completedTasks}")
        println("  Failed: ${report.results.failedTasks}")
        println("  Stopped: ${report.results.stoppedTasks}")
        println("  Non-terminal: ${report.results.nonTerminalTasks}")
        println("  Total rounds: ${report.results.totalRounds}")
        println("  Duration: ${report.results.durationMs}ms")
        println("  Throughput: ${"%.2f".format(report.results.throughputTasksPerSecond)} tasks/sec")
        println("  Total cost: ${"%.4f".format(report.results.totalCost)}")
        println("  Avg cost/task: ${"%.4f".format(report.results.avgCostPerTask)}")
        println("  Success rate: ${"%.2f".format(report.results.successRate * 100)}%")
        println()
        println("SLO Validation:")
        println("  Success rate: ${if (report.sloValidation.successRatePassed) "PASS" else "FAIL"} " +
            "(actual: ${"%.2f".format(report.sloValidation.successRateActual * 100)}%, " +
            "target: ${"%.2f".format(report.sloValidation.successRateTarget * 100)}%)")
        report.sloValidation.throughputTarget?.let { target ->
            println("  Throughput: ${if (report.sloValidation.throughputPassed == true) "PASS" else "FAIL"} " +
                "(actual: ${"%.2f".format(report.sloValidation.throughputActual)} tasks/sec, " +
                "target: ${"%.2f".format(target)} tasks/sec)")
        }
        println("  Overall: ${if (report.sloValidation.overallPassed) "PASS" else "FAIL"}")
        println()
    }

    /**
     * 解析命令行选项
     *
     * Parses CLI options.
     *
     * 从命令行参数解析测试配置选项。
     *
     * Parses test configuration options from CLI arguments.
     *
     * @param args 命令行参数
     *             CLI arguments
     * @return 测试选项配置
     *         Test options configuration
     */
    private fun parseOptions(args: Array<String>): Options {
        val arguments = BootstrapCliSupport.parseArgs(args)
        val configPath = BootstrapCliSupport.resolveConfigPath(arguments["config"])
        val totalTasks = arguments["total-tasks"]?.toIntOrNull()?.coerceAtLeast(1) ?: 100
        val simpleRatio = arguments["simple-ratio"]?.toDoubleOrNull()?.coerceIn(0.0, 1.0) ?: 0.7
        val realtimeRatio = arguments["realtime-ratio"]?.toDoubleOrNull()?.coerceIn(0.0, 1.0) ?: 0.2
        val maxRounds = arguments["max-rounds"]?.toIntOrNull()?.coerceAtLeast(1) ?: 2000
        val nodeCount = arguments["nodes"]?.toIntOrNull()?.coerceAtLeast(1) ?: 6
        val performanceTierSplitRatio = arguments["high-tier-ratio"]?.toDoubleOrNull()?.coerceIn(0.0, 1.0) ?: 0.4
        val outputFormat = when (arguments["output"]?.lowercase()) {
            "json" -> OutputFormat.JSON
            "both" -> OutputFormat.BOTH
            else -> OutputFormat.TEXT
        }
        val outputPath = arguments["output-path"]?.let { Path.of(it) }
        val sloSuccessRate = arguments["slo-success-rate"]?.toDoubleOrNull()?.coerceIn(0.0, 1.0)
            ?: SloValidationResult.DEFAULT_SUCCESS_RATE_TARGET
        val sloThroughput = arguments["slo-throughput"]?.toDoubleOrNull()?.coerceAtLeast(0.0)
        return Options(
            configPath = configPath,
            totalTasks = totalTasks,
            simpleRatio = simpleRatio,
            realtimeRatio = realtimeRatio,
            maxRounds = maxRounds,
            nodeCount = nodeCount,
            performanceTierSplitRatio = performanceTierSplitRatio,
            outputFormat = outputFormat,
            outputPath = outputPath,
            sloSuccessRate = sloSuccessRate,
            sloThroughput = sloThroughput
        )
    }

    /** 终态任务状态集合 */
    private val TERMINAL_STATUSES = setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)
}
