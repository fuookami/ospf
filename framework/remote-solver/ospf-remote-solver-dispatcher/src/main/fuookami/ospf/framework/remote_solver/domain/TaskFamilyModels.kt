/*
 * Task Family Models
 * 任务家族模型
 *
 * This file defines the task family feature model for intelligent node selection.
 * 本文件定义了用于智能节点选择的任务家族特征模型。
 *
 * Task families group tasks with similar characteristics, enabling
 * more precise node selection based on historical performance data.
 * 任务家族将具有相似特征的任务分组，使基于历史性能数据的节点选择更精确。
 */
package fuookami.ospf.framework.remote_solver.domain

import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta

/**
 * Task Family Feature
 * 任务家族特征
 *
 * Task family feature model for intelligent node selection.
 * 用于智能节点选择的任务家族特征模型。
 *
 * Task families group tasks with similar characteristics, enabling
 * more precise node selection based on historical performance data.
 * 任务家族将具有相似特征的任务分组，使基于历史性能数据的节点选择更精确。
 *
 * @param variableScale Classification of problem variable scale.
 *                       问题变量规模的分类。
 * @param constraintDensity Classification of constraint density.
 *                           约束密度的分类。
 * @param convergencePattern Classification of convergence pattern.
 *                            收敛模式的分类。
 * @param solverTypeHint Optional hint for solver type preference.
 *                        求解器类型偏好的可选提示。
 */
data class TaskFamilyFeature(
    val variableScale: VariableScale,
    val constraintDensity: ConstraintDensity,
    val convergencePattern: ConvergencePattern,
    val solverTypeHint: String? = null
) {
    /**
     * Computes a feature vector for similarity comparison.
     * 计算用于相似性比较的特征向量。
     *
     * @return Feature vector as double array.
     *         特征向量（double数组）。
     */
    fun toFeatureVector(): DoubleArray {
        return doubleArrayOf(
            variableScale.score,
            constraintDensity.score,
            convergencePattern.score
        )
    }

    /**
     * Computes similarity to another task family (0.0 to 1.0).
     * 计算与另一个任务家族的相似度（0.0到1.0）。
     *
     * Uses normalized Euclidean distance: similarity = 1 - distance/maxDistance.
     * 使用规范化欧几里得距离：相似度 = 1 - 距离/最大距离。
     *
     * @param other The other task family to compare with.
     *              要比较的另一个任务家族。
     * @return Similarity score (0.0 = completely different, 1.0 = identical).
     *         相似度评分（0.0 = 完全不同，1.0 = 相同）。
     */
    fun similarityTo(other: TaskFamilyFeature): Double {
        val v1 = toFeatureVector()
        val v2 = other.toFeatureVector()

        // Compute squared distance
        // 计算平方距离
        var sumSquaredDiff = 0.0
        for (i in v1.indices) {
            val diff = v1[i] - v2[i]
            sumSquaredDiff += diff * diff
        }
        val distance = kotlin.math.sqrt(sumSquaredDiff)

        // Maximum possible distance based on score ranges
        // VariableScale: 0.1 to 0.9 -> maxDiff = 0.8
        // ConstraintDensity: 0.2 to 0.9 -> maxDiff = 0.7
        // ConvergencePattern: 0.2 to 0.8 -> maxDiff = 0.6
        // 基于评分范围的最大可能距离
        // VariableScale: 0.1 到 0.9 -> 最大差值 = 0.8
        // ConstraintDensity: 0.2 到 0.9 -> 最大差值 = 0.7
        // ConvergencePattern: 0.2 到 0.8 -> 最大差值 = 0.6
        val maxDistance = kotlin.math.sqrt(0.64 + 0.49 + 0.36) // sqrt(1.49) ≈ 1.22

        return 1.0 - (distance / maxDistance)
    }

    companion object {
        /**
         * Extracts features from task metadata.
         * 从任务元数据提取特征。
         *
         * @param meta The task metadata to analyze.
         *             要分析的任务元数据。
         * @return TaskFamilyFeature extracted from the metadata.
         *         从元数据提取的TaskFamilyFeature。
         */
        fun fromTaskMeta(meta: TaskMeta): TaskFamilyFeature {
            val variableScale = VariableScale.fromCount(meta.estimatedVariableCount)
            val constraintDensity = ConstraintDensity.fromCounts(
                variableCount = meta.estimatedVariableCount,
                constraintCount = meta.estimatedConstraintCount
            )
            val convergencePattern = ConvergencePattern.fromRuntime(meta.historicalRuntimeMs)
            return TaskFamilyFeature(
                variableScale = variableScale,
                constraintDensity = constraintDensity,
                convergencePattern = convergencePattern,
                solverTypeHint = meta.solverType?.value
            )
        }
    }
}

