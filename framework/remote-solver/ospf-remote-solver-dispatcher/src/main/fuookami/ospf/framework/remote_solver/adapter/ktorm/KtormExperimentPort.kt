/**
 * 实验管理端口模块
 *
 * 本模块提供基于 Ktorm 的 A/B 实验管理功能，
 * 用于远程求解器调度器的策略实验和性能对比分析。
 *
 * Experiment Management Port Module
 *
 * This module provides A/B experiment management functionality based on Ktorm,
 * used for strategy experiments and performance comparison analysis
 * in the remote solver dispatcher.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.Experiment
import fuookami.ospf.framework.remote_solver.domain.ExperimentConfig
import fuookami.ospf.framework.remote_solver.domain.ExperimentMetrics
import fuookami.ospf.framework.remote_solver.domain.ExperimentStatus
import fuookami.ospf.framework.remote_solver.domain.ExperimentVariant
import fuookami.ospf.framework.remote_solver.domain.VariantMetrics
import fuookami.ospf.framework.remote_solver.port.ExperimentPort
import fuookami.ospf.framework.remote_solver.port.ExperimentResults
import fuookami.ospf.framework.remote_solver.port.VariantResult
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.ktorm.database.Database
import org.ktorm.dsl.*
import org.ktorm.entity.*
import org.ktorm.schema.*

/**
 * Ktorm 实验端口
 *
 * 基于 Ktorm 实现的实验管理端口，提供实验创建、更新、任务分配、
 * 结果记录和分析功能。支持 A/B 测试和统计分析。
 *
 * 表结构：
 * - remote_solver_experiment: 存储实验定义
 * - remote_solver_experiment_outcome: 存储任务执行结果
 * - remote_solver_task_experiment_assignment: 存储任务与实验变体的分配关系
 *
 * Ktorm Experiment Port
 *
 * Experiment management port implemented with Ktorm, providing experiment creation,
 * update, task assignment, result recording, and analysis functionality.
 * Supports A/B testing and statistical analysis.
 *
 * Table schemas:
 * - remote_solver_experiment: stores experiment definitions
 * - remote_solver_experiment_outcome: stores task execution outcomes
 * - remote_solver_task_experiment_assignment: stores task-to-variant assignments
 *
 * @param database Ktorm 数据库实例
 * @param json JSON 序列化配置
 */
