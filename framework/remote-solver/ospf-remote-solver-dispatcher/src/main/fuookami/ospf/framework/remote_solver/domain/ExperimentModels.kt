/*
 * Experiment Models
 * 实验模型
 *
 * This file defines the A/B experiment framework for comparing different scheduling strategies.
 * 本文件定义了用于比较不同调度策略的A/B实验框架。
 *
 * The framework supports:
 * 该框架支持：
 * - Multiple experiment variants with traffic allocation
 * - 多实验变体与流量分配
 * - Statistical significance testing
 * - 统计显著性检验
 * - Experiment lifecycle management
 * - 实验生命周期管理
 */
package fuookami.ospf.framework.remote_solver.domain

import kotlinx.serialization.Serializable

/**
 * Experiment
 * 实验
 *
 * A/B experiment for comparing different scheduling strategies.
 * 用于比较不同调度策略的A/B实验。
 *
 * This model supports:
 * 该模型支持：
 * - Offline training from historical data
 * - 从历史数据离线训练
 * - Online calibration based on real-time feedback
 * - 基于实时反馈的在线校准
 * - A/B testing of different scoring strategies
 * - 不同评分策略的A/B测试
 *
 * @param experimentId Unique identifier for the experiment.
 *                      实验的唯一标识符。
 * @param name Human-readable experiment name.
 *             人类可读的实验名称。
 * @param description Detailed description of the experiment.
 *                     实验的详细描述。
 * @param status Current experiment status.
 *               当前实验状态。
 * @param variants List of experiment variants.
 *                 实验变体列表。
 * @param config Experiment configuration.
 *               实验配置。
 * @param metrics Overall experiment metrics.
 *                整体实验指标。
 * @param createdAtEpochMs Creation timestamp in epoch milliseconds.
 *                          创建时间戳（epoch毫秒）。
 * @param startedAtEpochMs Start timestamp in epoch milliseconds (optional).
 *                         开始时间戳（epoch毫秒，可选）。
 * @param endedAtEpochMs End timestamp in epoch milliseconds (optional).
 *                       结束时间戳（epoch毫秒，可选）。
 */