/**
 * Variable Scale
 * 变量规模
 *
 * Variable scale classification based on problem size.
 * 基于问题大小的变量规模分类。
 *
 * @param range Range of variable counts for this classification.
 *              此分类的变量计数范围。
 * @param score Normalized score for similarity calculation.
 *              用于相似度计算的规范化评分。
 */
enum class VariableScale(val range: IntRange, val score: Double) {
    /**
     * Tiny problems: 0-100 variables.
     * 微小问题：0-100个变量。
     */
    TINY(0..100, 0.1),

    /**
     * Small problems: 101-1000 variables.
     * 小型问题：101-1000个变量。
     */
    SMALL(101..1000, 0.3),

    /**
     * Medium problems: 1001-10000 variables.
     * 中型问题：1001-10000个变量。
     */
    MEDIUM(1001..10000, 0.5),

    /**
     * Large problems: 10001-100000 variables.
     * 大型问题：10001-100000个变量。
     */
    LARGE(10001..100000, 0.7),

    /**
     * Huge problems: 100001+ variables.
     * 超大问题：100001+个变量。
     */
    HUGE(100001..Int.MAX_VALUE, 0.9);

    companion object {
        /**
         * Classifies variable scale from a count.
         * 从计数分类变量规模。
         *
         * @param count The variable count (nullable).
         *              变量计数（可为null）。
         * @return The appropriate VariableScale classification.
         *         对应的VariableScale分类。
         */
        fun fromCount(count: Int?): VariableScale {
            if (count == null) return MEDIUM // Default assumption
            // 默认假设
            return entries.find { count in it.range } ?: HUGE
        }
    }
}

/**
 * Constraint Density
 * 约束密度
 *
 * Constraint density classification.
 * 约束密度分类。
 *
 * Density = constraints / variables.
 * 密度 = 约束数 / 变量数。
 *
 * Boundary values:
 * 边界值：
 * - <0.5 -> SPARSE
 * - <2.0 -> MODERATE
 * - <5.0 -> DENSE
 * - >=5.0 -> VERY_DENSE
 *
 * @param minDensity Minimum density value for this classification.
 *                    此分类的最小密度值。
 * @param maxDensityExclusive Maximum density value (exclusive) for this classification.
 *                              此分类的最大密度值（不含）。
 * @param score Normalized score for similarity calculation.
 *              用于相似度计算的规范化评分。
 */
enum class ConstraintDensity(val minDensity: Double, val maxDensityExclusive: Double, val score: Double) {
    /**
     * Sparse constraints: density < 0.5.
     * 稀疏约束：密度 < 0.5。
     */
    SPARSE(0.0, 0.5, 0.2),

    /**
     * Moderate constraints: density 0.5-2.0.
     * 中等约束：密度 0.5-2.0。
     */
    MODERATE(0.5, 2.0, 0.5),

    /**
     * Dense constraints: density 2.0-5.0.
     * 密集约束：密度 2.0-5.0。
     */
    DENSE(2.0, 5.0, 0.7),

    /**
     * Very dense constraints: density >= 5.0.
     * 极密集约束：密度 >= 5.0。
     */
    VERY_DENSE(5.0, Double.MAX_VALUE, 0.9);

    companion object {
        /**
         * Classifies constraint density from variable and constraint counts.
         * 从变量和约束计数分类约束密度。
         *
         * @param variableCount The variable count (nullable).
         *                       变量计数（可为null）。
         * @param constraintCount The constraint count (nullable).
         *                         约束计数（可为null）。
         * @return The appropriate ConstraintDensity classification.
         *         对应的ConstraintDensity分类。
         */
        fun fromCounts(variableCount: Int?, constraintCount: Int?): ConstraintDensity {
            if (variableCount == null || constraintCount == null) return MODERATE
            if (variableCount == 0) return MODERATE
            val density = constraintCount.toDouble() / variableCount.toDouble()
            // Use explicit boundary checks with exclusive upper bound
            // 使用显式边界检查（不含上界）
            return when {
                density < 0.5 -> SPARSE
                density < 2.0 -> MODERATE
                density < 5.0 -> DENSE
                else -> VERY_DENSE
            }
        }
    }
}

