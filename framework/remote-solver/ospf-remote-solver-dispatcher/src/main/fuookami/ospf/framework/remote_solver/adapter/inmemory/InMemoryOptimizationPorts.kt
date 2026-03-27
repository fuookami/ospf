/**
 * 内存优化相关端口适配器
 *
 * 提供基于内存的优化相关功能实现，包括任务族性能追踪、评分模型管理和实验框架。
 * 用于测试和非持久化场景。
 *
 * In-memory optimization-related port adapters.
 *
 * Provides memory-based optimization feature implementations, including task family performance tracking,
 * scoring model management, and experiment framework. Used for testing and non-persistent scenarios.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.domain.Experiment
import fuookami.ospf.framework.remote_solver.domain.ExperimentStatus
import fuookami.ospf.framework.remote_solver.domain.ExperimentVariant
import fuookami.ospf.framework.remote_solver.domain.LearnableScoringModel
import fuookami.ospf.framework.remote_solver.domain.ScoringFeedback
import fuookami.ospf.framework.remote_solver.domain.TaskFamilyId
import fuookami.ospf.framework.remote_solver.domain.TaskFamilyNodePerformance
import fuookami.ospf.framework.remote_solver.port.ExperimentPort
import fuookami.ospf.framework.remote_solver.port.ExperimentResults
import fuookami.ospf.framework.remote_solver.port.ScoringModelPort
import fuookami.ospf.framework.remote_solver.port.TaskFamilyPort
import fuookami.ospf.framework.remote_solver.port.VariantResult

/**
 * 内存任务族端口实现
 *
 * 使用内存存储任务族在各节点上的性能数据，支持增量更新和最佳节点选择。
 *
 * In-memory task family port implementation.
 *
 * Uses in-memory storage for task family performance data on each node, supporting incremental updates
 * and best node selection.
 */
class InMemoryTaskFamilyPort : TaskFamilyPort {
    /**
     * 性能记录存储
     *
     * 键为任务族签名，值为节点性能记录列表。
     *
     * Performance record storage.
     *
     * Keyed by task family signature, valued by node performance record list.
     */
    private val performanceRecords = mutableMapOf<String, MutableList<TaskFamilyNodePerformance>>()

    /**
     * 获取任务族性能数据
     *
     * 返回指定任务族在各节点上的性能统计，以节点ID为键。
     *
     * Gets task family performance data.
     *
     * Returns performance statistics for the specified task family on each node, keyed by node ID.
     *
     * @param familyId 任务族标识
     *                 Task family identifier
     * @return 节点ID到性能数据的映射
     *         Node ID to performance data map
     */
    override suspend fun getPerformance(familyId: TaskFamilyId): Map<String, TaskFamilyNodePerformance> {
        return performanceRecords[familyId.signature]
            ?.associateBy { it.nodeId }
            ?: emptyMap()
    }

    /**
     * 记录任务族性能数据
     *
     * 增量更新指定任务族在特定节点上的性能统计，使用滑动平均算法。
     *
     * Records task family performance data.
     *
     * Incrementally updates performance statistics for the specified task family on a specific node,
     * using sliding average algorithm.
     *
     * @param familyId 任务族标识
     *                 Task family identifier
     * @param nodeId 节点ID
     *               Node ID
     * @param runtimeMs 运行时间（毫秒）
     *                   Runtime in milliseconds
     * @param cost 执行成本
     *             Execution cost
     * @param success 是否成功
     *                 Whether execution was successful
     */
    override suspend fun recordPerformance(
        familyId: TaskFamilyId,
        nodeId: String,
        runtimeMs: Long,
        cost: Double,
        success: Boolean
    ) {
        val key = familyId.signature
        val records = performanceRecords.getOrPut(key) { mutableListOf() }

        val existingIndex = records.indexOfFirst { it.nodeId == nodeId }
        if (existingIndex >= 0) {
            val existing = records[existingIndex]
            val newCount = existing.sampleCount + 1
            val alpha = 1.0 / newCount.toDouble()
            records[existingIndex] = existing.copy(
                sampleCount = newCount,
                avgRuntimeMs = existing.avgRuntimeMs * (1 - alpha) + runtimeMs.toDouble() * alpha,
                avgCostPerTask = existing.avgCostPerTask * (1 - alpha) + cost * alpha,
                successRate = existing.successRate * (1 - alpha) + (if (success) 1.0 else 0.0) * alpha,
                lastUpdatedEpochMs = System.currentTimeMillis()
            )
        } else {
            records.add(
                TaskFamilyNodePerformance(
                    familyId = familyId,
                    nodeId = nodeId,
                    sampleCount = 1,
                    avgRuntimeMs = runtimeMs.toDouble(),
                    avgCostPerTask = cost,
                    successRate = if (success) 1.0 else 0.0,
                    performanceScore = 1.0,
                    lastUpdatedEpochMs = System.currentTimeMillis()
                )
            )
        }
    }