@Serializable
data class Experiment(
    val experimentId: String,
    val name: String,
    val description: String,
    val status: ExperimentStatus,
    val variants: List<ExperimentVariant>,
    val config: ExperimentConfig,
    val metrics: ExperimentMetrics,
    val createdAtEpochMs: Long,
    val startedAtEpochMs: Long? = null,
    val endedAtEpochMs: Long? = null
) {
    /**
     * Gets the variant assignment for a task based on task ID.
     * 根据任务ID获取任务的变体分配。
     *
     * Uses consistent hashing based on task ID for stable assignment.
     * 使用基于任务ID的一致性哈希实现稳定分配。
     *
     * @param taskId The task identifier.
     *               任务标识符。
     * @return The assigned variant, or null if experiment is not running.
     *         分配的变体，如果实验未运行则返回null。
     */
    fun getVariant(taskId: String): ExperimentVariant? {
        if (status != ExperimentStatus.RUNNING) return null

        // Consistent hashing based on task ID for stable assignment
        // 基于任务ID的一致性哈希实现稳定分配
        val hash = Math.abs(taskId.hashCode())
        val bucket = hash % 100

        var cumulative = 0
        for (variant in variants.sortedBy { it.id }) {
            cumulative += variant.trafficPercentage
            if (bucket < cumulative) {
                return variant
            }
        }
        return variants.firstOrNull()
    }

    /**
     * Checks if the experiment has reached statistical significance.
     * 检查实验是否已达到统计显著性。
     *
     * @return True if the experiment has significant results.
     *         如果实验有显著结果则返回true。
     */
    fun hasSignificantResult(): Boolean {
        if (metrics.sampleCount < config.minSampleSize) return false

        val controlVariant = variants.find { it.isControl } ?: return false
        val treatmentVariants = variants.filter { !it.isControl }

        return treatmentVariants.any { treatment ->
            val pValue = calculatePValue(controlVariant, treatment)
            pValue != null && pValue < config.significanceLevel
        }
    }

    /**
     * Gets the winning variant if any.
     * 获取获胜的变体（如果有）。
     *
     * @return The winning variant, or null if no significant winner.
     *         获胜的变体，如果没有显著获胜者则返回null。
     */
    fun getWinner(): ExperimentVariant? {
        if (!hasSignificantResult()) return null

        val controlVariant = variants.find { it.isControl } ?: return null
        val treatmentVariants = variants.filter { !it.isControl }

        val primaryMetric = config.primaryMetric
        val higherIsBetter = primaryMetric.higherIsBetter

        return treatmentVariants
            .filter { treatment ->
                val pValue = calculatePValue(controlVariant, treatment)
                pValue != null && pValue < config.significanceLevel
            }
            .maxWithOrNull(
                if (higherIsBetter) {
                    compareBy { it.metrics.getMetricValue(primaryMetric) }
                } else {
                    compareBy { -it.metrics.getMetricValue(primaryMetric) }
                }
            )
    }

    /**
     * Calculates p-value between control and treatment variants.
     * 计算对照组和实验组变体之间的p值。
     *
     * Uses simplified two-sample t-test approximation.
     * 使用简化的双样本t检验近似。
     *
     * @param control The control variant.
     *                对照组变体。
     * @param treatment The treatment variant.
     *                   实验组变体。
     * @return The p-value, or null if insufficient data.
     *         p值，如果数据不足则返回null。
     */
    private fun calculatePValue(control: ExperimentVariant, treatment: ExperimentVariant): Double? {
        // Simplified two-sample t-test approximation
        // 简化的双样本t检验近似
        val n1 = control.metrics.sampleCount.toDouble()
        val n2 = treatment.metrics.sampleCount.toDouble()

        if (n1 < 2 || n2 < 2) return null

        val mean1 = control.metrics.avgSuccessRate
        val mean2 = treatment.metrics.avgSuccessRate
        val var1 = control.metrics.successRateVariance
        val var2 = treatment.metrics.successRateVariance

        val pooledSe = kotlin.math.sqrt(var1 / n1 + var2 / n2)
        if (pooledSe == 0.0) return null

        val tStat = (mean2 - mean1) / pooledSe
        val df = n1 + n2 - 2

        // Approximate p-value using normal distribution for large samples
        // 对大样本使用正态分布近似p值
        return if (df > 30) {
            2.0 * (1.0 - normalCdf(kotlin.math.abs(tStat)))
        } else {
            null // Need proper t-distribution for small samples
            // 对小样本需要正确的t分布
        }
    }

    /**
     * Approximates standard normal CDF.
     * 标准正态CDF近似。
     *
     * @param x The input value.
     *          输入值。
     * @return The CDF value.
     *         CDF值。
     */
    private fun normalCdf(x: Double): Double {
        // Approximation of standard normal CDF
        // 标准正态CDF近似
        val a1 = 0.254829592
        val a2 = -0.284496736
        val a3 = 1.421413741
        val a4 = -1.453152027
        val a5 = 1.061405429
        val p = 0.3275911

        val sign = if (x < 0) -1 else 1
        val absX = kotlin.math.abs(x) / kotlin.math.sqrt(2.0)
        val t = 1.0 / (1.0 + p * absX)
        val y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * kotlin.math.exp(-absX * absX)

        return 0.5 * (1.0 + sign * y)
    }
}

/**
 * Experiment Status
 * 实验状态
 *
 * Status values for experiment lifecycle.
 * 实验生命周期的状态值。
 */
@Serializable
enum class ExperimentStatus {
    /**
     * Experiment created but not started.
     * 实验已创建但未开始。
     */
    CREATED,

    /**
     * Experiment is currently running.
     * 实验正在运行。
     */
    RUNNING,

    /**
     * Experiment is temporarily paused.
     * 实验临时暂停。
     */
    PAUSED,

    /**
     * Experiment completed successfully.
     * 实验成功完成。
     */
    COMPLETED,

    /**
     * Experiment was cancelled.
     * 实验已取消。
     */
    CANCELLED
}

/**
 * Experiment Variant
 * 实验变体
 *
 * A variant in an A/B experiment.
 * A/B实验中的一个变体。
 *
 * @param id Unique identifier for the variant.
 *            变体的唯一标识符。
 * @param name Human-readable variant name.
 *             人类可读的变体名称。
 * @param isControl Whether this is the control (baseline) variant.
 *                   是否为对照组（基准）变体。
 * @param trafficPercentage Percentage of traffic allocated to this variant (0-100).
 *                            分配到此变体的流量百分比（0-100）。
 * @param scoringModelId Scoring model ID to use for this variant (optional).
 *                        此变体使用的评分模型ID（可选）。
 * @param schedulerWeightsOverride Custom weights override for scheduling (optional).
 *                                  调度的自定义权重覆盖（可选）。
 * @param metrics Performance metrics for this variant.
 *                此变体的性能指标。
 */
@Serializable
data class ExperimentVariant(
    val id: String,
    val name: String,
    val isControl: Boolean,
    val trafficPercentage: Int,
    val scoringModelId: String? = null,
    val schedulerWeightsOverride: ScoringWeights? = null,
    val metrics: VariantMetrics = VariantMetrics()
)