/**
 * Convergence Pattern
 * 收敛模式
 *
 * Convergence pattern based on historical runtime.
 * 基于历史运行时间的收敛模式。
 *
 * @param runtimeRangeMs Range of runtime in milliseconds for this classification.
 *                        此分类的运行时间范围（毫秒）。
 * @param score Normalized score for similarity calculation.
 *              用于相似度计算的规范化评分。
 */
enum class ConvergencePattern(val runtimeRangeMs: LongRange, val score: Double) {
    /**
     * Fast convergence: 0-5000ms.
     * 快速收敛：0-5000毫秒。
     */
    FAST(0L..5000L, 0.2),

    /**
     * Moderate convergence: 5001-30000ms.
     * 中等收敛：5001-30000毫秒。
     */
    MODERATE(5001L..30000L, 0.4),

    /**
     * Slow convergence: 30001-120000ms.
     * 缓慢收敛：30001-120000毫秒。
     */
    SLOW(30001L..120000L, 0.6),

    /**
     * Very slow convergence: 120001ms+.
     * 极慢收敛：120001毫秒+。
     */
    VERY_SLOW(120001L..Long.MAX_VALUE, 0.8);

    companion object {
        /**
         * Classifies convergence pattern from runtime.
         * 从运行时间分类收敛模式。
         *
         * @param runtimeMs The runtime in milliseconds (nullable).
         *                   运行时间（毫秒，可为null）。
         * @return The appropriate ConvergencePattern classification.
         *         对应的ConvergencePattern分类。
         */
        fun fromRuntime(runtimeMs: Long?): ConvergencePattern {
            if (runtimeMs == null) return MODERATE
            return entries.find { runtimeMs in it.runtimeRangeMs } ?: VERY_SLOW
        }
    }
}

/**
 * Task Family ID
 * 任务家族ID
 *
 * Task family identifier for grouping similar tasks.
 * 用于分组相似任务的任务家族标识符。
 *
 * @param signature Unique signature string for the family.
 *                   家族的唯一签名字符串。
 */
data class TaskFamilyId(
    val signature: String
) {
    companion object {
        /**
         * Creates a family ID from features.
         * 从特征创建家族ID。
         *
         * Combines first letters of feature classifications to create signature.
         * 组合特征分类的首字母来创建签名。
         *
         * @param feature The task family feature.
         *                任务家族特征。
         * @return TaskFamilyId derived from the feature.
         *         从特征派生的TaskFamilyId。
         */
        fun fromFeature(feature: TaskFamilyFeature): TaskFamilyId {
            val signature = buildString {
                append(feature.variableScale.name.first())
                append(feature.constraintDensity.name.first())
                append(feature.convergencePattern.name.first())
                feature.solverTypeHint?.let { append("-$it") }
            }
            return TaskFamilyId(signature)
        }
    }
}

/**
 * Task Family Node Performance
 * 任务家族节点性能
 *
 * Task family performance record for a specific node.
 * 特定节点的任务家族性能记录。
 *
 * Tracks historical performance metrics for a task family on a node.
 * 追踪任务家族在节点上的历史性能指标。
 *
 * @param familyId The task family identifier.
 *                  任务家族标识符。
 * @param nodeId The node identifier.
 *                节点标识符。
 * @param sampleCount Number of samples recorded.
 *                     已记录的样本数。
 * @param avgRuntimeMs Average runtime in milliseconds.
 *                      平均运行时间（毫秒）。
 * @param avgCostPerTask Average cost per task.
 *                        每任务平均成本。
 * @param successRate Success rate (0.0-1.0).
 *                     成功率（0.0-1.0）。
 * @param performanceScore Overall performance score.
 *                         整体性能评分。
 * @param lastUpdatedEpochMs Last update timestamp in epoch milliseconds.
 *                            最后更新时间戳（epoch毫秒）。
 */
