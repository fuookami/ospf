@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 调度器引擎
 *
 * 本模块提供任务调度决策引擎，
 * 负责根据任务特性和节点状态选择最优的执行节点。
 * 考虑因素包括：成本、截止日期风险、队列延迟等。
 *
 * Scheduler Engine
 *
 * This module provides task scheduling decision engine,
 * responsible for selecting optimal execution nodes based on task characteristics and node states.
 * Consideration factors include: cost, deadline risk, queue delay, etc.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import kotlin.math.max

/**
 * 调度器引擎
 *
 * 负责根据多维度评分选择最优执行节点。
 *
 * Scheduler engine.
 *
 * Responsible for selecting optimal execution nodes based on multi-dimensional scoring.
 *
 * @param clock 时钟端口，用于获取当前时间
 *              Clock port for getting current time
 * @param weights 调度权重配置
 *                Scheduler weights configuration
 */
class SchedulerEngine(
    private val clock: ClockPort,
    private val weights: SchedulerWeights = SchedulerWeights()
) {
    private data class NodeEvaluation(
        val node: NodeState,
        val etaSeconds: Double,
        val cost: Double,
        val deadlineRisk: Double,
        val queueDelay: Double,
        val weightedScore: Double
    )

    /**
     * 选择最优节点
     *
     * 根据任务特性和候选节点状态选择最优执行节点。
     * 对于有截止日期的任务，优先选择能满足截止日期的节点；
     * 对于无截止日期的任务，根据加权评分选择最优节点。
     *
     * Chooses optimal node.
     *
     * Selects optimal execution node based on task characteristics and candidate node states.
     * For tasks with deadlines, prioritizes nodes that can meet the deadline;
     * For tasks without deadlines, selects optimal node based on weighted scoring.
     *
     * @param task 待调度任务
     *             Task to be scheduled
     * @param candidates 候选节点列表
     *                   Candidate node list
     * @return 最优节点，如果没有可用节点则返回 null
     *         Optimal node, or null if no available nodes
     */
    fun chooseNode(task: TaskState, candidates: List<NodeState>): NodeState? {
        val nowEpochMs = clock.nowEpochMs()
        val evaluations = candidates
            .asSequence()
            .filter { it.online && it.availableUnits > 0 }
            .map { evaluate(task, it, nowEpochMs) }
            .toList()
        if (evaluations.isEmpty()) {
            return null
        }

        if (task.deadline != null) {
            val deadlineSatisfied = evaluations.filter { it.deadlineRisk <= 0.0 }
            if (deadlineSatisfied.isNotEmpty()) {
                return deadlineSatisfied
                    .minWithOrNull(
                        compareBy<NodeEvaluation> { it.cost }
                            .thenBy { it.deadlineRisk }
                            .thenBy { it.queueDelay }
                            .thenBy { it.node.nodeId.value }
                    )
                    ?.node
            }
            return evaluations
                .minWithOrNull(
                    compareBy<NodeEvaluation> { it.deadlineRisk }
                        .thenBy { it.cost }
                        .thenBy { it.queueDelay }
                        .thenBy { it.node.nodeId.value }
                )
                ?.node
        }

        return evaluations
            .minWithOrNull(
                compareBy<NodeEvaluation> { it.weightedScore }
                    .thenBy { it.cost }
                    .thenBy { it.node.nodeId.value }
            )
            ?.node
    }

    private fun evaluate(task: TaskState, node: NodeState, nowEpochMs: Long): NodeEvaluation {
        val etaSeconds = estimateEtaSeconds(task, node)
        val cost = etaSeconds * node.profile.pricePerSecond.toDouble() + node.profile.licenseCostPerSlice.toDouble()
        val deadlineRisk = deadlineRisk(task, etaSeconds, nowEpochMs)
        val queueDelay = queueDelaySeconds(node, etaSeconds)
        val weightedScore = weights.costWeight * cost +
            weights.deadlineRiskWeight * deadlineRisk +
            weights.queueDelayWeight * queueDelay
        return NodeEvaluation(
            node = node,
            etaSeconds = etaSeconds,
            cost = cost,
            deadlineRisk = deadlineRisk,
            queueDelay = queueDelay,
            weightedScore = weightedScore
        )
    }

    private fun estimateEtaSeconds(task: TaskState, node: NodeState): Double {
        val base = when (task.complexity) {
            TaskComplexity.SIMPLE -> 20.0
            TaskComplexity.COMPLEX -> 90.0
        }
        val realtimeFactor = when (task.timeSensitivity) {
            TimeSensitivity.REALTIME -> 0.75
            TimeSensitivity.NON_REALTIME -> 1.0
        }
        val performanceAdjusted = base / max(0.1, node.profile.performanceScore.toDouble())
        return max(
            node.profile.minBillingUnitSeconds.toDouble(),
            performanceAdjusted * realtimeFactor
        )
    }

    private fun queueDelaySeconds(node: NodeState, etaSeconds: Double): Double {
        val parallelUnits = max(1, node.profile.parallelUnits)
        val safeAvailableUnits = node.availableUnits.coerceIn(0, parallelUnits)
        val occupiedUnits = parallelUnits - safeAvailableUnits
        if (occupiedUnits <= 0) {
            return 0.0
        }
        val occupancyRatio = occupiedUnits.toDouble() / parallelUnits.toDouble()
        return etaSeconds * occupancyRatio
    }

    private fun deadlineRisk(task: TaskState, etaSeconds: Double, nowEpochMs: Long): Double {
        val deadlineEpochMs = task.deadline?.toEpochMilliseconds() ?: return 0.0
        val predictedFinish = nowEpochMs + (etaSeconds * 1000.0).toLong()
        if (predictedFinish <= deadlineEpochMs) {
            return 0.0
        }
        val window = max(1L, deadlineEpochMs - nowEpochMs).toDouble()
        return (predictedFinish - deadlineEpochMs) / window
    }
}