/**
 * Experiment Config
 * 实验配置
 *
 * Configuration parameters for an experiment.
 * 实验的配置参数。
 *
 * @param primaryMetric The primary metric for determining success.
 *                       用于确定成功的主要指标。
 * @param minSampleSize Minimum sample size for statistical significance.
 *                       统计显著性的最小样本量。
 * @param significanceLevel Significance level for hypothesis testing (default 0.05).
 *                           假设检验的显著性水平（默认0.05）。
 * @param maxDurationDays Maximum duration of the experiment in days.
 *                         实验的最大持续时间（天）。
 * @param autoComplete Whether to automatically complete when significant result found.
 *                      是否在发现显著结果时自动完成。
 */
@Serializable
data class ExperimentConfig(
    val primaryMetric: PrimaryMetric,
    val minSampleSize: Int = 1000,
    val significanceLevel: Double = 0.05,
    val maxDurationDays: Int = 14,
    val autoComplete: Boolean = true
)

/**
 * Primary Metric
 * 主要指标
 *
 * The primary metric used to evaluate experiment success.
 * 用于评估实验成功的主要指标。
 *
 * @param displayName Human-readable metric name.
 *                     人类可读的指标名称。
 * @param higherIsBetter Whether higher values indicate better performance.
 *                        较高值是否表示更好的性能。
 */
@Serializable
enum class PrimaryMetric(val displayName: String, val higherIsBetter: Boolean) {
    /**
     * Success rate of tasks.
     * 任务成功率。
     */
    SUCCESS_RATE("Success Rate", true),

    /**
     * Average runtime of tasks.
     * 任务平均运行时间。
     */
    AVG_RUNTIME("Average Runtime", false),

    /**
     * Average cost of tasks.
     * 任务平均成本。
     */
    AVG_COST("Average Cost", false),

    /**
     * Throughput (tasks per minute).
     * 吞吐量（每分钟任务数）。
     */
    THROUGHPUT("Throughput", true)
}

/**
 * Variant Metrics
 * 变体指标
 *
 * Performance metrics for an experiment variant.
 * 实验变体的性能指标。
 *
 * @param sampleCount Number of samples recorded.
 *                     已记录的样本数。
 * @param successCount Number of successful outcomes.
 *                     成功结果的数量。
 * @param totalRuntimeMs Total runtime in milliseconds.
 *                       总运行时间（毫秒）。
 * @param totalCost Total cost incurred.
 *                   已产生的总成本。
 * @param sumSquaredSuccessRate Sum of squared success rates for variance calculation.
 *                               成功率平方和，用于方差计算。
 */
@Serializable
data class VariantMetrics(
    val sampleCount: Int = 0,
    val successCount: Int = 0,
    val totalRuntimeMs: Long = 0,
    val totalCost: Double = 0.0,
    val sumSquaredSuccessRate: Double = 0.0
) {
    /**
     * Average success rate.
     * 平均成功率。
     */
    val avgSuccessRate: Double
        get() = if (sampleCount > 0) successCount.toDouble() / sampleCount.toDouble() else 0.0

    /**
     * Average runtime in milliseconds.
     * 平均运行时间（毫秒）。
     */
    val avgRuntimeMs: Double
        get() = if (sampleCount > 0) totalRuntimeMs.toDouble() / sampleCount.toDouble() else 0.0

    /**
     * Average cost per task.
     * 每任务平均成本。
     */
    val avgCost: Double
        get() = if (sampleCount > 0) totalCost / sampleCount.toDouble() else 0.0

    /**
     * Success rate variance (Bernoulli variance).
     * 成功率方差（伯努利方差）。
     */
    val successRateVariance: Double
        get() {
            if (sampleCount < 2) return 0.0
            val p = avgSuccessRate
            return p * (1 - p) // Bernoulli variance
            // 伯努利方差
        }

    /**
     * Gets the value for a specific metric.
     * 获取特定指标的值。
     *
     * @param metric The metric to retrieve.
     *               要获取的指标。
     * @return The metric value.
     *         指标值。
     */
    fun getMetricValue(metric: PrimaryMetric): Double {
        return when (metric) {
            PrimaryMetric.SUCCESS_RATE -> avgSuccessRate
            PrimaryMetric.AVG_RUNTIME -> avgRuntimeMs
            PrimaryMetric.AVG_COST -> avgCost
            PrimaryMetric.THROUGHPUT -> if (avgRuntimeMs > 0) 60000.0 / avgRuntimeMs else 0.0
        }
    }

    /**
     * Records a new outcome and returns updated metrics.
     * 记录新结果并返回更新后的指标。
     *
     * @param success Whether the outcome was successful.
     *                结果是否成功。
     * @param runtimeMs Runtime in milliseconds.
     *                  运行时间（毫秒）。
     * @param cost Cost incurred.
     *             已产生的成本。
     * @return Updated VariantMetrics instance.
     *         更新后的VariantMetrics实例。
     */
    fun record(success: Boolean, runtimeMs: Long, cost: Double): VariantMetrics {
        return VariantMetrics(
            sampleCount = sampleCount + 1,
            successCount = successCount + if (success) 1 else 0,
            totalRuntimeMs = totalRuntimeMs + runtimeMs,
            totalCost = totalCost + cost,
            sumSquaredSuccessRate = sumSquaredSuccessRate + if (success) 1.0 else 0.0
        )
    }
}

