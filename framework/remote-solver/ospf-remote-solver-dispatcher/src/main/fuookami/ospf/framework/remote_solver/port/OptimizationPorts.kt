/*
 * 优化端口接口集合
 *
 * Optimization Ports Interface Collection
 *
 * 该文件定义了与优化和调度相关的端口接口，包括任务家族性能追踪、可学习评分模型和实验管理。
 * This file defines port interfaces related to optimization and scheduling,
 * including task family performance tracking, learnable scoring models, and experiment management.
 *
 * 端口接口：
 * Port interfaces:
 * - TaskFamilyPort: 任务家族性能追踪 / Task family performance tracking
 * - ScoringModelPort: 可学习评分模型管理 / Learnable scoring model management
 * - ExperimentPort: 实验管理 / Experiment management
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.*

/**
 * 任务家族端口接口
 *
 * Task Family Port Interface
 *
 * 提供任务家族性能追踪功能的端口接口。
 * Port interface for task family performance tracking.
 */
interface TaskFamilyPort {
    /**
     * 获取任务家族性能数据
     *
     * Gets performance records for a task family.
     *
     * 检索指定任务家族在各节点上的性能表现。
     * Retrieves performance metrics for the specified task family across nodes.
     *
     * @param familyId 任务家族唯一标识符 / Unique task family identifier
     * @return 节点 ID 到性能数据的映射 / Map of node ID to performance data
     */
    suspend fun getPerformance(familyId: TaskFamilyId): Map<String, TaskFamilyNodePerformance>

    /**
     * 记录性能观测数据
     *
     * Records a performance observation.
     *
     * 记录一次任务执行的运行时间、成本和成功状态。
     * Records runtime, cost, and success status of a task execution.
     *
     * @param familyId 任务家族唯一标识符 / Unique task family identifier
     * @param nodeId 节点唯一标识符 / Unique node identifier
     * @param runtimeMs 运行时间（毫秒）/ Runtime in milliseconds
     * @param cost 执行成本 / Execution cost
     * @param success 是否成功 / Whether execution succeeded
     */
    suspend fun recordPerformance(
        familyId: TaskFamilyId,
        nodeId: String,
        runtimeMs: Long,
        cost: Double,
        success: Boolean
    )

    /**
     * 获取任务家族的最佳节点
     *
     * Gets the best performing node for a task family.
     *
     * 根据历史性能数据选择最适合执行该任务家族的节点。
     * Selects the most suitable node for executing the task family based on historical performance.
     *
     * @param familyId 任务家族唯一标识符 / Unique task family identifier
     * @param candidateNodes 候选节点 ID 列表 / List of candidate node IDs
     * @param minSamples 最小样本数要求（默认：5）/ Minimum sample count requirement (default: 5)
     * @return 最佳节点 ID，如无足够数据返回 null
     *         Best node ID, or null if insufficient data
     */
    suspend fun getBestNodeForFamily(
        familyId: TaskFamilyId,
        candidateNodes: List<String>,
        minSamples: Int = 5
    ): String?
}

/**
 * 评分模型端口接口
 *
 * Scoring Model Port Interface
 *
 * 提供可学习评分模型管理功能的端口接口。
 * Port interface for learnable scoring model management.
 */
interface ScoringModelPort {
    /**
     * 获取当前评分模型
     *
     * Gets the current scoring model.
     *
     * 获取当前活跃的评分模型，用于调度决策。
     * Retrieves the currently active scoring model for scheduling decisions.
     *
     * @return 当前评分模型 / Current scoring model
     */
    suspend fun getCurrentModel(): LearnableScoringModel

    /**
     * 更新评分模型
     *
     * Updates the scoring model.
     *
     * 使用新的模型参数更新评分模型。
     * Updates the scoring model with new model parameters.
     *
     * @param model 新的评分模型 / New scoring model
     */
    suspend fun updateModel(model: LearnableScoringModel)

    /**
     * 记录反馈数据
     *
     * Records feedback for model calibration.
     *
     * 记录一次调度的反馈结果，用于模型校准。
     * Records feedback from a scheduling decision for model calibration.
     *
     * @param feedback 评分反馈数据 / Scoring feedback data
     */
    suspend fun recordFeedback(feedback: ScoringFeedback)

    /**
     * 获取待处理反馈
     *
     * Gets pending feedback for calibration.
     *
     * 获取尚未处理的反馈数据，用于批量模型更新。
     * Retrieves unprocessed feedback data for batch model updates.
     *
     * @param limit 最大返回数量（默认：1000）/ Maximum number to return (default: 1000)
     * @return 待处理的反馈列表 / List of pending feedback
     */
    suspend fun getPendingFeedback(limit: Int = 1000): List<ScoringFeedback>

    /**
     * 清除已处理的反馈
     *
     * Clears processed feedback.
     *
     * 删除指定时间之前的已处理反馈数据。
     * Deletes processed feedback data before the specified time.
     *
     * @param olderThanEpochMs 时间阈值（毫秒级时间戳）/ Time threshold in milliseconds
     */
    suspend fun clearFeedback(olderThanEpochMs: Long)
}