class KtormExperimentPort(
    private val database: Database,
    private val json: Json = Json { ignoreUnknownKeys = true; encodeDefaults = true }
) : ExperimentPort {

    /**
     * 实验表定义
     *
     * 定义存储实验定义的数据库表结构。
     *
     * Experiment Table Definition
     *
     * Defines the database table structure for storing experiment definitions.
     */
    object ExperimentTable : Table<ExperimentRecord>("remote_solver_experiment") {
        val id = varchar("id").primaryKey().bindTo { it.id }
        val name = varchar("name").bindTo { it.name }
        val description = varchar("description").bindTo { it.description }
        val status = varchar("status").bindTo { it.status }
        val variantsJson = varchar("variants_json").bindTo { it.variantsJson }
        val configJson = varchar("config_json").bindTo { it.configJson }
        val metricsJson = varchar("metrics_json").bindTo { it.metricsJson }
        val createdAtEpochMs = long("created_at_epoch_ms").bindTo { it.createdAtEpochMs }
        val startedAtEpochMs = long("started_at_epoch_ms").bindTo { it.startedAtEpochMs }
        val endedAtEpochMs = long("ended_at_epoch_ms").bindTo { it.endedAtEpochMs }
    }

    /**
     * 实验结果表定义
     *
     * 定义存储任务执行结果的数据库表结构。
     *
     * Experiment Outcome Table Definition
     *
     * Defines the database table structure for storing task execution outcomes.
     */
    object ExperimentOutcomeTable : Table<ExperimentOutcomeRecord>("remote_solver_experiment_outcome") {
        val id = long("id").primaryKey()
        val experimentId = varchar("experiment_id")
        val variantId = varchar("variant_id")
        val taskId = varchar("task_id")
        val success = boolean("success")
        val runtimeMs = long("runtime_ms")
        val cost = double("cost")
        val timestampEpochMs = long("timestamp_epoch_ms")
    }

    /**
     * 任务分配表定义
     *
     * 定义存储任务与实验变体分配关系的数据库表结构。
     *
     * Task Assignment Table Definition
     *
     * Defines the database table structure for storing task-to-variant assignments.
     */
    object TaskAssignmentTable : Table<TaskAssignmentRecord>("remote_solver_task_experiment_assignment") {
        val id = long("id").primaryKey()
        val taskId = varchar("task_id").primaryKey()
        val experimentId = varchar("experiment_id")
        val variantId = varchar("variant_id")
        val assignedAtEpochMs = long("assigned_at_epoch_ms")
    }

    /**
     * 实验记录接口
     *
     * 定义实验表记录的实体接口。
     *
     * Experiment Record Interface
     *
     * Defines the entity interface for experiment table records.
     */
    interface ExperimentRecord : Entity<ExperimentRecord> {
        companion object : Entity.Factory<ExperimentRecord>()
        val id: String
        val name: String
        val description: String
        val status: String
        val variantsJson: String
        val configJson: String
        val metricsJson: String
        val createdAtEpochMs: Long
        val startedAtEpochMs: Long?
        val endedAtEpochMs: Long?
    }

    /**
     * 实验结果记录接口
     *
     * 定义实验结果表记录的实体接口。
     *
     * Experiment Outcome Record Interface
     *
     * Defines the entity interface for experiment outcome table records.
     */
    interface ExperimentOutcomeRecord : Entity<ExperimentOutcomeRecord> {
        companion object : Entity.Factory<ExperimentOutcomeRecord>()
        val id: Long
        val experimentId: String
        val variantId: String
        val taskId: String
        val success: Boolean
        val runtimeMs: Long
        val cost: Double
        val timestampEpochMs: Long
    }

    /**
     * 任务分配记录接口
     *
     * 定义任务分配表记录的实体接口。
     *
     * Task Assignment Record Interface
     *
     * Defines the entity interface for task assignment table records.
     */
    interface TaskAssignmentRecord : Entity<TaskAssignmentRecord> {
        companion object : Entity.Factory<TaskAssignmentRecord>()
        val id: Long
        val taskId: String
        val experimentId: String
        val variantId: String
        val assignedAtEpochMs: Long
    }

    private val experimentSequence = database.sequenceOf(ExperimentTable)
    private val outcomeSequence = database.sequenceOf(ExperimentOutcomeTable)
    private val assignmentSequence = database.sequenceOf(TaskAssignmentTable)

    /**
     * 获取活跃实验列表
     *
     * 获取所有正在运行的实验列表。
     *
     * @return 活跃实验列表
     *
     * Gets active experiments
     *
     * Retrieves all experiments that are currently running.
     *
     * @return List of active experiments
     */
    override suspend fun getActiveExperiments(): List<Experiment> {
        return experimentSequence
            .filter { it.status eq ExperimentStatus.RUNNING.name }
            .map { parseExperiment(it) }
            .toList()
    }

    /**
     * 获取单个实验
     *
     * 根据 ID 获取指定的实验信息。
     *
     * @param experimentId 实验 ID
     * @return 实验信息，如果不存在则返回 null
     *
     * Gets single experiment
     *
     * Retrieves the specified experiment information by ID.
     *
     * @param experimentId Experiment ID
     * @return Experiment information, or null if not found
     */
    override suspend fun getExperiment(experimentId: String): Experiment? {
        return experimentSequence
            .find { it.id eq experimentId }
            ?.let { parseExperiment(it) }
    }

    /**
     * 创建实验
     *
     * 将新实验插入到数据库中。
     *
     * @param experiment 实验信息
     *
     * Creates experiment
     *
     * Inserts a new experiment into the database.
     *
     * @param experiment Experiment information
     */
    override suspend fun createExperiment(experiment: Experiment) {
        database.insert(ExperimentTable) {
            set(it.id, experiment.experimentId)
            set(it.name, experiment.name)
            set(it.description, experiment.description)
            set(it.status, experiment.status.name)
            set(it.variantsJson, json.encodeToString(experiment.variants))
            set(it.configJson, json.encodeToString(experiment.config))
            set(it.metricsJson, json.encodeToString(experiment.metrics))
            set(it.createdAtEpochMs, experiment.createdAtEpochMs)
            set(it.startedAtEpochMs, experiment.startedAtEpochMs)
            set(it.endedAtEpochMs, experiment.endedAtEpochMs)
        }
    }

    /**
     * 更新实验
     *
     * 更新数据库中的实验信息。
     *
     * @param experiment 实验信息
     *
     * Updates experiment
     *
     * Updates experiment information in the database.
     *
     * @param experiment Experiment information
     */
    override suspend fun updateExperiment(experiment: Experiment) {
        database.update(ExperimentTable) {
            where { it.id eq experiment.experimentId }
            set(it.name, experiment.name)
            set(it.description, experiment.description)
            set(it.status, experiment.status.name)
            set(it.variantsJson, json.encodeToString(experiment.variants))
            set(it.configJson, json.encodeToString(experiment.config))
            set(it.metricsJson, json.encodeToString(experiment.metrics))
            set(it.startedAtEpochMs, experiment.startedAtEpochMs)
            set(it.endedAtEpochMs, experiment.endedAtEpochMs)
        }
    }

    /**
     * 获取任务的实验变体
     *
     * 获取任务所属的实验和分配的变体信息。
     *
     * @param taskId 任务 ID
     * @return 实验和变体的配对，如果任务未分配则返回 null
     *
     * Gets variant for task
     *
     * Retrieves the experiment and assigned variant for the task.
     *
     * @param taskId Task ID
     * @return Pair of experiment and variant, or null if task is not assigned
     */
    override suspend fun getVariantForTask(taskId: String): Pair<Experiment, ExperimentVariant>? {
        val assignment = assignmentSequence.find { it.taskId eq taskId }
        if (assignment == null) return null

        val experiment = getExperiment(assignment.experimentId)
        if (experiment == null) return null

        val variant = experiment.variants.find { it.id == assignment.variantId }
        if (variant == null) return null

        return Pair(experiment, variant)
    }

    /**
     * 记录实验结果
     *
     * 记录任务在实验中的执行结果，并更新实验统计指标。
     *
     * @param experimentId 实验 ID
     * @param variantId 变体 ID
     * @param taskId 任务 ID
     * @param success 是否成功
     * @param runtimeMs 运行时间（毫秒）
     * @param cost 成本
     *
     * Records outcome
     *
     * Records task execution outcome in the experiment and updates experiment metrics.
     *
     * @param experimentId Experiment ID
     * @param variantId Variant ID
     * @param taskId Task ID
     * @param success Whether succeeded
     * @param runtimeMs Runtime (milliseconds)
     * @param cost Cost
     */
    override suspend fun recordOutcome(
        experimentId: String,
        variantId: String,
        taskId: String,
        success: Boolean,
        runtimeMs: Long,
        cost: Double
    ) {
        database.insert(ExperimentOutcomeTable) {
            set(it.experimentId, experimentId)
            set(it.variantId, variantId)
            set(it.taskId, taskId)
            set(it.success, success)
            set(it.runtimeMs, runtimeMs)
            set(it.cost, cost)
            set(it.timestampEpochMs, System.currentTimeMillis())
        }

        // Update experiment metrics
        // 更新实验统计指标
        val experiment = getExperiment(experimentId)
        if (experiment != null) {
            val updatedVariants = experiment.variants.map { variant ->
                if (variant.id == variantId) {
                    variant.copy(metrics = variant.metrics.record(success, runtimeMs, cost))
                } else {
                    variant
                }
            }
            val updatedExperiment = experiment.copy(
                variants = updatedVariants,
                metrics = experiment.metrics.record()
            )
            updateExperiment(updatedExperiment)
        }
    }

    /**
     * 获取实验结果分析
     *
     * 计算并返回实验的统计分析结果，包括各变体的性能指标、
     * 相对对照组的改进百分比和统计显著性检验。
     *
     * @param experimentId 实验 ID
     * @return 实验结果分析，如果实验不存在则返回 null
     *
     * Gets experiment results
     *
     * Calculates and returns statistical analysis results for the experiment,
     * including performance metrics for each variant, improvement percentage
     * relative to control group, and statistical significance tests.
     *
     * @param experimentId Experiment ID
     * @return Experiment results analysis, or null if experiment not found
     */
    override suspend fun getExperimentResults(experimentId: String): ExperimentResults? {
        val experiment = getExperiment(experimentId)
        if (experiment == null) return null

        val controlVariant = experiment.variants.find { it.isControl }
        val variantResults = experiment.variants.map { variant ->
            val improvement = if (controlVariant != null && !variant.isControl) {
                val controlValue = controlVariant.metrics.getMetricValue(experiment.config.primaryMetric)
                val variantValue = variant.metrics.getMetricValue(experiment.config.primaryMetric)
                val higherIsBetter = experiment.config.primaryMetric.higherIsBetter
                if (controlValue != 0.0) {
                    val delta = variantValue - controlValue
                    (if (higherIsBetter) delta else -delta) / controlValue * 100.0
                } else null
            } else null

            val pValue = if (controlVariant != null && !variant.isControl) {
                calculatePValue(controlVariant.metrics, variant.metrics)
            } else null

            VariantResult(
                variantId = variant.id,
                variantName = variant.name,
                isControl = variant.isControl,
                sampleCount = variant.metrics.sampleCount,
                successRate = variant.metrics.avgSuccessRate,
                avgRuntimeMs = variant.metrics.avgRuntimeMs,
                avgCost = variant.metrics.avgCost,
                improvementOverControl = improvement,
                pValue = pValue
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
     * 分配任务到实验变体
     *
     * 将任务分配到指定的实验变体，用于实验分组。
     *
     * @param taskId 任务 ID
     * @param experimentId 实验 ID
     * @param variantId 变体 ID
     *
     * Assigns task to variant
     *
     * Assigns a task to the specified experiment variant for experiment grouping.
     *
     * @param taskId Task ID
     * @param experimentId Experiment ID
     * @param variantId Variant ID
     */
    suspend fun assignTaskToVariant(taskId: String, experimentId: String, variantId: String) {
        database.insert(TaskAssignmentTable) {
            set(it.taskId, taskId)
            set(it.experimentId, experimentId)
            set(it.variantId, variantId)
            set(it.assignedAtEpochMs, System.currentTimeMillis())
        }
    }

    /**
     * 解析实验记录
     *
     * 将数据库记录解析为实验对象。
     *
     * @param record 数据库记录
     * @return 实验对象
     *
     * Parses experiment record
     *
     * Parses database record to experiment object.
     *
     * @param record Database record
     * @return Experiment object
     */
    private fun parseExperiment(record: ExperimentRecord): Experiment {
        val variants = json.decodeFromString<List<ExperimentVariant>>(record.variantsJson)
        val config = json.decodeFromString<ExperimentConfig>(record.configJson)
        val metrics = json.decodeFromString<ExperimentMetrics>(record.metricsJson)
        val status = ExperimentStatus.valueOf(record.status)

        return Experiment(
            experimentId = record.id,
            name = record.name,
            description = record.description,
            status = status,
            variants = variants,
            config = config,
            metrics = metrics,
            createdAtEpochMs = record.createdAtEpochMs,
            startedAtEpochMs = record.startedAtEpochMs,
            endedAtEpochMs = record.endedAtEpochMs
        )
    }

    /**
     * 计算 P 值
     *
     * 使用双样本 t 检验计算对照组和实验组之间的统计显著性。
     *
     * @param control 对照组指标
     * @param treatment 实验组指标
     * @return P 值，如果样本不足则返回 null
     *
     * Calculates P-value
     *
     * Uses two-sample t-test to calculate statistical significance between
     * control and treatment groups.
     *
     * @param control Control group metrics
     * @param treatment Treatment group metrics
     * @return P-value, or null if sample size is insufficient
     */
    private fun calculatePValue(control: VariantMetrics, treatment: VariantMetrics): Double? {
        val n1 = control.sampleCount.toDouble()
        val n2 = treatment.sampleCount.toDouble()
        if (n1 < 2 || n2 < 2) return null

        val mean1 = control.avgSuccessRate
        val mean2 = treatment.avgSuccessRate
        val var1 = control.successRateVariance
        val var2 = treatment.successRateVariance

        val pooledSe = kotlin.math.sqrt(var1 / n1 + var2 / n2)
        if (pooledSe == 0.0) return null

        val tStat = (mean2 - mean1) / pooledSe
        return if (n1 + n2 > 30) {
            2.0 * (1.0 - normalCdf(kotlin.math.abs(tStat)))
        } else null
    }

    /**
     * 正态分布累积函数
     *
     * 计算标准正态分布的累积概率值。
     *
     * @param x 输入值
     * @return 累积概率值
     *
     * Normal cumulative distribution function
     *
     * Calculates the cumulative probability of standard normal distribution.
     *
     * @param x Input value
     * @return Cumulative probability
     */
    private fun normalCdf(x: Double): Double {
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

    /**
     * 创建数据库表
     *
     * 如果表不存在则创建实验相关的数据库表和索引。
     *
     * Creates database tables
     *
     * Creates experiment-related database tables and indexes if they don't exist.
     */
    fun createTables() {
        database.useConnection { conn ->
            conn.createStatement().execute("""
                CREATE TABLE IF NOT EXISTS remote_solver_experiment (
                    id VARCHAR(100) PRIMARY KEY,
                    name VARCHAR(200) NOT NULL,
                    description VARCHAR(500),
                    status VARCHAR(50) NOT NULL,
                    variants_json VARCHAR(2000) NOT NULL,
                    config_json VARCHAR(500) NOT NULL,
                    metrics_json VARCHAR(500) NOT NULL,
                    created_at_epoch_ms BIGINT NOT NULL,
                    started_at_epoch_ms BIGINT,
                    ended_at_epoch_ms BIGINT
                )
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE TABLE IF NOT EXISTS remote_solver_experiment_outcome (
                    id BIGSERIAL PRIMARY KEY,
                    experiment_id VARCHAR(100) NOT NULL,
                    variant_id VARCHAR(100) NOT NULL,
                    task_id VARCHAR(100) NOT NULL,
                    success BOOLEAN NOT NULL,
                    runtime_ms BIGINT NOT NULL,
                    cost DOUBLE PRECISION NOT NULL,
                    timestamp_epoch_ms BIGINT NOT NULL
                )
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE TABLE IF NOT EXISTS remote_solver_task_experiment_assignment (
                    id BIGSERIAL PRIMARY KEY,
                    task_id VARCHAR(100) PRIMARY KEY,
                    experiment_id VARCHAR(100) NOT NULL,
                    variant_id VARCHAR(100) NOT NULL,
                    assigned_at_epoch_ms BIGINT NOT NULL
                )
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE INDEX IF NOT EXISTS idx_experiment_outcome_exp_var
                ON remote_solver_experiment_outcome (experiment_id, variant_id)
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE INDEX IF NOT EXISTS idx_experiment_status
                ON remote_solver_experiment (status)
            """.trimIndent())
        }
    }
}