data class TaskFamilyNodePerformance(
    val familyId: TaskFamilyId,
    val nodeId: String,
    val sampleCount: Int,
    val avgRuntimeMs: Double,
    val avgCostPerTask: Double,
    val successRate: Double,
    val performanceScore: Double,
    val lastUpdatedEpochMs: Long
) {
    /**
     * Confidence level based on sample count.
     * 基于样本数的置信度水平。
     *
     * Higher sample count yields higher confidence.
     * 更高的样本数产生更高的置信度。
     *
     * @return Confidence level (0.0-1.0).
     *         置信度水平（0.0-1.0）。
     */
    fun confidence(): Double {
        return when {
            sampleCount >= 100 -> 1.0
            sampleCount >= 50 -> 0.8
            sampleCount >= 20 -> 0.6
            sampleCount >= 10 -> 0.4
            sampleCount >= 5 -> 0.2
            else -> 0.1
        }
    }
}

/**
 * Task Family Registry
 * 任务家族注册表
 *
 * Task family registry for tracking performance across nodes.
 * 追踪跨节点性能的任务家族注册表。
 *
 * @param families Map of family:node keys to performance records.
 *                  家族:节点键到性能记录的映射。
 */
data class TaskFamilyRegistry(
    val families: Map<String, TaskFamilyNodePerformance> = emptyMap()
) {
    /**
     * Gets the best performing nodes for a task family.
     * 获取任务家族的最佳表现节点。
     *
     * Returns nodes with sufficient sample count.
     * 返回具有足够样本数的节点。
     *
     * @param familyId The task family identifier.
     *                  任务家族标识符。
     * @param minSamples Minimum samples required for consideration (default 5).
     *                    考虑所需的最小样本数（默认5）。
     * @return Map of nodeId to performance records.
     *         nodeId到性能记录的映射。
     */
    fun getBestNode(familyId: TaskFamilyId, minSamples: Int = 5): Map<String, TaskFamilyNodePerformance> {
        return families
            .filter { it.value.familyId == familyId && it.value.sampleCount >= minSamples }
            .values
            .associateBy { it.nodeId }
    }

    /**
     * Records a performance observation.
     * 记录性能观察。
     *
     * Updates or creates a performance record with exponential moving average.
     * 使用指数移动平均更新或创建性能记录。
     *
     * @param familyId The task family identifier.
     *                  任务家族标识符。
     * @param nodeId The node identifier.
     *                节点标识符。
     * @param runtimeMs Runtime in milliseconds.
     *                  运行时间（毫秒）。
     * @param cost Cost incurred.
     *             已产生的成本。
     * @param success Whether the task succeeded.
     *                任务是否成功。
     * @param timestampEpochMs Observation timestamp in epoch milliseconds.
     *                          观察时间戳（epoch毫秒）。
     * @return Updated TaskFamilyRegistry instance.
     *         更新后的TaskFamilyRegistry实例。
     */
    fun record(
        familyId: TaskFamilyId,
        nodeId: String,
        runtimeMs: Long,
        cost: Double,
        success: Boolean,
        timestampEpochMs: Long
    ): TaskFamilyRegistry {
        val key = "${familyId.signature}:$nodeId"
        val existing = families[key]
        val newRecord = if (existing != null) {
            val newCount = existing.sampleCount + 1
            val alpha = 1.0 / newCount.toDouble()
            TaskFamilyNodePerformance(
                familyId = familyId,
                nodeId = nodeId,
                sampleCount = newCount,
                avgRuntimeMs = existing.avgRuntimeMs * (1 - alpha) + runtimeMs.toDouble() * alpha,
                avgCostPerTask = existing.avgCostPerTask * (1 - alpha) + cost * alpha,
                successRate = existing.successRate * (1 - alpha) + (if (success) 1.0 else 0.0) * alpha,
                performanceScore = existing.performanceScore, // Recalculated separately
                // 单独重新计算
                lastUpdatedEpochMs = timestampEpochMs
            )
        } else {
            TaskFamilyNodePerformance(
                familyId = familyId,
                nodeId = nodeId,
                sampleCount = 1,
                avgRuntimeMs = runtimeMs.toDouble(),
                avgCostPerTask = cost,
                successRate = if (success) 1.0 else 0.0,
                performanceScore = 1.0,
                lastUpdatedEpochMs = timestampEpochMs
            )
        }
        return TaskFamilyRegistry(families + (key to newRecord))
    }
}
