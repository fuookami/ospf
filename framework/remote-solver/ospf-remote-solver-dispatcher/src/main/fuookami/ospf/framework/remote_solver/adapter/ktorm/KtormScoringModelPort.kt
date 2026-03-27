/**
 * 评分模型端口模块
 *
 * 本模块提供基于 Ktorm 的可学习评分模型管理功能，
 * 用于远程求解器调度器的节点评分和模型校准。
 *
 * Scoring Model Port Module
 *
 * This module provides learnable scoring model management functionality based on Ktorm,
 * used for node scoring and model calibration in the remote solver dispatcher.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.LearnableScoringModel
import fuookami.ospf.framework.remote_solver.domain.NodeScoreFeatures
import fuookami.ospf.framework.remote_solver.domain.ScoringFeedback
import fuookami.ospf.framework.remote_solver.domain.ScoringWeights
import fuookami.ospf.framework.remote_solver.domain.TaskOutcome
import fuookami.ospf.framework.remote_solver.domain.TrainingMetrics
import fuookami.ospf.framework.remote_solver.port.ScoringModelPort
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.ktorm.database.Database
import org.ktorm.dsl.*
import org.ktorm.entity.*
import org.ktorm.schema.*
import java.time.Instant

/**
 * Ktorm 评分模型端口
 *
 * 基于 Ktorm 实现的评分模型端口，提供模型获取、更新、
 * 反馈记录和校准功能。支持基于历史数据的模型学习和优化。
 *
 * 表结构：
 * - remote_solver_scoring_model: 存储当前评分模型
 * - remote_solver_scoring_feedback: 存储用于校准的反馈数据
 *
 * Ktorm Scoring Model Port
 *
 * Scoring model port implemented with Ktorm, providing model retrieval,
 * update, feedback recording, and calibration functionality.
 * Supports model learning and optimization based on historical data.
 *
 * Table schemas:
 * - remote_solver_scoring_model: stores the current scoring model
 * - remote_solver_scoring_feedback: stores feedback for calibration
 *
 * @param database Ktorm 数据库实例
 * @param json JSON 序列化配置
 */
