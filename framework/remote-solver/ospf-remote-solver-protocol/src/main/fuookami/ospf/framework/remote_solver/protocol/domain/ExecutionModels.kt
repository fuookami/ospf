@file:OptIn(kotlin.time.ExperimentalTime::class)
package fuookami.ospf.framework.remote_solver.protocol.domain

import kotlin.time.Duration
import kotlin.time.DurationUnit
import kotlin.time.Instant
import kotlin.time.toDuration
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/**
 * 求解器类型。
 * Solver type.
 */
@Serializable
enum class SolverType {
    /** SCIP 求解器 / SCIP solver */
    SCIP,

    /** Gurobi 求解器 / Gurobi solver */
    GUROBI,

    /** 自动选择 / Auto selection */
    AUTO
}

/** 远程问题结论。 / Remote problem conclusion. */
@Serializable
enum class RemoteProblemStatus {
    /** 存在可行解 / A feasible solution exists */
    FEASIBLE,

    /** 已证明不可行 / Proven infeasible */
    INFEASIBLE,

    /** 已证明无界 / Proven unbounded */
    UNBOUNDED,

    /** 不可行或无界 / Infeasible or unbounded */
    INFEASIBLE_OR_UNBOUNDED,

    /** 未知 / Unknown */
    UNKNOWN
}

/** 远程终止原因。 / Remote termination reason. */
@Serializable
enum class RemoteTerminationReason {
    /** 正常完成 / Completed normally */
    COMPLETED,

    /** 时间限制 / Time limit */
    TIME_LIMIT,

    /** 节点限制 / Node limit */
    NODE_LIMIT,

    /** 迭代限制 / Iteration limit */
    ITERATION_LIMIT,

    /** 解数量限制 / Solution limit */
    SOLUTION_LIMIT,

    /** 目标限制 / Objective limit */
    OBJECTIVE_LIMIT,

    /** 已取消 / Cancelled */
    CANCELLED,

    /** 已中断 / Interrupted */
    INTERRUPTED,

    /** 数值失败 / Numerical failure */
    NUMERICAL_FAILURE,

    /** 后端失败 / Backend failure */
    BACKEND_FAILURE
}

/** 远程解存在性。 / Remote solution presence. */
@Serializable
enum class RemoteSolutionPresence {
    /** 无解 / No solution */
    NONE,

    /** 有 incumbent / Incumbent exists */
    INCUMBENT,

    /** 已证明最优解 / Proven optimal solution */
    OPTIMAL
}

/** 远程证明状态。 / Remote proof status. */
@Serializable
enum class RemoteProofStatus {
    /** 没有证明 / No proof */
    NONE,

    /** 仅声明，未经独立验证 / Claimed without independent verification */
    CLAIMED,

    /** 已通过后端和协议复验 / Verified by backend and protocol checks */
    VERIFIED
}

/**
 * 执行句柄。
 * Execution handle.
 *
 * @property handleId 句柄 ID / Handle ID
 * @property taskId 任务 ID / Task ID
 * @property sliceId 切片 ID / Slice ID
 * @property nodeId 节点 ID / Node ID
 * @property startedAt 启动时间戳 / Started timestamp
 */
@Serializable
data class ExecutionHandle(
    val handleId: HandleId,
    val taskId: TaskId,
    val sliceId: SliceId,
    val nodeId: NodeId,
    @SerialName("startedAtEpochMs")
    @Serializable(with = RemoteSolverEpochMillisecondsInstantSerializer::class)
    val startedAt: Instant
) {
    constructor(
        handleId: String,
        taskId: String,
        sliceId: String,
        nodeId: String,
        startedAtEpochMs: Long
    ) : this(
        handleId = HandleId.of(handleId),
        taskId = TaskId.of(taskId),
        sliceId = SliceId.of(sliceId),
        nodeId = NodeId.of(nodeId),
        startedAt = Instant.fromEpochMilliseconds(startedAtEpochMs)
    )

    /** 启动时间戳（毫秒）兼容别名 / Compatibility alias for started epoch milliseconds */
    val startedAtEpochMs: Long get() = startedAt.toEpochMilliseconds()
}

