/*
 * 远程求解器配置
 *
 * 本模块定义远程求解器的核心配置参数，
 * 包括调度器配置、任务时间片设置、性能学习参数等。
 *
 * Remote Solver Configuration
 *
 * This module defines the core configuration parameters for the remote solver,
 * including scheduler configuration, task quantum settings, and performance learning parameters.
 */
package fuookami.ospf.framework.remote_solver.application

/**
 * 远程求解器配置
 *
 * 包含调度器和任务处理的所有可配置参数。
 *
 * Remote solver configuration.
 *
 * Contains all configurable parameters for scheduler and task processing.
 *
 * @param schedulerConfigVersion 调度器配置版本号，默认为 "v0"
 *                               Scheduler configuration version, defaults to "v0"
 * @param schedulerHotReloadEnabled 是否启用调度器热更新，默认为 false
 *                                  Whether scheduler hot reload is enabled, defaults to false
 * @param simpleTaskQuantumMs 简单任务的时间片长度（毫秒），默认为 2000ms
 *                            Simple task quantum length in milliseconds, defaults to 2000ms
 * @param complexTaskQuantumMs 复杂任务的时间片长度（毫秒），默认为 4000ms
 *                             Complex task quantum length in milliseconds, defaults to 4000ms
 * @param complexTaskQuantumMinMs 复杂任务时间片最小值（毫秒），默认为 2000ms
 *                                Complex task quantum minimum in milliseconds, defaults to 2000ms
 * @param complexTaskQuantumMaxMs 复杂任务时间片最大值（毫秒），默认为 10000ms
 *                                Complex task quantum maximum in milliseconds, defaults to 10000ms
 * @param complexSolveEstimateMs 复杂任务求解估计时间（毫秒），默认为 12000ms
 *                               Complex task solve estimate in milliseconds, defaults to 12000ms
 * @param complexCheckpointEstimateMs 复杂任务检查点估计时间（毫秒），默认为 800ms
 *                                    Complex task checkpoint estimate in milliseconds, defaults to 800ms
 * @param complexQuantumAlpha 复杂任务时间片计算的 alpha 系数，默认为 0.7
 *                             Complex task quantum calculation alpha coefficient, defaults to 0.7
 * @param complexQuantumBeta 复杂任务时间片计算的 beta 系数，默认为 0.5
 *                           Complex task quantum calculation beta coefficient, defaults to 0.5
 * @param complexQuantumPricePenalty 复杂任务时间片的价格惩罚系数，默认为 0.1
 *                                   Complex task quantum price penalty coefficient, defaults to 0.1
 * @param maxSchedulingBatch 最大调度批量大小，默认为 64
 *                           Maximum scheduling batch size, defaults to 64
 * @param nodeHeartbeatTimeoutMs 节点心跳超时时间（毫秒），默认为 30000ms
 *                               Node heartbeat timeout in milliseconds, defaults to 30000ms
 * @param sliceTimeoutGraceMs 时间片超时宽限时间（毫秒），默认为 200ms
 *                            Slice timeout grace period in milliseconds, defaults to 200ms
 * @param dispatchLockTtlMs 分发锁 TTL 时间（毫秒），默认为 15000ms
 *                          Dispatch lock TTL in milliseconds, defaults to 15000ms
 * @param complexTaskVariableThreshold 复杂任务变量数阈值，默认为 5000
 *                                      Complex task variable threshold, defaults to 5000
 * @param complexTaskConstraintThreshold 复杂任务约束数阈值，默认为 5000
 *                                        Complex task constraint threshold, defaults to 5000
 * @param complexTaskHistoricalRuntimeThresholdMs 复杂任务历史运行时间阈值（毫秒），默认为 120000ms
 *                                                 Complex task historical runtime threshold in milliseconds, defaults to 120000ms
 * @param complexUrgencyWeight 复杂任务紧急度权重，默认为 0.4
 *                             Complex task urgency weight, defaults to 0.4
 * @param complexWaitingAgeWeight 复杂任务等待年龄权重，默认为 0.3
 *                                 Complex task waiting age weight, defaults to 0.3
 * @param complexProgressNeedWeight 复杂任务进度需求权重，默认为 0.2
 *                                   Complex task progress need weight, defaults to 0.2
 * @param complexCostSensitivityWeight 复杂任务成本敏感度权重，默认为 0.1
 *                                      Complex task cost sensitivity weight, defaults to 0.1
 * @param performanceLearningEnabled 是否启用性能学习，默认为 true
 *                                   Whether performance learning is enabled, defaults to true
 * @param performanceLearningRate 性能学习率，默认为 0.2
 *                                Performance learning rate, defaults to 0.2
 * @param performanceScoreMin 性能分数最小值，默认为 0.1
 *                            Performance score minimum, defaults to 0.1
 * @param performanceScoreMax 性能分数最大值，默认为 10.0
 *                            Performance score maximum, defaults to 10.0
 */
data class RemoteSolverConfig(
    val schedulerConfigVersion: String = "v0",
    val schedulerHotReloadEnabled: Boolean = false,
    val simpleTaskQuantumMs: Long = 2000L,
    val complexTaskQuantumMs: Long = 4000L,
    val complexTaskQuantumMinMs: Long = 2000L,
    val complexTaskQuantumMaxMs: Long = 10000L,
    val complexSolveEstimateMs: Long = 12000L,
    val complexCheckpointEstimateMs: Long = 800L,
    val complexQuantumAlpha: Double = 0.7,
    val complexQuantumBeta: Double = 0.5,
    val complexQuantumPricePenalty: Double = 0.1,
    val maxSchedulingBatch: Int = 64,
    val nodeHeartbeatTimeoutMs: Long = 30000L,
    val sliceTimeoutGraceMs: Long = 200L,
    val dispatchLockTtlMs: Long = 15000L,
    val complexTaskVariableThreshold: Int = 5000,
    val complexTaskConstraintThreshold: Int = 5000,
    val complexTaskHistoricalRuntimeThresholdMs: Long = 120000L,
    val complexUrgencyWeight: Double = 0.4,
    val complexWaitingAgeWeight: Double = 0.3,
    val complexProgressNeedWeight: Double = 0.2,
    val complexCostSensitivityWeight: Double = 0.1,
    val performanceLearningEnabled: Boolean = true,
    val performanceLearningRate: Double = 0.2,
    val performanceScoreMin: Double = 0.1,
    val performanceScoreMax: Double = 10.0
)

/**
 * 调度器权重配置
 *
 * 用于节点选择决策的权重参数。
 *
 * Scheduler weights configuration.
 *
 * Weight parameters for node selection decisions.
 *
 * @param costWeight 成本权重，默认为 0.6
 *                   Cost weight, defaults to 0.6
 * @param deadlineRiskWeight 截止日期风险权重，默认为 0.3
 *                           Deadline risk weight, defaults to 0.3
 * @param queueDelayWeight 队列延迟权重，默认为 0.1
 *                         Queue delay weight, defaults to 0.1
 */
data class SchedulerWeights(
    val costWeight: Double = 0.6,
    val deadlineRiskWeight: Double = 0.3,
    val queueDelayWeight: Double = 0.1
)
