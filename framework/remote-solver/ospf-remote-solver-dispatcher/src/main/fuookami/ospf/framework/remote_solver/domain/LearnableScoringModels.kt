/*
 * Learnable Scoring Models
 * 可学习评分模型
 *
 * This file defines the learnable scoring model for intelligent node selection.
 * 本文件定义了用于智能节点选择的可学习评分模型。
 *
 * The model supports:
 * 该模型支持：
 * - Offline training from historical data
 * - 从历史数据离线训练
 * - Online calibration based on real-time feedback
 * - 基于实时反馈的在线校准
 * - A/B testing of different scoring strategies
 * - 不同评分策略的A/B测试
 */
package fuookami.ospf.framework.remote_solver.domain

import kotlinx.serialization.Serializable

/**
 * Learnable Scoring Model
 * 可学习评分模型
 *
 * Learnable scoring model for intelligent node selection.
 * 用于智能节点选择的可学习评分模型。
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
 * @param version Model version identifier.
 *                模型版本标识符。
 * @param weights Scoring weights for different features.
 *                不同特征的评分权重。
 * @param featureImportance Importance scores for each feature (optional).
 *                          每个特征的重要性评分（可选）。
 * @param trainingMetrics Training quality metrics (optional).
 *                         训练质量指标（可选）。
 * @param updatedAtEpochMs Last update timestamp in epoch milliseconds.
 *                         最后更新时间戳（epoch毫秒）。
 */
@Serializable
data class LearnableScoringModel(
    val version: String,
    val weights: ScoringWeights,
    val featureImportance: Map<String, Double> = emptyMap(),
    val trainingMetrics: TrainingMetrics? = null,
    val updatedAtEpochMs: Long
) {
    /**
     * Computes a score for a node given task and node features.
     * 根据任务和节点特征计算节点的评分。
     *
     * Uses weighted combination of normalized features.
     * 使用规范化特征的加权组合。
     *
     * @param features The node scoring features.
     *                 节点评分特征。
     * @return The computed score (higher is better).
     *         计算的评分（越高越好）。
     */
    fun score(features: NodeScoreFeatures): Double {
        val w = weights
        return w.costWeight * features.normalizedCost +
            w.performanceWeight * (1.0 - features.normalizedPerformance) +
            w.queueDelayWeight * features.normalizedQueueDelay +
            w.successRateWeight * (1.0 - features.normalizedSuccessRate) +
            w.matchScoreWeight * features.familyMatchScore
    }

    /**
     * Creates an updated model with calibrated weights.
     * 创建带有校准权重的更新模型。
     *
     * Uses gradient-based weight adjustment based on feedback.
     * 使用基于反馈的梯度权重调整。
     *
     * @param feedback List of scoring feedback samples.
     *                 评分反馈样本列表。
     * @param learningRate Learning rate for weight adjustment (default 0.1).
     *                     权重调整的学习率（默认0.1）。
     * @return Updated LearnableScoringModel with calibrated weights.
     *         带有校准权重的更新LearnableScoringModel。
     */
    fun calibrate(
        feedback: List<ScoringFeedback>,
        learningRate: Double = 0.1
    ): LearnableScoringModel {
        if (feedback.isEmpty()) return this

        // Simple gradient-based weight adjustment
        // 简单的梯度权重调整
        val gradient = mutableMapOf<String, Double>()
        feedback.forEach { f ->
            val error = f.predictedScore - f.actualScore
            gradient["cost"] = gradient.getOrDefault("cost", 0.0) + error * f.features.normalizedCost
            gradient["performance"] = gradient.getOrDefault("performance", 0.0) + error * (1.0 - f.features.normalizedPerformance)
            gradient["queueDelay"] = gradient.getOrDefault("queueDelay", 0.0) + error * f.features.normalizedQueueDelay
            gradient["successRate"] = gradient.getOrDefault("successRate", 0.0) + error * (1.0 - f.features.normalizedSuccessRate)
        }

        val n = feedback.size.toDouble()
        val newWeights = ScoringWeights(
            costWeight = (weights.costWeight - learningRate * gradient.getOrDefault("cost", 0.0) / n).coerceIn(0.0, 1.0),
            performanceWeight = (weights.performanceWeight - learningRate * gradient.getOrDefault("performance", 0.0) / n).coerceIn(0.0, 1.0),
            queueDelayWeight = (weights.queueDelayWeight - learningRate * gradient.getOrDefault("queueDelay", 0.0) / n).coerceIn(0.0, 1.0),
            successRateWeight = (weights.successRateWeight - learningRate * gradient.getOrDefault("successRate", 0.0) / n).coerceIn(0.0, 1.0),
            matchScoreWeight = weights.matchScoreWeight
        )

        // Normalize weights to sum to 1.0
        // 归一化权重使其总和为1.0
        val total = newWeights.costWeight + newWeights.performanceWeight +
            newWeights.queueDelayWeight + newWeights.successRateWeight + newWeights.matchScoreWeight
        val normalizedWeights = if (total > 0) {
            newWeights.copy(
                costWeight = newWeights.costWeight / total,
                performanceWeight = newWeights.performanceWeight / total,
                queueDelayWeight = newWeights.queueDelayWeight / total,
                successRateWeight = newWeights.successRateWeight / total,
                matchScoreWeight = newWeights.matchScoreWeight / total
            )
        } else {
            newWeights
        }

        return LearnableScoringModel(
            version = "${version.split("-").first()}-${System.currentTimeMillis()}",
            weights = normalizedWeights,
            featureImportance = featureImportance,
            trainingMetrics = trainingMetrics,
            updatedAtEpochMs = System.currentTimeMillis()
        )
    }

    companion object {
        /**
         * Default scoring model with balanced weights.
         * 具有均衡权重的默认评分模型。
         */
        val DEFAULT = LearnableScoringModel(
            version = "default-1.0",
            weights = ScoringWeights(
                costWeight = 0.35,
                performanceWeight = 0.25,
                queueDelayWeight = 0.15,
                successRateWeight = 0.15,
                matchScoreWeight = 0.10
            ),
            updatedAtEpochMs = System.currentTimeMillis()
        )

        /**
         * Cost-optimized model.
         * 成本优化模型。
         *
         * Emphasizes cost efficiency over performance.
         * 强调成本效率而非性能。
         */
        val COST_OPTIMIZED = LearnableScoringModel(
            version = "cost-optimized-1.0",
            weights = ScoringWeights(
                costWeight = 0.50,
                performanceWeight = 0.20,
                queueDelayWeight = 0.10,
                successRateWeight = 0.10,
                matchScoreWeight = 0.10
            ),
            updatedAtEpochMs = System.currentTimeMillis()
        )

        /**
         * Performance-optimized model.
         * 性能优化模型。
         *
         * Emphasizes performance over cost efficiency.
         * 强调性能而非成本效率。
         */
        val PERFORMANCE_OPTIMIZED = LearnableScoringModel(
            version = "performance-optimized-1.0",
            weights = ScoringWeights(
                costWeight = 0.20,
                performanceWeight = 0.40,
                queueDelayWeight = 0.15,
                successRateWeight = 0.15,
                matchScoreWeight = 0.10
            ),
            updatedAtEpochMs = System.currentTimeMillis()
        )
    }
}