/**
 * Experiment Metrics
 * 实验指标
 *
 * Overall metrics for an experiment.
 * 实验的整体指标。
 *
 * @param sampleCount Total number of samples across all variants.
 *                     所有变体的总样本数。
 * @param startTimeEpochMs Experiment start time in epoch milliseconds (optional).
 *                         实验开始时间（epoch毫秒，可选）。
 * @param lastUpdateEpochMs Last update time in epoch milliseconds (optional).
 *                           最后更新时间（epoch毫秒，可选）。
 */
@Serializable
data class ExperimentMetrics(
    val sampleCount: Int = 0,
    val startTimeEpochMs: Long? = null,
    val lastUpdateEpochMs: Long? = null
) {
    /**
     * Records a new sample and returns updated metrics.
     * 记录新样本并返回更新后的指标。
     *
     * @return Updated ExperimentMetrics instance.
     *         更新后的ExperimentMetrics实例。
     */
    fun record(): ExperimentMetrics {
        return ExperimentMetrics(
            sampleCount = sampleCount + 1,
            startTimeEpochMs = startTimeEpochMs ?: System.currentTimeMillis(),
            lastUpdateEpochMs = System.currentTimeMillis()
        )
    }
}

/**
 * Experiment Registry
 * 实验注册表
 *
 * Registry for managing active experiments.
 * 管理活跃实验的注册表。
 *
 * @param experiments Map of experiment ID to Experiment.
 *                    实验ID到Experiment的映射。
 */
@Serializable
data class ExperimentRegistry(
    val experiments: Map<String, Experiment> = emptyMap()
) {
    /**
     * Gets all currently running experiments.
     * 获取所有正在运行的实验。
     *
     * @return List of active experiments.
     *         活跃实验列表。
     */
    fun getActiveExperiments(): List<Experiment> {
        return experiments.values.filter { it.status == ExperimentStatus.RUNNING }
    }

    /**
     * Gets the experiment and variant assignment for a task.
     * 获取任务的实验和变体分配。
     *
     * @param taskId The task identifier.
     *               任务标识符。
     * @return The experiment and assigned variant, or null if no active experiment.
     *         实验和分配的变体，如果没有活跃实验则返回null。
     */
    fun getVariantForTask(taskId: String): Pair<Experiment, ExperimentVariant>? {
        for (experiment in getActiveExperiments()) {
            val variant = experiment.getVariant(taskId)
            if (variant != null) {
                return Pair(experiment, variant)
            }
        }
        return null
    }

    /**
     * Records an outcome for a specific experiment variant.
     * 记录特定实验变体的结果。
     *
     * @param experimentId The experiment identifier.
     *                      实验标识符。
     * @param variantId The variant identifier.
     *                   变体标识符。
     * @param success Whether the outcome was successful.
     *                结果是否成功。
     * @param runtimeMs Runtime in milliseconds.
     *                  运行时间（毫秒）。
     * @param cost Cost incurred.
     *             已产生的成本。
     * @return Updated ExperimentRegistry instance.
     *         更新后的ExperimentRegistry实例。
     */
    fun recordOutcome(
        experimentId: String,
        variantId: String,
        success: Boolean,
        runtimeMs: Long,
        cost: Double
    ): ExperimentRegistry {
        val experiment = experiments[experimentId] ?: return this
        val variantIndex = experiment.variants.indexOfFirst { it.id == variantId }
        if (variantIndex < 0) return this

        val updatedVariant = experiment.variants[variantIndex].copy(
            metrics = experiment.variants[variantIndex].metrics.record(success, runtimeMs, cost)
        )
        val updatedVariants = experiment.variants.toMutableList()
        updatedVariants[variantIndex] = updatedVariant

        val updatedExperiment = experiment.copy(
            variants = updatedVariants,
            metrics = experiment.metrics.record()
        )

        return ExperimentRegistry(experiments + (experimentId to updatedExperiment))
    }
}