    /**
     * 获取任务族最佳执行节点
     *
     * 根据历史性能数据选择最适合执行指定任务族的节点。
     *
     * Gets best execution node for task family.
     *
     * Selects the most suitable node for executing the specified task family based on historical performance data.
     *
     * @param familyId 任务族标识
     *                 Task family identifier
     * @param candidateNodes 候选节点ID列表
     *                       Candidate node ID list
     * @param minSamples 最小样本数量要求
     *                    Minimum sample count requirement
     * @return 最佳节点ID，如果无足够数据则返回null
     *         Best node ID, returns null if insufficient data
     */
    override suspend fun getBestNodeForFamily(
        familyId: TaskFamilyId,
        candidateNodes: List<String>,
        minSamples: Int
    ): String? {
        val performances = getPerformance(familyId)
            .filter { it.value.sampleCount >= minSamples }
            .filterKeys { it in candidateNodes }

        if (performances.isEmpty()) return null

        return performances.entries
            .minWithOrNull(compareBy { -it.value.successRate * 100 + it.value.avgCostPerTask * 10 })
            ?.key
    }

    /**
     * 清空性能记录
     *
     * 删除所有存储的性能数据。
     *
     * Clears performance records.
     *
     * Deletes all stored performance data.
     */
    fun clear() {
        performanceRecords.clear()
    }
}

/**
 * 内存评分模型端口实现
 *
 * 使用内存存储可学习的评分模型和反馈数据，支持模型更新和反馈收集。
 *
 * In-memory scoring model port implementation.
 *
 * Uses in-memory storage for learnable scoring model and feedback data, supporting model updates
 * and feedback collection.
 *
 * @param initialModel 初始评分模型，默认为DEFAULT
 *                     Initial scoring model, defaults to DEFAULT
 */
