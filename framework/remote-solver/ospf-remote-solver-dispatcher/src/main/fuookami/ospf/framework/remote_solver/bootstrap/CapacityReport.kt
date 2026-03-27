/**
 * CapacityReport - 容量测试报告
 *
 * CapacityReport - Capacity test report models.
 *
 * 定义负载测试和容量规划的数据模型。
 * 包含测试配置、执行结果和 SLO（服务水平目标）验证结果。
 *
 * Defines data models for load testing and capacity planning.
 * Includes test configuration, execution results, and SLO (Service Level Objective) validation results.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import kotlinx.serialization.Serializable

/**
 * 容量测试报告
 *
 * Capacity test report.
 *
 * 容量测试的完整报告，包含测试标识、配置、结果和 SLO 验证。
 * 用于容量规划和 SLO 验证的关键指标捕获。
 *
 * Complete report for capacity testing, containing test identity,
 * configuration, results, and SLO validation.
 * Captures key metrics for capacity planning and SLO validation.
 *
 * @param testId 测试唯一标识符
 *                Unique test identifier
 * @param generatedAtEpochMs 报告生成时间（Unix 毫秒时间戳）
 *                           Report generation time (Unix epoch milliseconds)
 * @param config 测试配置参数
 *               Test configuration parameters
 * @param results 测试执行结果
 *                Test execution results
 * @param sloValidation SLO 验证结果
 *                       SLO validation results
 */
@Serializable
data class CapacityReport(
    val testId: String,
    val generatedAtEpochMs: Long,
    val config: CapacityTestConfig,
    val results: CapacityResults,
    val sloValidation: SloValidationResult
)

/**
 * 容量测试配置
 *
 * Capacity test configuration.
 *
 * 定义容量测试的参数配置，包括任务数量、节点数量和任务类型比例等。
 *
 * Defines parameter configuration for capacity testing,
 * including task count, node count, and task type ratios.
 *
 * @param totalTasks 总任务数量
 *                    Total task count
 * @param nodeCount 测试节点数量
 *                  Test node count
 * @param simpleRatio 简单任务比例（0.0-1.0）
 *                    Simple task ratio (0.0-1.0)
 * @param realtimeRatio 实时任务比例（0.0-1.0）
 *                       Realtime task ratio (0.0-1.0)
 * @param highTierRatio 高性能节点比例（0.0-1.0）
 *                       High-performance node ratio (0.0-1.0)
 * @param maxRounds 最大调度轮数
 *                   Maximum scheduling rounds
 */
@Serializable
data class CapacityTestConfig(
    val totalTasks: Int,
    val nodeCount: Int,
    val simpleRatio: Double,
    val realtimeRatio: Double,
    val highTierRatio: Double,
    val maxRounds: Int
)

/**
 * 容量测试结果
 *
 * Capacity test results.
 *
 * 记录容量测试的执行结果，包括任务统计、时间指标和成本指标。
 *
 * Records execution results of capacity testing,
 * including task statistics, time metrics, and cost metrics.
 *
 * @param totalTasks 处理的总任务数
 *                    Total tasks processed
 * @param completedTasks 完成的任务数
 *                        Completed task count
 * @param failedTasks 失败的任务数
 *                    Failed task count
 * @param stoppedTasks 停止的任务数
 *                     Stopped task count
 * @param nonTerminalTasks 非终态任务数（运行中或待处理）
 *                         Non-terminal task count (running or pending)
 * @param totalRounds 总调度轮数
 *                    Total scheduling rounds
 * @param startEpochMs 测试开始时间（Unix 毫秒时间戳）
 *                     Test start time (Unix epoch milliseconds)
 * @param endEpochMs 测试结束时间（Unix 毫秒时间戳）
 *                   Test end time (Unix epoch milliseconds)
 * @param durationMs 测试持续时间（毫秒）
 *                   Test duration (milliseconds)
 * @param throughputTasksPerSecond 吞吐量（任务/秒）
 *                                  Throughput (tasks per second)
 * @param totalCost 总成本
 *                   Total cost
 * @param avgCostPerTask 平均每任务成本
 *                        Average cost per task
 * @param successRate 成功率（0.0-1.0）
 *                     Success rate (0.0-1.0)
 */
@Serializable
data class CapacityResults(
    val totalTasks: Int,
    val completedTasks: Int,
    val failedTasks: Int,
    val stoppedTasks: Int,
    val nonTerminalTasks: Int,
    val totalRounds: Int,
    val startEpochMs: Long,
    val endEpochMs: Long,
    val durationMs: Long,
    val throughputTasksPerSecond: Double,
    val totalCost: Double,
    val avgCostPerTask: Double,
    val successRate: Double
)

/**
 * SLO 验证结果
 *
 * SLO validation result.
 *
 * 记录服务水平目标的验证结果，对比目标值与实际值。
 *
 * Records Service Level Objective validation results,
 * comparing target values with actual values.
 *
 * @param successRateTarget 成功率目标值
 *                           Success rate target value
 * @param successRateActual 成功率实际值
 *                           Success rate actual value
 * @param successRatePassed 成功率是否达标
 *                           Whether success rate passed target
 * @param throughputTarget 吞吐量目标值，可为 null（不验证）
 *                         Throughput target value, can be null (no validation)
 * @param throughputActual 吞吐量实际值
 *                         Throughput actual value
 * @param throughputPassed 吞吐量是否达标，可为 null（不验证）
 *                         Whether throughput passed target, can be null
 * @param avgCostTarget 平均成本目标值，可为 null（不验证）
 *                      Average cost target value, can be null
 * @param avgCostActual 平均成本实际值
 *                      Average cost actual value
 * @param avgCostPassed 平均成本是否达标，可为 null（不验证）
 *                      Whether average cost passed target, can be null
 * @param overallPassed 总体是否达标
 *                       Overall passed status
 */
@Serializable
data class SloValidationResult(
    val successRateTarget: Double,
    val successRateActual: Double,
    val successRatePassed: Boolean,
    val throughputTarget: Double?,
    val throughputActual: Double,
    val throughputPassed: Boolean?,
    val avgCostTarget: Double?,
    val avgCostActual: Double,
    val avgCostPassed: Boolean?,
    val overallPassed: Boolean
) {
    companion object {
        /**
         * 默认 SLO 目标值（基于 design.md §33）
         *
         * Default SLO targets (based on design.md §33).
         *
         * 默认的成功率目标为 99.9%，吞吐量目标为 500 任务/秒。
         * 无默认成本目标。
         *
         * Default success rate target is 99.9%, throughput target is 500 tasks/sec.
         * No default cost target.
         */
        val DEFAULT_SUCCESS_RATE_TARGET = 0.999 // 99.9%
        val DEFAULT_THROUGHPUT_TARGET = 500.0   // tasks/second
        val DEFAULT_AVG_COST_TARGET = null      // No default cost target
    }
}