/**
 * 切片结果。
 * Slice result.
 *
 * @property sliceId 切片 ID / Slice ID
 * @property completed 是否完成 / Whether completed
 * @property feasible 是否可行 / Whether feasible
 * @property objectiveValue 目标值 / Objective value
 * @property objectiveValueInt64 CP 精确整数目标值 / Exact Int64 CP objective value
 * @property gap 最优间隙 / Optimality gap
 * @property elapsed 耗时 / Elapsed
 * @property message 结果消息 / Result message
 * @property schemaVersion 结果协议版本 / Result protocol version
 * @property problemStatus 正交问题结论 / Orthogonal problem conclusion
 * @property terminationReason 正交终止原因 / Orthogonal termination reason
 * @property solutionPresence 解存在性 / Solution presence
 * @property proofStatus 证明状态 / Proof status
 * @property resultRef 结果对象引用 / Result artifact reference
 * @property provenance 脱敏执行来源 / Redacted execution provenance
 * @property fingerprints 审计指纹 / Audit fingerprints
 * @property fingerprintSchemas 指纹 schema / Fingerprint schemas
 * @property statistics 求解统计 / Solve statistics
 * @property diagnostics 结构化诊断 / Structured diagnostics
 * @property runId 求解运行标识 / Solve run identifier
 * @property attemptId 求解尝试标识 / Solve attempt identifier
 * @property artifactDigest 结果 artifact 摘要 / Result artifact digest
 */