/**
 * Scoring Weights
 * 评分权重
 *
 * Weights for different features in scoring calculation.
 * 评分计算中不同特征的权重。
 *
 * All weights should be in range [0.0, 1.0] and ideally sum to 1.0.
 * 所有权重应在[0.0, 1.0]范围内，理想情况下总和为1.0。
 *
 * @param costWeight Weight for normalized cost feature (default 0.35).
 *                   规范化成本特征的权重（默认0.35）。
 * @param performanceWeight Weight for normalized performance feature (default 0.25).
 *                          规范化性能特征的权重（默认0.25）。
 * @param queueDelayWeight Weight for normalized queue delay feature (default 0.15).
 *                         规范化队列延迟特征的权重（默认0.15）。
 * @param successRateWeight Weight for normalized success rate feature (default 0.15).
 *                          规范化成功率特征的权重（默认0.15）。
 * @param matchScoreWeight Weight for family match score feature (default 0.10).
 *                         家族匹配评分特征的权重（默认0.10）。
 */
@Serializable
data class ScoringWeights(
    val costWeight: Double = 0.35,
    val performanceWeight: Double = 0.25,
    val queueDelayWeight: Double = 0.15,
    val successRateWeight: Double = 0.15,
    val matchScoreWeight: Double = 0.10
)

/**
 * Node Score Features
 * 节点评分特征
 *
 * Features used for node scoring in the scheduling decision.
 * 调度决策中用于节点评分的特征。
 *
 * All features should be normalized to [0.0, 1.0] range.
 * 所有特征应规范化到[0.0, 1.0]范围。
 *
 * @param nodeId The node identifier.
 *               节点标识符。
 * @param normalizedCost Normalized cost (0.0 = cheapest, 1.0 = most expensive).
 *                       规范化成本（0.0 = 最便宜，1.0 = 最昂贵）。
 * @param normalizedPerformance Normalized performance (0.0 = slowest, 1.0 = fastest).
 *                              规范化性能（0.0 = 最慢，1.0 = 最快）。
 * @param normalizedQueueDelay Normalized queue delay (0.0 = no delay, 1.0 = max delay).
 *                             规范化队列延迟（0.0 = 无延迟，1.0 = 最大延迟）。
 * @param normalizedSuccessRate Normalized success rate (0.0 = lowest, 1.0 = highest).
 *                              规范化成功率（0.0 = 最低，1.0 = 最高）。
 * @param familyMatchScore Task family match score (0.0 = no match, 1.0 = perfect match).
 *                         任务家族匹配评分（0.0 = 无匹配，1.0 = 完美匹配）。
 */
@Serializable
data class NodeScoreFeatures(
    val nodeId: String,
    val normalizedCost: Double,
    val normalizedPerformance: Double,
    val normalizedQueueDelay: Double,
    val normalizedSuccessRate: Double,
    val familyMatchScore: Double = 0.0
)

