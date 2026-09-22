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
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import kotlin.math.max
import kotlin.math.roundToLong
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicReference

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
    weights: SchedulerWeights = SchedulerWeights()
) {
    private data class NodeEvaluation(
        val node: NodeState,
        val etaSeconds: Double,
        val cost: Double,
        val deadlineRisk: Double,
        val queueDelay: Double,
        val weightedScore: Double,
        val migrationCost: Double,
        val progressNeed: Double,
        val admissionReasons: List<AdmissionReasonCode> = emptyList()
    )

    /** Current weights for smooth weighted round robin, keyed by candidate set. */
    private val roundRobinCurrentWeights = ConcurrentHashMap<String, MutableMap<String, Long>>()
    /** Runtime scoring weights; replaced atomically by scheduler hot reload. */
    private val weightsRef = AtomicReference(weights)

    /** Returns the currently active scheduler scoring weights. */
    fun schedulerWeights(): SchedulerWeights = weightsRef.get()

    /** Applies a complete scoring-weight snapshot atomically. */
    fun updateWeights(weights: SchedulerWeights) {
        weightsRef.set(weights)
    }

    private val currentWeights: SchedulerWeights
        get() = weightsRef.get()

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
    fun chooseNode(task: TaskState, candidates: List<NodeState>): NodeState? =
        explainNodeSelection(task, candidates).selectedNode

    /**
     * Chooses a node while keeping a suspended task on its current node until
     * a materially better candidate is available. The overload preserves the
     * original stateless API for callers that do not track a preferred node.
     */
    fun chooseNode(
        task: TaskState,
        candidates: List<NodeState>,
        preferredNodeId: fuookami.ospf.framework.remote_solver.protocol.domain.NodeId?,
        migrationHysteresisRatio: Double = currentWeights.migrationHysteresisRatio,
        minSlicesBeforeMigration: Int = 0,
        migrationCostWeight: Double = 0.0,
        slicesExecuted: Int = 0,
        progressSignal: ProgressSignal? = null
    ): NodeState? = explainNodeSelection(
        task = task,
        candidates = candidates,
        preferredNodeId = preferredNodeId,
        migrationHysteresisRatio = migrationHysteresisRatio,
        minSlicesBeforeMigration = minSlicesBeforeMigration,
        migrationCostWeight = migrationCostWeight,
        slicesExecuted = slicesExecuted,
        progressSignal = progressSignal
    ).selectedNode

    /**
     * Returns the complete admission and scoring explanation for a decision.
     * The candidate order and tie breakers are deterministic so the same
     * snapshot produces the same decision apart from the intentional RR cursor.
     */
    fun explainNodeSelection(
        task: TaskState,
        candidates: List<NodeState>,
        preferredNodeId: fuookami.ospf.framework.remote_solver.protocol.domain.NodeId? = null,
        migrationHysteresisRatio: Double = currentWeights.migrationHysteresisRatio,
        minSlicesBeforeMigration: Int = 0,
        migrationCostWeight: Double = 0.0,
        slicesExecuted: Int = 0,
        progressSignal: ProgressSignal? = null
    ): NodeSelectionDecision {
        val nowEpochMs = clock.nowEpochMs()
        val evaluations = candidates
            .distinctBy { it.nodeId.value }
            .map {
                evaluate(
                    task = task,
                    node = it,
                    nowEpochMs = nowEpochMs,
                    preferredNodeId = preferredNodeId,
                    migrationCostWeight = migrationCostWeight
                )
            }
        val eligible = evaluations.filter { it.admissionReasons.isEmpty() }
        if (eligible.isEmpty()) {
            return NodeSelectionDecision(
                selectedNode = null,
                reason = "no_eligible_node",
                candidates = evaluations.map { it.toCandidate() }
            )
        }

        val baseline = baselineSelection(task, eligible, progressSignal)
        val progressPreference = progressPreference(task, progressSignal)
        val rrCandidates = if (
            task.effectiveComplexity() == TaskComplexity.COMPLEX &&
            task.effectiveDeadline() == null &&
            currentWeights.weightedRoundRobinEnabled &&
            progressPreference == ProgressPreference.NONE
        ) {
            val bestScore = eligible.minOf { it.weightedScore }
            val tolerance = max(0.0, currentWeights.roundRobinScoreTolerance)
            eligible.filter { evaluation ->
                evaluation.weightedScore <= bestScore * (1.0 + tolerance) + 1e-9
            }
        } else {
            emptyList()
        }
        val roundRobinSelection = if (rrCandidates.size > 1) {
            smoothWeightedRoundRobin(task, rrCandidates)
        } else {
            null
        }
        val preferred = preferredNodeId?.let { id ->
            eligible.firstOrNull { it.node.nodeId == id }
        }
        val migration = applyMigrationHysteresis(
            task = task,
            baseline = roundRobinSelection ?: baseline,
            preferred = preferred,
            migrationHysteresisRatio = migrationHysteresisRatio,
            slicesExecuted = slicesExecuted,
            minSlicesBeforeMigration = minSlicesBeforeMigration
        )
        val selected = migration.first
        val usedRoundRobin = roundRobinSelection != null && selected.node.nodeId == roundRobinSelection.node.nodeId
        val reason = when {
            migration.second != null -> migration.second!!
            progressPreference == ProgressPreference.FAST_UPGRADE -> "progress_fast_upgrade"
            progressPreference == ProgressPreference.CHEAP_DOWNGRADE -> "progress_cheap_downgrade"
            usedRoundRobin -> "weighted_round_robin"
            task.effectiveDeadline() != null && selected.deadlineRisk <= 0.0 -> "deadline_satisfied_low_cost"
            task.effectiveDeadline() != null -> "lowest_deadline_risk"
            else -> "lowest_weighted_score"
        }
        return NodeSelectionDecision(
            selectedNode = selected.node,
            reason = reason,
            candidates = evaluations.map { evaluation ->
                evaluation.toCandidate(
                    roundRobinWeight = rrCandidates
                        .firstOrNull { it.node.nodeId == evaluation.node.nodeId }
                        ?.let(::roundRobinWeight)
                )
            },
            usedWeightedRoundRobin = usedRoundRobin,
            migrationDecision = when {
                migration.second == "migration_hysteresis_hold" -> "held"
                preferredNodeId != null && selected.node.nodeId != preferredNodeId -> "migrated"
                preferredNodeId != null -> "held"
                else -> "initial"
            }
        )
    }

    private fun baselineSelection(
        task: TaskState,
        evaluations: List<NodeEvaluation>,
        progressSignal: ProgressSignal?
    ): NodeEvaluation {
        if (progressSignal?.deadlineRiskIncreasing == true) {
            return evaluations.minWithOrNull(
                compareBy<NodeEvaluation> { it.etaSeconds }
                    .thenBy { it.deadlineRisk }
                    .thenBy { it.cost }
                    .thenBy { it.node.nodeId.value }
            )!!
        }
        if (task.effectiveDeadline() != null) {
            val deadlineSatisfied = evaluations.filter { it.deadlineRisk <= 0.0 }
            if (deadlineSatisfied.isNotEmpty()) {
                return deadlineSatisfied.minWithOrNull(
                    compareBy<NodeEvaluation> { it.cost }
                        .thenBy { it.deadlineRisk }
                        .thenBy { it.queueDelay }
                        .thenBy { it.node.nodeId.value }
                )!!
            }
            return evaluations.minWithOrNull(
                compareBy<NodeEvaluation> { it.deadlineRisk }
                    .thenBy { it.cost }
                    .thenBy { it.queueDelay }
                    .thenBy { it.node.nodeId.value }
            )!!
        }
        when (progressPreference(task, progressSignal)) {
            ProgressPreference.FAST_UPGRADE -> return evaluations.minWithOrNull(
                compareBy<NodeEvaluation> { it.etaSeconds }
                    .thenBy { it.cost }
                    .thenBy { it.node.nodeId.value }
            )!!

            ProgressPreference.CHEAP_DOWNGRADE -> return evaluations.minWithOrNull(
                compareBy<NodeEvaluation> { it.cost }
                    .thenBy { it.etaSeconds }
                    .thenBy { it.node.nodeId.value }
            )!!

            ProgressPreference.NONE -> Unit
        }
        return evaluations.minWithOrNull(
            compareBy<NodeEvaluation> { it.weightedScore }
                .thenBy { it.cost }
                .thenBy { it.node.nodeId.value }
        )!!
    }

    private fun applyMigrationHysteresis(
        task: TaskState,
        baseline: NodeEvaluation,
        preferred: NodeEvaluation?,
        migrationHysteresisRatio: Double,
        slicesExecuted: Int,
        minSlicesBeforeMigration: Int
    ): Pair<NodeEvaluation, String?> {
        if (preferred == null || preferred.node.nodeId == baseline.node.nodeId) {
            return baseline to null
        }
        if (slicesExecuted.coerceAtLeast(0) < minSlicesBeforeMigration.coerceAtLeast(0)) {
            return preferred to "migration_hysteresis_hold"
        }
        // A deadline miss cannot be retained merely to avoid migration.
        if (task.effectiveDeadline() != null && preferred.deadlineRisk > 0.0 && baseline.deadlineRisk <= 0.0) {
            return baseline to null
        }
        val improvement = (preferred.weightedScore - baseline.weightedScore) /
            max(1e-9, kotlin.math.abs(preferred.weightedScore))
        return if (improvement < migrationHysteresisRatio.coerceAtLeast(0.0)) {
            preferred to "migration_hysteresis_hold"
        } else {
            baseline to null
        }
    }

    private fun smoothWeightedRoundRobin(
        task: TaskState,
        evaluations: List<NodeEvaluation>
    ): NodeEvaluation {
        val ordered = evaluations.sortedBy { it.node.nodeId.value }
        val key = buildRoundRobinKey(task, ordered)
        val state = roundRobinCurrentWeights.computeIfAbsent(key) { ConcurrentHashMap() }
        val activeIds = ordered.map { it.node.nodeId.value }.toSet()
        synchronized(state) {
            state.keys.retainAll(activeIds)
            var totalWeight = 0L
            var selected: NodeEvaluation? = null
            var selectedCurrent = Long.MIN_VALUE
            for (evaluation in ordered) {
                val weight = roundRobinWeight(evaluation)
                totalWeight += weight
                val current = (state[evaluation.node.nodeId.value] ?: 0L) + weight
                state[evaluation.node.nodeId.value] = current
                if (
                    selected == null ||
                    current > selectedCurrent ||
                    (current == selectedCurrent && evaluation.node.nodeId.value < selected!!.node.nodeId.value)
                ) {
                    selected = evaluation
                    selectedCurrent = current
                }
            }
            val chosen = selected!!
            state[chosen.node.nodeId.value] = (state[chosen.node.nodeId.value] ?: 0L) - totalWeight
            return chosen
        }
    }

    private fun buildRoundRobinKey(task: TaskState, evaluations: List<NodeEvaluation>): String =
        listOf(
            task.payload.modelData.modelType.name,
            task.payload.taskMeta.solverType?.value ?: "*",
            evaluations.joinToString(",") { it.node.nodeId.value }
        ).joinToString("|")

    private fun roundRobinWeight(evaluation: NodeEvaluation): Long {
        val capacity = max(0.1, evaluation.node.profile.performanceScore.toDouble()) *
            max(1, evaluation.node.profile.parallelUnits).toDouble()
        return max(1.0, (capacity * 100.0).roundToLong().toDouble()).roundToLong()
    }

    private fun evaluate(
        task: TaskState,
        node: NodeState,
        nowEpochMs: Long,
        preferredNodeId: fuookami.ospf.framework.remote_solver.protocol.domain.NodeId?,
        migrationCostWeight: Double
    ): NodeEvaluation {
        val etaSeconds = estimateEtaSeconds(task, node)
        val queueDelay = task.effectiveQueueWaitEstimateMs()?.toDouble()?.div(1000.0)
            ?: queueDelaySeconds(node, etaSeconds)
        // Cost and deadline risk must use the same node-specific execution
        // estimate. A caller-supplied cost is a task estimate, not a license
        // or billing override for every candidate node.
        val cost = estimatedCost(task, node, etaSeconds)
        val deadlineRisk = deadlineRisk(task, etaSeconds, queueDelay, nowEpochMs)
        val migrationCost = migrationCost(task, node, preferredNodeId)
        val remainingProgress = progressNeed(task)
        val performancePenalty = remainingProgress / max(0.1, node.profile.performanceScore.toDouble())
        val weightedScore = currentWeights.costWeight * cost +
            currentWeights.deadlineRiskWeight * deadlineRisk +
            currentWeights.queueDelayWeight * queueDelay +
            currentWeights.progressWeight.coerceAtLeast(0.0) * performancePenalty +
            migrationCostWeight.coerceAtLeast(0.0) * migrationCost
        return NodeEvaluation(
            node = node,
            etaSeconds = etaSeconds,
            cost = cost,
            deadlineRisk = deadlineRisk,
            queueDelay = queueDelay,
            weightedScore = weightedScore,
            migrationCost = migrationCost,
            progressNeed = remainingProgress,
            admissionReasons = admissionReasons(task, node)
        )
    }

    private fun admissionReasons(task: TaskState, node: NodeState): List<AdmissionReasonCode> {
        val reasons = mutableListOf<AdmissionReasonCode>()
        if (!node.online) {
            reasons += AdmissionReasonCode.OFFLINE
        }
        if (node.availableUnits <= 0) {
            reasons += AdmissionReasonCode.NO_AVAILABLE_SLOT
        }
        val modelType = resolvedModelType(task)
        if (modelType == null || modelType !in node.profile.supportedModelTypes) {
            reasons += AdmissionReasonCode.MODEL_UNSUPPORTED
        }
        val requiredSolverType = task.payload.taskMeta.solverType?.value
            ?: task.payload.extension["solverType"]
            ?: task.payload.taskMeta.metadata["solverType"]
        if (requiredSolverType != null &&
            !node.profile.solverType.value.equals(requiredSolverType, ignoreCase = true)
        ) {
            reasons += AdmissionReasonCode.SOLVER_UNSUPPORTED
        }
        if (requiresCapability(task, "requiresCheckpoint") && !node.profile.supportsCheckpoint) {
            reasons += AdmissionReasonCode.CHECKPOINT_UNSUPPORTED
        }
        if (requiresCapability(task, "requiresWarmStart") && !node.profile.supportsWarmStart) {
            reasons += AdmissionReasonCode.WARM_START_UNSUPPORTED
        }
        if (requiresCapability(task, "requiresInterrupt") && !node.profile.supportsInterrupt) {
            reasons += AdmissionReasonCode.INTERRUPT_UNSUPPORTED
        }
        reasons += task.schedulingModeAdmissionReasons(node)
        val checkpointRef = task.effectiveCheckpointRef()
        if (checkpointRef != null && !canResumeCheckpoint(task, node)) {
            if (task.payload.scheduling?.resumeMode ==
                fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.NATIVE_CHECKPOINT &&
                !node.profile.supportsNativeCheckpoint
            ) {
                reasons += AdmissionReasonCode.CHECKPOINT_UNSUPPORTED
            } else {
                reasons += AdmissionReasonCode.WARM_START_UNSUPPORTED
            }
        }
        return reasons
    }

    /** Returns a parsed model type, or null when admission cannot be safe. */
    private fun resolvedModelType(task: TaskState): NormalizedModelType? {
        val modelType = task.payload.modelData.modelType
        if (modelType != NormalizedModelType.UNKNOWN) {
            return modelType
        }
        val declared = task.payload.extension["modelType"]
            ?: task.payload.taskMeta.metadata["modelType"]
            ?: task.payload.taskMeta.targetType?.value
        val parsed = declared?.trim()?.uppercase()?.let {
            when (it) {
                "LINEAR", "LP" -> NormalizedModelType.LINEAR
                "QUADRATIC", "QP" -> NormalizedModelType.QUADRATIC
                "CP", "CONSTRAINT-PROGRAMMING", "CONSTRAINT_PROGRAMMING" -> NormalizedModelType.CP
                else -> null
            }
        }
        if (parsed != null) {
            return parsed
        }
        // A legacy/opaque reference has no inline bytes from which the
        // dispatcher can infer a type. Keep UNKNOWN explicit: only a node
        // that advertises UNKNOWN may admit it, so this never bypasses the
        // supported-model capability filter. Otherwise admission reports
        // MODEL_UNSUPPORTED instead of silently queueing an unsafe dispatch.
        return if (task.payload.modelData.ref != null && task.payload.modelData.format == null) {
            task.payload.modelData.ref.let { NormalizedModelType.UNKNOWN }
        } else {
            null
        }
    }

    private fun canResumeCheckpoint(task: TaskState, node: NodeState): Boolean = when (
        task.payload.scheduling?.resumeMode
    ) {
        fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.NATIVE_CHECKPOINT ->
            node.profile.supportsNativeCheckpoint
        fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.BASIS -> false
        else -> node.profile.supportsWarmStart
    }

    private fun requiresCapability(task: TaskState, key: String): Boolean =
        task.payload.extension[key]?.trim()?.equals("true", ignoreCase = true) == true ||
            task.payload.taskMeta.metadata[key]?.trim()?.equals("true", ignoreCase = true) == true

    private fun NodeEvaluation.toCandidate(roundRobinWeight: Long? = null): NodeSelectionCandidate =
        NodeSelectionCandidate(
            nodeId = node.nodeId.value,
            eligible = admissionReasons.isEmpty(),
            reasons = admissionReasons,
            estimatedCost = cost,
            deadlineRisk = deadlineRisk,
            queueDelaySeconds = queueDelay,
            weightedScore = weightedScore,
            roundRobinWeight = roundRobinWeight,
            migrationCost = migrationCost,
            progressNeed = progressNeed
        )

    private fun estimateEtaSeconds(task: TaskState, node: NodeState): Double {
        val base = when (task.effectiveComplexity()) {
            TaskComplexity.SIMPLE -> 20.0
            TaskComplexity.COMPLEX -> 90.0
        }
        val estimatedRuntimeSeconds = task.effectiveRuntimeEstimateMs()?.toDouble()?.div(1000.0)
        val realtimeFactor = when (task.effectiveTimeSensitivity()) {
            TimeSensitivity.REALTIME -> 0.75
            TimeSensitivity.NON_REALTIME -> 1.0
        }
        val performanceAdjusted = (estimatedRuntimeSeconds ?: base) /
            max(0.1, node.profile.performanceScore.toDouble())
        return max(
            node.profile.minBillingUnitSeconds.toDouble(),
            performanceAdjusted * realtimeFactor
        )
    }

    private enum class ProgressPreference {
        NONE,
        FAST_UPGRADE,
        CHEAP_DOWNGRADE
    }

    /**
     * Pick a clear policy at the ends of the progress spectrum. Near the
     * start, remaining work justifies a faster node; near completion, paying
     * for speed is wasteful. Deadlined tasks continue through deadline logic.
     */
    private fun progressPreference(
        task: TaskState,
        signal: ProgressSignal?
    ): ProgressPreference {
        val observed = signal ?: return ProgressPreference.NONE
        if (observed.deadlineRiskIncreasing) {
            return ProgressPreference.FAST_UPGRADE
        }

        val gap = (observed.currentGap ?: task.latestResult?.gap?.toDouble())
            ?.coerceIn(0.0, 1.0)
            ?: return ProgressPreference.NONE
        val improvement = observed.improvement
        val hasMeasuredHistory = observed.previousGap != null
        val meaningfulImprovement = improvement != null &&
            improvement >= currentWeights.progressMinImprovement.coerceAtLeast(0.0)
        val noImprovement = hasMeasuredHistory && (
            observed.noImprovementSlices >= currentWeights.progressNoImprovementSlices.coerceAtLeast(1) ||
                improvement == null ||
                improvement < currentWeights.progressMinImprovement.coerceAtLeast(0.0)
            )
        val regressed = improvement != null && improvement < 0.0

        // A fast node is justified by a high remaining gap plus measured
        // stagnation/regression. The absolute gap is only a gate, never the
        // signal by itself.
        if (
            gap >= currentWeights.progressFastGapThreshold.coerceIn(0.0, 1.0) &&
            hasMeasuredHistory &&
            (noImprovement || regressed) &&
            !meaningfulImprovement
        ) {
            return ProgressPreference.FAST_UPGRADE
        }

        val latest = task.latestResult
        val hasIncumbent = latest?.feasible == true &&
            latest.solutionPresence != fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence.NONE
        val highQualityIncumbent = hasIncumbent && (
            latest.optimal ||
                latest.proofStatus == fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus.VERIFIED ||
                gap <= currentWeights.progressCheapGapThreshold.coerceIn(0.0, 1.0)
            )
        // Once a credible incumbent exists, a flat/slowly improving gap in
        // the low-gap region favors a cheaper node.
        if (
            highQualityIncumbent &&
            gap <= currentWeights.progressCheapGapThreshold.coerceIn(0.0, 1.0) &&
            hasMeasuredHistory &&
            noImprovement &&
            !regressed
        ) {
            return ProgressPreference.CHEAP_DOWNGRADE
        }
        return ProgressPreference.NONE
    }

    private fun progressNeed(task: TaskState): Double =
        task.latestResult?.gap?.toDouble()?.coerceIn(0.0, 1.0) ?: 1.0

    private fun migrationCost(task: TaskState, node: NodeState, preferredNodeId: fuookami.ospf.framework.remote_solver.protocol.domain.NodeId?): Double {
        if (preferredNodeId == null || preferredNodeId == node.nodeId) {
            return 0.0
        }
        val checkpointSeconds = (task.effectiveCheckpointEstimateMs() ?: 0L).toDouble() / 1000.0
        return checkpointSeconds * node.profile.pricePerSecond.toDouble() +
            node.profile.licenseCostPerSlice.toDouble()
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

    private fun deadlineRisk(
        task: TaskState,
        etaSeconds: Double,
        queueDelaySeconds: Double,
        nowEpochMs: Long
    ): Double {
        val deadlineEpochMs = task.effectiveDeadline()?.toEpochMilliseconds() ?: return 0.0
        val predictedDurationMs = ((max(0.0, queueDelaySeconds) + max(0.0, etaSeconds)) * 1000.0)
            .toLong()
        val predictedFinish = nowEpochMs + predictedDurationMs
        if (predictedFinish <= deadlineEpochMs) {
            return 0.0
        }
        val window = max(1L, deadlineEpochMs - nowEpochMs).toDouble()
        return (predictedFinish - deadlineEpochMs) / window
    }

    private fun estimatedCost(task: TaskState, node: NodeState, etaSeconds: Double): Double {
        val billedSeconds = max(
            node.profile.minBillingUnitSeconds.toDouble(),
            kotlin.math.ceil(max(0.0, etaSeconds))
        )
        val nodeCost = billedSeconds * node.profile.pricePerSecond.toDouble() +
            node.profile.licenseCostPerSlice.toDouble()
        // The caller estimate describes fixed setup/accounting work that is
        // independent of the selected node. Add it to the node-specific
        // runtime, billing-unit, and license cost so a large estimate cannot
        // flatten candidate ordering.
        return nodeCost + (task.effectiveCostEstimate() ?: 0.0)
    }
}