@Serializable
data class SliceResult(
    val sliceId: SliceId,
    val completed: Boolean,
    val feasible: Boolean,
    val objectiveValue: Flt64?,
    val gap: Flt64?,
    @SerialName("elapsedMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val elapsed: Duration,
    val message: String? = null,
    val schemaVersion: String = "1.0",
    val problemStatus: RemoteProblemStatus = if (feasible) {
        RemoteProblemStatus.FEASIBLE
    } else {
        RemoteProblemStatus.UNKNOWN
    },
    val terminationReason: RemoteTerminationReason = RemoteTerminationReason.COMPLETED,
    val solutionPresence: RemoteSolutionPresence = when {
        feasible -> RemoteSolutionPresence.INCUMBENT
        else -> RemoteSolutionPresence.NONE
    },
    val proofStatus: RemoteProofStatus = RemoteProofStatus.NONE,
    val resultRef: ObjectRef? = null,
    val provenance: Map<String, String> = emptyMap(),
    val fingerprints: Map<String, String> = emptyMap(),
    val fingerprintSchemas: Map<String, String> = emptyMap(),
    val statistics: Map<String, String> = emptyMap(),
    val diagnostics: Map<String, String> = emptyMap(),
    val runId: String? = null,
    val attemptId: String? = null,
    val artifactDigest: String? = null,
    val objectiveValueInt64: Long? = null
) {
    constructor(
        sliceId: String,
        completed: Boolean,
        feasible: Boolean,
        objectiveValue: Double?,
        gap: Double?,
        elapsedMs: Long,
        message: String? = null
    ) : this(
        sliceId = SliceId.of(sliceId),
        completed = completed,
        feasible = feasible,
        objectiveValue = objectiveValue?.let { Flt64(it) },
        gap = gap?.let { Flt64(it) },
        elapsed = elapsedMs.toDuration(DurationUnit.MILLISECONDS),
        message = message
    )

    /** 耗时毫秒兼容别名 / Compatibility alias for elapsed milliseconds */
    val elapsedMs: Long get() = elapsed.inWholeMilliseconds
}

/**
 * 求解结果。
 * Solve result.
 *
 * @property feasible 是否可行 / Whether feasible
 * @property optimal 是否最优 / Whether optimal
 * @property objectiveValue 目标值 / Objective value
 * @property objectiveValueInt64 CP 精确整数目标值 / Exact Int64 CP objective value
 * @property gap 最优间隙 / Optimality gap
 * @property elapsed 总耗时 / Total elapsed
 * @property checkpointRef 检查点引用 / Checkpoint reference
 * @property resultRef 结果对象引用 / Result object reference
 * @property message 结果消息 / Result message
 * @property extension 扩展字段 / Extension fields
 * @property schemaVersion 报告协议版本 / Report protocol version
 * @property problemStatus 正交问题结论 / Orthogonal problem conclusion
 * @property terminationReason 正交终止原因 / Orthogonal termination reason
 * @property solutionPresence 解存在性 / Solution presence
 * @property proofStatus 证明状态 / Proof status
 * @property provenance 脱敏执行来源 / Redacted execution provenance
 * @property fingerprints 审计指纹 / Audit fingerprints
 * @property fingerprintSchemas 指纹 schema / Fingerprint schemas
 * @property statistics 求解统计 / Solve statistics
 * @property diagnostics 结构化诊断 / Structured diagnostics
 * @property runId 求解运行标识 / Solve run identifier
 * @property attemptId 求解尝试标识 / Solve attempt identifier
 * @property artifactDigest 结果 artifact 摘要 / Result artifact digest
 */
@Serializable
data class SolveResult(
    val feasible: Boolean,
    val optimal: Boolean,
    val objectiveValue: Flt64?,
    val gap: Flt64?,
    @SerialName("elapsedMs")
    @Serializable(with = RemoteSolverMillisecondsDurationSerializer::class)
    val elapsed: Duration,
    val checkpointRef: ObjectRef? = null,
    val resultRef: ObjectRef? = null,
    val message: String? = null,
    val extension: Map<String, String> = emptyMap(),
    val schemaVersion: String = "1.0",
    val problemStatus: RemoteProblemStatus = if (feasible) {
        RemoteProblemStatus.FEASIBLE
    } else {
        RemoteProblemStatus.INFEASIBLE
    },
    val terminationReason: RemoteTerminationReason = RemoteTerminationReason.COMPLETED,
    val solutionPresence: RemoteSolutionPresence = when {
        optimal -> RemoteSolutionPresence.OPTIMAL
        feasible -> RemoteSolutionPresence.INCUMBENT
        else -> RemoteSolutionPresence.NONE
    },
    val proofStatus: RemoteProofStatus = RemoteProofStatus.NONE,
    val provenance: Map<String, String> = emptyMap(),
    val fingerprints: Map<String, String> = emptyMap(),
    val fingerprintSchemas: Map<String, String> = emptyMap(),
    val statistics: Map<String, String> = emptyMap(),
    val diagnostics: Map<String, String> = emptyMap(),
    val runId: String? = null,
    val attemptId: String? = null,
    val artifactDigest: String? = null,
    val objectiveValueInt64: Long? = null
) {
    constructor(
        feasible: Boolean,
        optimal: Boolean,
        objectiveValue: Double?,
        gap: Double?,
        elapsedMs: Long,
        checkpointRef: ObjectRef? = null,
        resultRef: ObjectRef? = null,
        message: String? = null,
        extension: Map<String, String> = emptyMap()
    ) : this(
        feasible = feasible,
        optimal = optimal,
        objectiveValue = objectiveValue?.let { Flt64(it) },
        gap = gap?.let { Flt64(it) },
        elapsed = elapsedMs.toDuration(DurationUnit.MILLISECONDS),
        checkpointRef = checkpointRef,
        resultRef = resultRef,
        message = message,
        extension = extension
    )

    /** 耗时毫秒兼容别名 / Compatibility alias for elapsed milliseconds */
    val elapsedMs: Long get() = elapsed.inWholeMilliseconds
}