/**
 * Scoring Feedback
 * 评分反馈
 *
 * Feedback data for model calibration.
 * 用于模型校准的反馈数据。
 *
 * Contains predicted vs actual scores for adjusting weights.
 * 包含预测评分与实际评分，用于调整权重。
 *
 * @param taskId The task identifier.
 *               任务标识符。
 * @param nodeId The node that processed the task.
 *               处理任务的节点。
 * @param features The features used for scoring.
 *                 用于评分的特征。
 * @param predictedScore The predicted score before execution.
 *                       执行前的预测评分。
 * @param actualScore The actual outcome score after execution.
 *                    执行后的实际结果评分。
 * @param outcome The task outcome classification.
 *                任务结果分类。
 * @param timestampEpochMs Timestamp when this feedback was recorded.
 *                          此反馈记录时的时间戳。
 */
@Serializable
data class ScoringFeedback(
    val taskId: String,
    val nodeId: String,
    val features: NodeScoreFeatures,
    val predictedScore: Double,
    val actualScore: Double,
    val outcome: TaskOutcome,
    val timestampEpochMs: Long
)

/**
 * Task Outcome
 * 任务结果
 *
 * Classification of task execution outcome for feedback.
 * 用于反馈的任务执行结果分类。
 *
 * Used to categorize execution results for model calibration.
 * 用于对执行结果分类以进行模型校准。
 */
@Serializable
enum class TaskOutcome {
    /**
     * Completed faster than expected.
     * 比预期更快完成。
     */
    SUCCESS_FAST,

    /**
     * Completed as expected.
     * 按预期完成。
     */
    SUCCESS_NORMAL,

    /**
     * Completed but slower than expected.
     * 完成但比预期慢。
     */
    SUCCESS_SLOW,

    /**
     * Failed due to timeout.
     * 因超时失败。
     */
    FAILED_TIMEOUT,

    /**
     * Failed due to error.
     * 因错误失败。
     */
    FAILED_ERROR
}

/**
 * Training Metrics
 * 训练指标
 *
 * Quality metrics for a trained model.
 * 训练模型的质量指标。
 *
 * @param sampleCount Number of samples used for training.
 *                    用于训练的样本数。
 * @param meanAbsoluteError Mean absolute error of predictions.
 *                           预测的平均绝对误差。
 * @param rootMeanSquareError Root mean square error of predictions.
 *                             预测的均方根误差。
 * @param r2Score R-squared score (coefficient of determination, optional).
 *                R平方评分（决定系数，可选）。
 * @param trainedAtEpochMs Training completion timestamp in epoch milliseconds.
 *                         训练完成时间戳（epoch毫秒）。
 */
@Serializable
data class TrainingMetrics(
    val sampleCount: Int,
    val meanAbsoluteError: Double,
    val rootMeanSquareError: Double,
    val r2Score: Double? = null,
    val trainedAtEpochMs: Long
)

/**
 * Scoring Model Config
 * 评分模型配置
 *
 * Configuration for the scoring model behavior.
 * 评分模型行为的配置。
 *
 * @param enabled Whether learnable scoring is enabled (default true).
 *                 是否启用可学习评分（默认true）。
 * @param modelType Type of scoring model to use (default DEFAULT).
 *                   要使用的评分模型类型（默认DEFAULT）。
 * @param learningRate Learning rate for online calibration (default 0.1).
 *                     在线校准的学习率（默认0.1）。
 * @param calibrationInterval Number of samples between calibration runs (default 100).
 *                             校准运行之间的样本数（默认100）。
 * @param minSamplesForCalibration Minimum samples required for calibration (default 50).
 *                                 校准所需的最小样本数（默认50）。
 */
data class ScoringModelConfig(
    val enabled: Boolean = true,
    val modelType: ModelType = ModelType.DEFAULT,
    val learningRate: Double = 0.1,
    val calibrationInterval: Int = 100,
    val minSamplesForCalibration: Int = 50
) {
    /**
     * Model Type
     * 模型类型
     *
     * Types of scoring models available.
     * 可用的评分模型类型。
     */
    enum class ModelType {
        /**
         * Default balanced model.
         * 默认均衡模型。
         */
        DEFAULT,

        /**
         * Cost-optimized model.
         * 成本优化模型。
         */
        COST_OPTIMIZED,

        /**
         * Performance-optimized model.
         * 性能优化模型。
         */
        PERFORMANCE_OPTIMIZED,

        /**
         * Learned model (trained from data).
         * 学习模型（从数据训练）。
         */
        LEARNED
    }

    /**
     * Gets the initial model based on configuration.
     * 根据配置获取初始模型。
     *
     * @return The appropriate LearnableScoringModel instance.
     *         对应的LearnableScoringModel实例。
     */
    fun getInitialModel(): LearnableScoringModel {
        return when (modelType) {
            ModelType.COST_OPTIMIZED -> LearnableScoringModel.COST_OPTIMIZED
            ModelType.PERFORMANCE_OPTIMIZED -> LearnableScoringModel.PERFORMANCE_OPTIMIZED
            else -> LearnableScoringModel.DEFAULT
        }
    }
}