class KtormScoringModelPort(
    private val database: Database,
    private val json: Json = Json { ignoreUnknownKeys = true; encodeDefaults = true }
) : ScoringModelPort {

    /**
     * 评分模型表定义
     *
     * 定义存储评分模型的数据库表结构。
     *
     * Scoring Model Table Definition
     *
     * Defines the database table structure for storing scoring models.
     */
    object ScoringModelTable : Table<ScoringModelRecord>("remote_solver_scoring_model") {
        val id = int("id").primaryKey().bindTo { it.id }
        val version = varchar("version").bindTo { it.version }
        val weightsJson = varchar("weights_json").bindTo { it.weightsJson }
        val featureImportanceJson = varchar("feature_importance_json").bindTo { it.featureImportanceJson }
        val trainingMetricsJson = varchar("training_metrics_json").bindTo { it.trainingMetricsJson }
        val updatedAtEpochMs = long("updated_at_epoch_ms").bindTo { it.updatedAtEpochMs }
        val isActive = boolean("is_active").bindTo { it.isActive }
    }

    /**
     * 评分反馈表定义
     *
     * 定义存储评分反馈的数据库表结构。
     *
     * Scoring Feedback Table Definition
     *
     * Defines the database table structure for storing scoring feedback.
     */
    object ScoringFeedbackTable : Table<ScoringFeedbackRecord>("remote_solver_scoring_feedback") {
        val id = long("id").primaryKey().bindTo { it.id }
        val taskId = varchar("task_id").bindTo { it.taskId }
        val nodeId = varchar("node_id").bindTo { it.nodeId }
        val featuresJson = varchar("features_json").bindTo { it.featuresJson }
        val predictedScore = double("predicted_score").bindTo { it.predictedScore }
        val actualScore = double("actual_score").bindTo { it.actualScore }
        val outcome = varchar("outcome").bindTo { it.outcome }
        val timestampEpochMs = long("timestamp_epoch_ms").bindTo { it.timestampEpochMs }
        val processed = boolean("processed").bindTo { it.processed }
    }

    /**
     * 评分模型记录接口
     *
     * 定义评分模型表记录的实体接口。
     *
     * Scoring Model Record Interface
     *
     * Defines the entity interface for scoring model table records.
     */
    interface ScoringModelRecord : Entity<ScoringModelRecord> {
        companion object : Entity.Factory<ScoringModelRecord>()
        val id: Int
        val version: String
        val weightsJson: String
        val featureImportanceJson: String?
        val trainingMetricsJson: String?
        val updatedAtEpochMs: Long
        val isActive: Boolean
    }

    /**
     * 评分反馈记录接口
     *
     * 定义评分反馈表记录的实体接口。
     *
     * Scoring Feedback Record Interface
     *
     * Defines the entity interface for scoring feedback table records.
     */
    interface ScoringFeedbackRecord : Entity<ScoringFeedbackRecord> {
        companion object : Entity.Factory<ScoringFeedbackRecord>()
        val id: Long
        val taskId: String
        val nodeId: String
        val featuresJson: String
        val predictedScore: Double
        val actualScore: Double
        val outcome: String
        val timestampEpochMs: Long
        val processed: Boolean
    }

    private val modelSequence = database.sequenceOf(ScoringModelTable)
    private val feedbackSequence = database.sequenceOf(ScoringFeedbackTable)

    /**
     * 获取当前模型
     *
     * 获取当前激活的评分模型，如果没有激活模型则返回默认模型。
     *
     * @return 当前评分模型
     *
     * Gets current model
     *
     * Retrieves the currently active scoring model.
     * Returns default model if no active model exists.
     *
     * @return Current scoring model
     */
    override suspend fun getCurrentModel(): LearnableScoringModel {
        val activeModel = modelSequence
            .filter { it.isActive eq true }
            .firstOrNull()

        return if (activeModel != null) {
            parseModel(activeModel)
        } else {
            LearnableScoringModel.DEFAULT
        }
    }

    /**
     * 更新模型
     *
     * 更新评分模型，将现有模型标记为非激活并插入新的激活模型。
     *
     * @param model 新的评分模型
     *
     * Updates model
     *
     * Updates the scoring model by marking existing models as inactive
     * and inserting a new active model.
     *
     * @param model New scoring model
     */
    override suspend fun updateModel(model: LearnableScoringModel) {
        // Deactivate all existing models
        // 将所有现有模型标记为非激活
        database.update(ScoringModelTable) {
            where { it.isActive eq true }
            set(it.isActive, false)
        }

        // Insert new active model
        // 插入新的激活模型
        database.insert(ScoringModelTable) {
            set(it.version, model.version)
            set(it.weightsJson, json.encodeToString(model.weights))
            set(it.featureImportanceJson, if (model.featureImportance.isNotEmpty()) {
                json.encodeToString(model.featureImportance)
            } else null)
            set(it.trainingMetricsJson, if (model.trainingMetrics != null) {
                json.encodeToString(model.trainingMetrics)
            } else null)
            set(it.updatedAtEpochMs, model.updatedAtEpochMs)
            set(it.isActive, true)
        }
    }

    /**
     * 记录反馈
     *
     * 将评分反馈数据插入到数据库中，用于后续的模型校准。
     *
     * @param feedback 评分反馈数据
     *
     * Records feedback
     *
     * Inserts scoring feedback data into the database for later model calibration.
     *
     * @param feedback Scoring feedback data
     */
    override suspend fun recordFeedback(feedback: ScoringFeedback) {
        database.insert(ScoringFeedbackTable) {
            set(it.taskId, feedback.taskId)
            set(it.nodeId, feedback.nodeId)
            set(it.featuresJson, json.encodeToString(feedback.features))
            set(it.predictedScore, feedback.predictedScore)
            set(it.actualScore, feedback.actualScore)
            set(it.outcome, feedback.outcome.name)
            set(it.timestampEpochMs, feedback.timestampEpochMs)
            set(it.processed, false)
        }
    }

    /**
     * 获取待处理反馈
     *
     * 获取未处理的评分反馈数据，按时间戳排序。
     *
     * @param limit 最大返回数量
     * @return 待处理反馈列表
     *
     * Gets pending feedback
     *
     * Retrieves unprocessed scoring feedback data, sorted by timestamp.
     *
     * @param limit Maximum number to return
     * @return List of pending feedback
     */
    override suspend fun getPendingFeedback(limit: Int): List<ScoringFeedback> {
        return feedbackSequence
            .filter { it.processed eq false }
            .sortedBy { it.timestampEpochMs }
            .take(limit)
            .map { parseFeedback(it) }
            .toList()
    }

    /**
     * 清理反馈
     *
     * 将指定时间之前的反馈标记为已处理，并删除过期的已处理反馈。
     *
     * @param olderThanEpochMs 清理截止时间戳（毫秒）
     *
     * Clears feedback
     *
     * Marks feedback before the specified time as processed,
     * and deletes old processed feedback.
     *
     * @param olderThanEpochMs Clear cutoff timestamp (milliseconds)
     */
    override suspend fun clearFeedback(olderThanEpochMs: Long) {
        // Mark feedback as processed
        // 将反馈标记为已处理
        database.update(ScoringFeedbackTable) {
            where {
                it.timestampEpochMs lte olderThanEpochMs and (it.processed eq false)
            }
            set(it.processed, true)
        }

        // Delete old processed feedback (older than 30 days)
        // 删除过期的已处理反馈（超过 30 天）
        val cutoffEpochMs = olderThanEpochMs - (30L * 24 * 60 * 60 * 1000)
        database.delete(ScoringFeedbackTable) {
            it.timestampEpochMs lte cutoffEpochMs and (it.processed eq true)
        }
    }

    /**
     * 解析模型记录
     *
     * 将数据库记录解析为评分模型对象。
     *
     * @param record 数据库记录
     * @return 评分模型对象
     *
     * Parses model record
     *
     * Parses database record to scoring model object.
     *
     * @param record Database record
     * @return Scoring model object
     */
    private fun parseModel(record: ScoringModelRecord): LearnableScoringModel {
        val weights = json.decodeFromString<ScoringWeights>(record.weightsJson)
        val featureImportance = record.featureImportanceJson?.let {
            json.decodeFromString<Map<String, Double>>(it)
        } ?: emptyMap()
        val trainingMetrics = record.trainingMetricsJson?.let {
            json.decodeFromString<TrainingMetrics>(it)
        }

        return LearnableScoringModel(
            version = record.version,
            weights = weights,
            featureImportance = featureImportance,
            trainingMetrics = trainingMetrics,
            updatedAtEpochMs = record.updatedAtEpochMs
        )
    }

    /**
     * 解析反馈记录
     *
     * 将数据库记录解析为评分反馈对象。
     *
     * @param record 数据库记录
     * @return 评分反馈对象
     *
     * Parses feedback record
     *
     * Parses database record to scoring feedback object.
     *
     * @param record Database record
     * @return Scoring feedback object
     */
    private fun parseFeedback(record: ScoringFeedbackRecord): ScoringFeedback {
        val features = json.decodeFromString<NodeScoreFeatures>(record.featuresJson)
        val outcome = TaskOutcome.valueOf(record.outcome)

        return ScoringFeedback(
            taskId = record.taskId,
            nodeId = record.nodeId,
            features = features,
            predictedScore = record.predictedScore,
            actualScore = record.actualScore,
            outcome = outcome,
            timestampEpochMs = record.timestampEpochMs
        )
    }

    /**
     * 创建数据库表
     *
     * 如果表不存在则创建评分模型相关的数据库表和索引。
     *
     * Creates the database tables if they don't exist.
     *
     * Creates scoring model-related database tables and indexes if they don't exist.
     */
    fun createTables() {
        database.useConnection { conn ->
            conn.createStatement().execute("""
                CREATE TABLE IF NOT EXISTS remote_solver_scoring_model (
                    id SERIAL PRIMARY KEY,
                    version VARCHAR(100) NOT NULL,
                    weights_json VARCHAR(500) NOT NULL,
                    feature_importance_json VARCHAR(1000),
                    training_metrics_json VARCHAR(1000),
                    updated_at_epoch_ms BIGINT NOT NULL,
                    is_active BOOLEAN NOT NULL DEFAULT true
                )
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE TABLE IF NOT EXISTS remote_solver_scoring_feedback (
                    id BIGSERIAL PRIMARY KEY,
                    task_id VARCHAR(100) NOT NULL,
                    node_id VARCHAR(100) NOT NULL,
                    features_json VARCHAR(1000) NOT NULL,
                    predicted_score DOUBLE PRECISION NOT NULL,
                    actual_score DOUBLE PRECISION NOT NULL,
                    outcome VARCHAR(50) NOT NULL,
                    timestamp_epoch_ms BIGINT NOT NULL,
                    processed BOOLEAN NOT NULL DEFAULT false
                )
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE INDEX IF NOT EXISTS idx_scoring_feedback_pending
                ON remote_solver_scoring_feedback (processed, timestamp_epoch_ms)
            """.trimIndent())

            conn.createStatement().execute("""
                CREATE INDEX IF NOT EXISTS idx_scoring_feedback_timestamp
                ON remote_solver_scoring_feedback (timestamp_epoch_ms)
            """.trimIndent())
        }
    }
}