/*
 * 调度器热更新模型
 *
 * 本模块定义调度器配置热更新相关的数据模型，
 * 包括运行时配置、审计记录等。
 *
 * Scheduler Hot Reload Models
 *
 * This module defines data models related to scheduler configuration hot reload,
 * including runtime configuration and audit records.
 */
package fuookami.ospf.framework.remote_solver.application

/**
 * 调度器运行时配置
 *
 * 用于热更新的调度器运行时配置参数集合。
 *
 * Scheduler runtime configuration.
 *
 * Collection of scheduler runtime configuration parameters for hot reload.
 *
 * @param simpleTaskQuantumMs 简单任务的时间片长度（毫秒）
 *                            Simple task quantum length in milliseconds
 * @param complexTaskQuantumMs 复杂任务的时间片长度（毫秒）
 *                             Complex task quantum length in milliseconds
 * @param complexTaskQuantumMinMs 复杂任务时间片最小值（毫秒）
 *                                Complex task quantum minimum in milliseconds
 * @param complexTaskQuantumMaxMs 复杂任务时间片最大值（毫秒）
 *                                Complex task quantum maximum in milliseconds
 * @param complexSolveEstimateMs 复杂任务求解估计时间（毫秒）
 *                               Complex task solve estimate in milliseconds
 * @param complexCheckpointEstimateMs 复杂任务检查点估计时间（毫秒）
 *                                    Complex task checkpoint estimate in milliseconds
 * @param complexQuantumAlpha 复杂任务时间片计算的 alpha 系数
 *                             Complex task quantum calculation alpha coefficient
 * @param complexQuantumBeta 复杂任务时间片计算的 beta 系数
 *                           Complex task quantum calculation beta coefficient
 * @param complexQuantumPricePenalty 复杂任务时间片的价格惩罚系数
 *                                   Complex task quantum price penalty coefficient
 * @param complexUrgencyWeight 复杂任务紧急度权重
 *                             Complex task urgency weight
 * @param complexWaitingAgeWeight 复杂任务等待年龄权重
 *                                 Complex task waiting age weight
 * @param complexProgressNeedWeight 复杂任务进度需求权重
 *                                   Complex task progress need weight
 * @param complexCostSensitivityWeight 复杂任务成本敏感度权重
 *                                      Complex task cost sensitivity weight
 */
data class SchedulerRuntimeConfig(
    val simpleTaskQuantumMs: Long,
    val complexTaskQuantumMs: Long,
    val complexTaskQuantumMinMs: Long,
    val complexTaskQuantumMaxMs: Long,
    val complexSolveEstimateMs: Long,
    val complexCheckpointEstimateMs: Long,
    val complexQuantumAlpha: Double,
    val complexQuantumBeta: Double,
    val complexQuantumPricePenalty: Double,
    val complexUrgencyWeight: Double,
    val complexWaitingAgeWeight: Double,
    val complexProgressNeedWeight: Double,
    val complexCostSensitivityWeight: Double
) {
    /**
     * 将运行时配置转换为变更集
     *
     * Converts runtime configuration to a change set.
     *
     * @return 键值对形式的配置变更集，键为配置参数名，值为参数值字符串
     *         Configuration change set as key-value pairs, where keys are parameter names and values are parameter value strings
     */
    fun toChangeSet(): Map<String, String> =
        linkedMapOf(
            "scheduler.simple-task-quantum-ms" to simpleTaskQuantumMs.toString(),
            "scheduler.complex-task-quantum-ms" to complexTaskQuantumMs.toString(),
            "scheduler.complex-task-quantum-min-ms" to complexTaskQuantumMinMs.toString(),
            "scheduler.complex-task-quantum-max-ms" to complexTaskQuantumMaxMs.toString(),
            "scheduler.complex-solve-estimate-ms" to complexSolveEstimateMs.toString(),
            "scheduler.complex-checkpoint-estimate-ms" to complexCheckpointEstimateMs.toString(),
            "scheduler.complex-quantum-alpha" to complexQuantumAlpha.toString(),
            "scheduler.complex-quantum-beta" to complexQuantumBeta.toString(),
            "scheduler.complex-quantum-price-penalty" to complexQuantumPricePenalty.toString(),
            "scheduler.complex-urgency-weight" to complexUrgencyWeight.toString(),
            "scheduler.complex-waiting-age-weight" to complexWaitingAgeWeight.toString(),
            "scheduler.complex-progress-need-weight" to complexProgressNeedWeight.toString(),
            "scheduler.complex-cost-sensitivity-weight" to complexCostSensitivityWeight.toString()
        )

    companion object {
        /**
         * 从基础配置创建运行时配置
         *
         * Creates runtime configuration from base configuration.
         *
         * @param base 基础配置
         *             Base configuration
         * @return 新创建的运行时配置
         *         Newly created runtime configuration
         */
        fun from(base: RemoteSolverConfig): SchedulerRuntimeConfig =
            SchedulerRuntimeConfig(
                simpleTaskQuantumMs = base.simpleTaskQuantumMs,
                complexTaskQuantumMs = base.complexTaskQuantumMs,
                complexTaskQuantumMinMs = base.complexTaskQuantumMinMs,
                complexTaskQuantumMaxMs = base.complexTaskQuantumMaxMs,
                complexSolveEstimateMs = base.complexSolveEstimateMs,
                complexCheckpointEstimateMs = base.complexCheckpointEstimateMs,
                complexQuantumAlpha = base.complexQuantumAlpha,
                complexQuantumBeta = base.complexQuantumBeta,
                complexQuantumPricePenalty = base.complexQuantumPricePenalty,
                complexUrgencyWeight = base.complexUrgencyWeight,
                complexWaitingAgeWeight = base.complexWaitingAgeWeight,
                complexProgressNeedWeight = base.complexProgressNeedWeight,
                complexCostSensitivityWeight = base.complexCostSensitivityWeight
            )
    }
}

/**
 * 调度器热更新审计记录
 *
 * 记录每次热更新操作的审计信息。
 *
 * Scheduler hot reload audit record.
 *
 * Records audit information for each hot reload operation.
 *
 * @param version 新版本号
 *                New version number
 * @param previousVersion 前一个版本号
 *                        Previous version number
 * @param operator 操作者标识
 *                 Operator identifier
 * @param effectiveAtEpochMs 生效时间戳（毫秒）
 *                           Effective timestamp in milliseconds
 * @param changeSet 配置变更集
 *                  Configuration change set
 * @param rollbackFromVersion 回滚来源版本号，如果此次更新是回滚操作
 *                             Rollback source version, if this update is a rollback operation
 */
data class SchedulerHotReloadAuditRecord(
    val version: String,
    val previousVersion: String,
    val operator: String,
    val effectiveAtEpochMs: Long,
    val changeSet: Map<String, String>,
    val rollbackFromVersion: String? = null
)
