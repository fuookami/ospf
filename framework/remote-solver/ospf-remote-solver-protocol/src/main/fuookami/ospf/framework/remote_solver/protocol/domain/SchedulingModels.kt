@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.protocol.domain

import kotlin.time.Duration
import kotlin.time.Instant
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/**
 * How a running solver slice may be stopped by the scheduler.
 *
 * The value describes an execution guarantee, rather than a scheduler
 * preference. A missing value means that the sender has no requirement.
 */
@Serializable
enum class PreemptionMode {
    /** The execution cannot be safely stopped between slices. */
    NON_PREEMPTIBLE,

    /** The worker returns at a solver-safe boundary and resumes from state supplied by the caller. */
    CONTROLLED_RETURN,

    /** The solver provides a native interrupt and native search-state checkpoint. */
    NATIVE
}

/**
 * How a later slice can continue a previous slice.
 *
 * The modes are ordered by the kind of state they carry, but consumers must
 * still check the advertised worker capability before selecting one.
 */
@Serializable
enum class ResumeMode {
    /** No resumable state is available. */
    NONE,

    /** Continue with a solver warm start without a portable search checkpoint. */
    WARM_START,

    /** Continue with a linear-programming basis or equivalent backend state. */
    BASIS,

    /** Continue from a backend-native checkpoint. */
    NATIVE_CHECKPOINT
}

/**
 * Explicit result of a scheduler slice.
 *
 * `completed` and `terminationReason` remain available for existing clients;
 * this field distinguishes a resumable preemption from a terminal result.
 */
@Serializable
enum class SliceOutcome {
    /** The slice reached a terminal solve result. */
    COMPLETED,

    /** The scheduler stopped the slice and another slice may continue it. */
    PREEMPTED,

    /** A checkpoint was emitted while the slice was stopped. */
    CHECKPOINTED,

    /** The slice was stopped without a checkpoint but retained a resumable incumbent. */
    RESUMABLE,

    /** The slice was cancelled and must not be resumed. */
    CANCELLED,

    /** The slice ended because of an execution failure. */
    FAILED
}

/**
 * Estimates supplied by a caller or worker for scheduler accounting.
 *
 * All durations are encoded as integer milliseconds to match the existing
 * remote protocol wire format.
 */
@Serializable
data class SchedulingEstimate(
    @SerialName("estimatedRuntimeMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val runtime: Duration? = null,
    @SerialName("estimatedCheckpointMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val checkpoint: Duration? = null,
    @SerialName("estimatedQueueWaitMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val queueWait: Duration? = null,
    val cost: Flt64? = null
)

/**
 * Quality target used by the scheduler and solver worker.
 *
 * The target is intentionally solver-neutral: a backend may satisfy any
 * subset it understands and reports the effective result in its normal
 * result fields.
 */
@Serializable
data class QualityTarget(
    val maxGap: Flt64? = null,
    val objectiveLimit: Flt64? = null,
    val requireFeasible: Boolean? = null,
    val requireOptimal: Boolean? = null,
    val metadata: Map<String, String> = emptyMap()
)

/**
 * Additive scheduling information attached to a solve request.
 *
 * This is optional so an older payload containing only model and solver data
 * remains valid. When present, fields describe requirements or accounting
 * inputs for the task; they do not replace the canonical solver settings in
 * [TaskMeta] or [SolverConfig].
 */
@Serializable
data class SchedulingRequest(
    val complexity: TaskComplexity? = null,
    val timeSensitivity: TimeSensitivity? = null,
    val priority: Int? = null,
    @SerialName("deadlineEpochMs")
    @Serializable(with = RemoteSolverEpochMillisecondsInstantSerializer::class)
    val deadline: Instant? = null,
    val budgetScope: BudgetScopeId? = null,
    val budgetLimit: Flt64? = null,
    val estimate: SchedulingEstimate? = null,
    val modelFingerprint: String? = null,
    val modelFingerprintSchema: String? = null,
    val checkpointRef: ObjectRef? = null,
    val incumbentRef: ObjectRef? = null,
    val qualityTarget: QualityTarget? = null,
    val preemptionMode: PreemptionMode? = null,
    val resumeMode: ResumeMode? = null,
    val metadata: Map<String, String> = emptyMap()
)

/**
 * Effective scheduling information returned with a slice or final result.
 *
 * This records what was actually selected, including the effective quantum,
 * queue delay and state references. It is informational and additive; legacy
 * consumers can ignore the whole object.
 */
@Serializable
data class SchedulingDecision(
    val dispatchId: DispatchId? = null,
    val taskId: TaskId? = null,
    val sliceId: SliceId? = null,
    val nodeId: NodeId? = null,
    val priority: Int? = null,
    @SerialName("deadlineEpochMs")
    @Serializable(with = RemoteSolverEpochMillisecondsInstantSerializer::class)
    val deadline: Instant? = null,
    val budgetScope: BudgetScopeId? = null,
    val budgetLimit: Flt64? = null,
    @SerialName("quantumMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val quantum: Duration? = null,
    @SerialName("queueWaitMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val queueWait: Duration? = null,
    val estimate: SchedulingEstimate? = null,
    val modelFingerprint: String? = null,
    val modelFingerprintSchema: String? = null,
    val checkpointRef: ObjectRef? = null,
    val incumbentRef: ObjectRef? = null,
    val qualityTarget: QualityTarget? = null,
    val preemptionMode: PreemptionMode? = null,
    val resumeMode: ResumeMode? = null,
    val outcome: SliceOutcome? = null,
    val reason: String? = null,
    val metadata: Map<String, String> = emptyMap()
)