class InMemoryScoringModelPort(
    initialModel: LearnableScoringModel = LearnableScoringModel.DEFAULT
) : ScoringModelPort {
    /**
     * 当前评分模型
     *
     * 存储当前使用的评分模型。
     *
     * Current scoring model.
     *
     * Stores the currently used scoring model.
     */
    private var currentModel = initialModel

    /**
     * 反馈数据列表
     *
     * 存储收集的评分反馈数据。
     *
     * Feedback data list.
     *
     * Stores collected scoring feedback data.
     */
    private val feedbackList = mutableListOf<ScoringFeedback>()

    /**
     * 获取当前评分模型
     *
     * 返回当前使用的评分模型。
     *
     * Gets current scoring model.
     *
     * Returns the currently used scoring model.
     *
     * @return 当前评分模型
     *         Current scoring model
     */
    override suspend fun getCurrentModel(): LearnableScoringModel {
        return currentModel
    }

    /**
     * 更新评分模型
     *
     * 更新当前使用的评分模型。
     *
     * Updates scoring model.
     *
     * Updates the currently used scoring model.
     *
     * @param model 新的评分模型
     *              New scoring model
     */
    override suspend fun updateModel(model: LearnableScoringModel) {
        currentModel = model
    }

    /**
     * 记录评分反馈
     *
     * 将评分反馈数据添加到反馈列表中。
     *
     * Records scoring feedback.
     *
     * Adds scoring feedback data to the feedback list.
     *
     * @param feedback 评分反馈
     *                  Scoring feedback
     */
    override suspend fun recordFeedback(feedback: ScoringFeedback) {
        feedbackList.add(feedback)
    }

    /**
     * 获取待处理的反馈数据
     *
     * 返回最近指定数量的反馈数据。
     *
     * Gets pending feedback data.
     *
     * Returns the most recent feedback data up to the specified limit.
     *
     * @param limit 最大返回数量
     *              Maximum return count
     * @return 反馈数据列表
     *         Feedback data list
     */
    override suspend fun getPendingFeedback(limit: Int): List<ScoringFeedback> {
        return feedbackList.takeLast(limit)
    }

    /**
     * 清理旧反馈数据
     *
     * 删除指定时间之前的反馈数据。
     *
     * Clears old feedback data.
     *
     * Deletes feedback data before the specified time.
     *
     * @param olderThanEpochMs 时间阈值（毫秒）
     *                          Time threshold in milliseconds
     */
    override suspend fun clearFeedback(olderThanEpochMs: Long) {
        feedbackList.removeAll { it.timestampEpochMs < olderThanEpochMs }
    }

    /**
     * 清空所有数据
     *
     * 重置评分模型和清空反馈列表。
     *
     * Clears all data.
     *
     * Resets scoring model and clears feedback list.
     */
    fun clear() {
        feedbackList.clear()
        currentModel = LearnableScoringModel.DEFAULT
    }
}

/**
 * 内存实验端口实现
 *
 * 使用内存存储A/B实验数据，支持实验创建、更新、变体分配和结果统计。
 *
 * In-memory experiment port implementation.
 *
 * Uses in-memory storage for A/B experiment data, supporting experiment creation, updates,
 * variant assignment, and result statistics.
 */
class InMemoryExperimentPort : ExperimentPort {
    /**
     * 实验存储映射
     *
     * 键为实验ID，值为实验数据。
     *
     * Experiment storage map.
     *
     * Keyed by experiment ID, valued by experiment data.
     */
    private val experiments = mutableMapOf<String, Experiment>()

    /**
     * 获取活跃实验列表
     *
     * 返回所有运行中的实验。
     *
     * Gets active experiment list.
     *
     * Returns all running experiments.
     *
     * @return 活跃实验列表
     *         Active experiment list
     */
    override suspend fun getActiveExperiments(): List<Experiment> {
        return experiments.values.filter { it.status == ExperimentStatus.RUNNING }
    }

    /**
     * 获取实验数据
     *
     * 根据实验ID获取实验数据。
     *
     * Gets experiment data.
     *
     * Retrieves experiment data by experiment ID.
     *
     * @param experimentId 实验ID
     *                     Experiment ID
     * @return 实验数据，如果不存在则返回null
     *         Experiment data, returns null if not found
     */
    override suspend fun getExperiment(experimentId: String): Experiment? {
        return experiments[experimentId]
    }

    /**
     * 创建实验
     *
     * 将新实验数据存储到内存中。
     *
     * Creates experiment.
     *
     * Stores new experiment data in memory.
     *
     * @param experiment 实验数据
     *                   Experiment data
     */
    override suspend fun createExperiment(experiment: Experiment) {
        experiments[experiment.experimentId] = experiment
    }

    /**
     * 更新实验
     *
     * 更新现有实验数据。
     *
     * Updates experiment.
     *
     * Updates existing experiment data.
     *
     * @param experiment 实验数据
     *                   Experiment data
     */
    override suspend fun updateExperiment(experiment: Experiment) {
        experiments[experiment.experimentId] = experiment
    }