/**
 * 实验端口接口
 *
 * Experiment Port Interface
 *
 * 提供实验管理功能的端口接口，支持 A/B 测试和特性开关。
 * Port interface for experiment management, supporting A/B testing and feature flags.
 */
interface ExperimentPort {
    /**
     * 获取活跃实验
     *
     * Gets active experiments.
     *
     * 获取当前所有正在进行的实验。
     * Retrieves all currently active experiments.
     *
     * @return 活跃实验列表 / List of active experiments
     */
    suspend fun getActiveExperiments(): List<Experiment>

    /**
     * 根据 ID 获取实验
     *
     * Gets experiment by ID.
     *
     * 根据实验 ID 获取实验详情。
     * Retrieves experiment details by its ID.
     *
     * @param experimentId 实验唯一标识符 / Unique experiment identifier
     * @return 实验对象，如不存在返回 null
     *         Experiment object, or null if not found
     */
    suspend fun getExperiment(experimentId: String): Experiment?

    /**
     * 创建实验
     *
     * Creates a new experiment.
     *
     * 创建一个新的实验，包含一个或多个变体。
     * Creates a new experiment with one or more variants.
     *
     * @param experiment 实验对象 / Experiment object
     */
    suspend fun createExperiment(experiment: Experiment)

    /**
     * 更新实验
     *
     * Updates an experiment.
     *
     * 更新现有实验的配置或状态。
     * Updates configuration or status of an existing experiment.
     *
     * @param experiment 实验对象 / Experiment object
     */
    suspend fun updateExperiment(experiment: Experiment)

    /**
     * 获取任务的变体分配
     *
     * Gets the variant assignment for a task.
     *
     * 获取指定任务被分配到的实验和变体。
     * Retrieves the experiment and variant assigned to the specified task.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @return 实验和变体对，如无分配返回 null
     *         Pair of experiment and variant, or null if no assignment
     */
    suspend fun getVariantForTask(taskId: String): Pair<Experiment, ExperimentVariant>?

    /**
     * 记录实验结果
     *
     * Records an outcome for an experiment variant.
     *
     * 记录一次任务执行的实验结果，用于统计分析。
     * Records the outcome of a task execution for statistical analysis.
     *
     * @param experimentId 实验唯一标识符 / Unique experiment identifier
     * @param variantId 变体唯一标识符 / Unique variant identifier
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @param success 是否成功 / Whether execution succeeded
     * @param runtimeMs 运行时间（毫秒）/ Runtime in milliseconds
     * @param cost 执行成本 / Execution cost
     */
    suspend fun recordOutcome(
        experimentId: String,
        variantId: String,
        taskId: String,
        success: Boolean,
        runtimeMs: Long,
        cost: Double
    )

    /**
     * 获取实验结果
     *
     * Gets experiment results.
     *
     * 获取实验的统计分析结果。
     * Retrieves statistical analysis results for the experiment.
     *
     * @param experimentId 实验唯一标识符 / Unique experiment identifier
     * @return 实验结果，如不存在返回 null
     *         Experiment results, or null if not found
     */
    suspend fun getExperimentResults(experimentId: String): ExperimentResults?
}

/**
 * 实验结果汇总
 *
 * Experiment Results Summary
 *
 * 包含实验的统计结果和变体比较数据。
 * Contains statistical results and variant comparison data for an experiment.
 *
 * @param experimentId 实验唯一标识符 / Unique experiment identifier
 * @param status 实验状态 / Experiment status
 * @param variants 变体结果列表 / List of variant results
 * @param hasWinner 是否已有胜出变体 / Whether a winner has been determined
 * @param winnerId 胜出变体 ID（可选）/ Winning variant ID (optional)
 * @param significanceLevel 统计显著性水平 / Statistical significance level
 * @param generatedAtEpochMs 结果生成时间的毫秒级时间戳 / Result generation timestamp in milliseconds
 */
data class ExperimentResults(
    val experimentId: String,
    val status: ExperimentStatus,
    val variants: List<VariantResult>,
    val hasWinner: Boolean,
    val winnerId: String? = null,
    val significanceLevel: Double,
    val generatedAtEpochMs: Long
)

/**
 * 变体结果
 *
 * Variant Result
 *
 * 包含单个变体在实验中的统计数据。
 * Contains statistical data for a single variant in an experiment.
 *
 * @param variantId 变体唯一标识符 / Unique variant identifier
 * @param variantName 变体名称 / Variant name
 * @param isControl 是否为对照组 / Whether this is the control group
 * @param sampleCount 样本数量 / Sample count
 * @param successRate 成功率 / Success rate
 * @param avgRuntimeMs 平均运行时间（毫秒）/ Average runtime in milliseconds
 * @param avgCost 平均成本 / Average cost
 * @param improvementOverControl 相对于对照组的改进百分比（可选）/ Improvement percentage over control (optional)
 * @param pValue 统计显著性 P 值（可选）/ Statistical significance p-value (optional)
 */
data class VariantResult(
    val variantId: String,
    val variantName: String,
    val isControl: Boolean,
    val sampleCount: Int,
    val successRate: Double,
    val avgRuntimeMs: Double,
    val avgCost: Double,
    val improvementOverControl: Double? = null,
    val pValue: Double? = null
)