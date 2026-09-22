@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * Dispatcher scheduling decision models.
 *
 * These models keep admission and scheduling explanations inside the
 * application layer. They are deliberately not persisted and are not part
 * of the remote solver protocol.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import kotlin.time.Instant

/** The scheduling class derived from task complexity and time sensitivity. */
enum class TaskAdmissionClass {
    SIMPLE_REALTIME,
    SIMPLE_BATCH,
    COMPLEX_BATCH;

    companion object {
        fun of(task: TaskState): TaskAdmissionClass = when {
            task.effectiveComplexity() == TaskComplexity.COMPLEX -> COMPLEX_BATCH
            task.effectiveTimeSensitivity() == TimeSensitivity.REALTIME -> SIMPLE_REALTIME
            else -> SIMPLE_BATCH
        }
    }
}

/** Reasons used by the dispatcher when a node cannot admit a task. */
enum class AdmissionReasonCode {
    ACCEPTED,
    OFFLINE,
    NO_AVAILABLE_SLOT,
    MODEL_UNSUPPORTED,
    SOLVER_UNSUPPORTED,
    INTERRUPT_UNSUPPORTED,
    CHECKPOINT_UNSUPPORTED,
    WARM_START_UNSUPPORTED,
    BUDGET_EXCEEDED,
    WAITING_FOR_BUDGET,
    NO_COMPATIBLE_NODE
}

data class NodeAdmissionDecision(
    val nodeId: String,
    val eligible: Boolean,
    val reasons: List<AdmissionReasonCode> = emptyList()
)

data class AdmissionDecision(
    val taskId: String,
    val taskClass: TaskAdmissionClass,
    val admitted: Boolean,
    val reason: AdmissionReasonCode,
    val nodes: List<NodeAdmissionDecision>
) {
    val eligibleNodeIds: List<String> get() = nodes.filter { it.eligible }.map { it.nodeId }
}

data class NodeSelectionCandidate(
    val nodeId: String,
    val eligible: Boolean,
    val reasons: List<AdmissionReasonCode> = emptyList(),
    val estimatedCost: Double? = null,
    val deadlineRisk: Double? = null,
    val queueDelaySeconds: Double? = null,
    val weightedScore: Double? = null,
    val roundRobinWeight: Long? = null,
    /** Estimated one-slice cost incurred by moving from the preferred node. */
    val migrationCost: Double? = null,
    /** Remaining progress demand used by the fast/cheap policy. */
    val progressNeed: Double? = null
)

data class NodeSelectionDecision(
    val selectedNode: NodeState?,
    val reason: String,
    val candidates: List<NodeSelectionCandidate>,
    val usedWeightedRoundRobin: Boolean = false,
    val migrationDecision: String = "initial"
) {
    val selectedNodeId: NodeId? get() = selectedNode?.nodeId
    val selectedScore: Double? get() = selectedNodeId?.let { id ->
        candidates.firstOrNull { it.nodeId == id.value }?.weightedScore
    }
}

/**
 * Runtime progress observation used by the node policy.
 *
 * A gap value alone is not enough to call a task "slow": the policy needs
 * the previous observation or an explicit no-improvement streak.
 */
data class ProgressSignal(
    val previousGap: Double? = null,
    val currentGap: Double? = null,
    /** Positive values mean that the optimality gap improved. */
    val improvement: Double? = null,
    val noImprovementSlices: Int = 0,
    val deadlineRiskIncreasing: Boolean = false
)

data class QuantumDecision(
    val quantumMs: Long,
    val reason: String,
    val complexity: TaskComplexity,
    val performanceScore: Double,
    val checkpointEstimateMs: Long
)

data class SliceAuditRecord(
    val admissionClass: TaskAdmissionClass,
    val selectionReason: String,
    val selectionScore: Double? = null,
    val quantumReason: String,
    val migrationDecision: String = "initial",
    val eligibleNodeIds: List<String> = emptyList()
)

/** Effective scheduling values used by both admission and execution paths. */
internal fun TaskState.effectiveComplexity(): TaskComplexity =
    payload.scheduling?.complexity ?: complexity

internal fun TaskState.effectiveTimeSensitivity(): TimeSensitivity {
    val declared = payload.scheduling?.timeSensitivity ?: timeSensitivity
    return if (effectiveComplexity() == TaskComplexity.COMPLEX && declared == TimeSensitivity.REALTIME) {
        TimeSensitivity.NON_REALTIME
    } else {
        declared
    }
}

internal fun TaskState.effectivePriority(): Int =
    payload.scheduling?.priority ?: priority

internal fun TaskState.effectiveDeadline(): Instant? =
    payload.scheduling?.deadline ?: deadline

internal fun TaskState.effectiveBudgetScope(): BudgetScopeId =
    payload.scheduling?.budgetScope ?: budgetScope

internal fun TaskState.effectiveBudgetLimit(): Flt64? =
    payload.scheduling?.budgetLimit ?: budgetLimit

internal fun TaskState.effectiveCheckpointRef(): ObjectRef? =
    latestSnapshotRef ?: payload.scheduling?.checkpointRef

internal fun TaskState.effectiveRuntimeEstimateMs(): Long? =
    payload.scheduling?.estimate?.runtime?.inWholeMilliseconds?.takeIf { it > 0L }

internal fun TaskState.effectiveCheckpointEstimateMs(): Long? =
    payload.scheduling?.estimate?.checkpoint?.inWholeMilliseconds?.takeIf { it >= 0L }

internal fun TaskState.effectiveQueueWaitEstimateMs(): Long? =
    payload.scheduling?.estimate?.queueWait?.inWholeMilliseconds?.takeIf { it >= 0L }

internal fun TaskState.effectiveCostEstimate(): Double? =
    payload.scheduling?.estimate?.cost?.toDouble()?.takeIf { it.isFinite() && it >= 0.0 }

/** Convert declared protocol execution modes into the dispatcher's capability vocabulary. */
internal fun TaskState.schedulingModeAdmissionReasons(node: NodeState): List<AdmissionReasonCode> = buildList {
    when (payload.scheduling?.preemptionMode) {
        fuookami.ospf.framework.remote_solver.protocol.domain.PreemptionMode.CONTROLLED_RETURN -> {
            if (!node.profile.supportsCheckpoint) add(AdmissionReasonCode.CHECKPOINT_UNSUPPORTED)
            if (!node.profile.supportsWarmStart) add(AdmissionReasonCode.WARM_START_UNSUPPORTED)
        }

        fuookami.ospf.framework.remote_solver.protocol.domain.PreemptionMode.NATIVE -> {
            if (!node.profile.supportsInterrupt) add(AdmissionReasonCode.INTERRUPT_UNSUPPORTED)
            if (!node.profile.supportsCheckpoint) add(AdmissionReasonCode.CHECKPOINT_UNSUPPORTED)
        }

        else -> Unit
    }
    when (payload.scheduling?.resumeMode) {
        fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.WARM_START -> {
            if (!node.profile.supportsWarmStart) add(AdmissionReasonCode.WARM_START_UNSUPPORTED)
        }

        // NodeCapabilityProfile has no basis-specific capability. Do not silently
        // treat a basis declaration as a generic warm start.
        fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.BASIS ->
            add(AdmissionReasonCode.WARM_START_UNSUPPORTED)

        fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode.NATIVE_CHECKPOINT -> {
            if (!node.profile.supportsCheckpoint) add(AdmissionReasonCode.CHECKPOINT_UNSUPPORTED)
            if (!node.profile.supportsInterrupt) add(AdmissionReasonCode.INTERRUPT_UNSUPPORTED)
        }

        else -> Unit
    }
}