    /**
     * 获取任务的实验变体
     *
     * 根据任务ID查找所属的活跃实验和分配的变体。
     *
     * Gets experiment variant for task.
     *
     * Finds the active experiment and assigned variant for a task by task ID.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 实验和变体的配对，如果不属于任何实验则返回null
     *         Experiment and variant pair, returns null if not in any experiment
     */
    override suspend fun getVariantForTask(taskId: String): Pair<Experiment, ExperimentVariant>? {
        for (experiment in getActiveExperiments()) {
            val variant = experiment.getVariant(taskId)
            if (variant != null) {
                return Pair(experiment, variant)
            }
        }
        return null
    }

    /**
     * 记录实验结果
     *
     * 更新实验变体的执行结果统计数据。
     *
     * Records experiment outcome.
     *
     * Updates execution result statistics for an experiment variant.
     *
     * @param experimentId 实验ID
     *                     Experiment ID
     * @param variantId 变体ID
     *                   Variant ID
     * @param taskId 任务ID
     *               Task ID
     * @param success 是否成功
     *                 Whether execution was successful
     * @param runtimeMs 运行时间（毫秒）
     *                   Runtime in milliseconds
     * @param cost 执行成本
     *             Execution cost
     */
    override suspend fun recordOutcome(
        experimentId: String,
        variantId: String,
        taskId: String,
        success: Boolean,
        runtimeMs: Long,
        cost: Double
    ) {
        val experiment = experiments[experimentId] ?: return
        val variantIndex = experiment.variants.indexOfFirst { it.id == variantId }
        if (variantIndex < 0) return

        val updatedVariant = experiment.variants[variantIndex].copy(
            metrics = experiment.variants[variantIndex].metrics.record(success, runtimeMs, cost)
        )
        val updatedVariants = experiment.variants.toMutableList()
        updatedVariants[variantIndex] = updatedVariant

        experiments[experimentId] = experiment.copy(
            variants = updatedVariants,
            metrics = experiment.metrics.record()
        )
    }

    /**
     * 获取实验结果
     *
     * 计算并返回实验的统计结果，包括各变体的性能指标和胜者判定。
     *
     * Gets experiment results.
     *
     * Calculates and returns experiment statistical results, including performance metrics
     * for each variant and winner determination.
     *
     * @param experimentId 实验ID
     *                     Experiment ID
     * @return 实验结果，如果实验不存在则返回null
     *         Experiment results, returns null if experiment not found
     */
    override suspend fun getExperimentResults(experimentId: String): ExperimentResults? {
        val experiment = experiments[experimentId] ?: return null
        val controlVariant = experiment.variants.find { it.isControl } ?: return null

        val variantResults = experiment.variants.map { variant ->
            val improvementOverControl = if (!variant.isControl && controlVariant.metrics.sampleCount > 0) {
                val controlValue = controlVariant.metrics.getMetricValue(experiment.config.primaryMetric)
                val variantValue = variant.metrics.getMetricValue(experiment.config.primaryMetric)
                if (controlValue != 0.0) {
                    (variantValue - controlValue) / controlValue * 100
                } else null
            } else null

            VariantResult(
                variantId = variant.id,
                variantName = variant.name,
                isControl = variant.isControl,
                sampleCount = variant.metrics.sampleCount,
                successRate = variant.metrics.avgSuccessRate,
                avgRuntimeMs = variant.metrics.avgRuntimeMs,
                avgCost = variant.metrics.avgCost,
                improvementOverControl = improvementOverControl
            )
        }

        val winner = experiment.getWinner()

        return ExperimentResults(
            experimentId = experimentId,
            status = experiment.status,
            variants = variantResults,
            hasWinner = winner != null,
            winnerId = winner?.id,
            significanceLevel = experiment.config.significanceLevel,
            generatedAtEpochMs = System.currentTimeMillis()
        )
    }

    /**
     * 清空所有实验数据
     *
     * 删除所有存储的实验数据。
     *
     * Clears all experiment data.
     *
     * Deletes all stored experiment data.
     */
    fun clear() {
        experiments.clear()
    }
}