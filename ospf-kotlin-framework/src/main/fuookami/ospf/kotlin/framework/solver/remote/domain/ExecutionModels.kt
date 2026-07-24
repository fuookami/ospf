@file:OptIn(kotlin.time.ExperimentalTime::class)
package fuookami.ospf.kotlin.framework.solver.remote.domain

import kotlin.time.Duration
import kotlin.time.Instant
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/**
 * 求解器类型。 / Solver type.
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

/** 远程问题结论 / Remote problem conclusion */
@Serializable
enum class RemoteProblemStatus {
    FEASIBLE,
    INFEASIBLE,
    UNBOUNDED,
    INFEASIBLE_OR_UNBOUNDED,
    UNKNOWN
}

/** 远程终止原因 / Remote termination reason */
@Serializable
enum class RemoteTerminationReason {
    COMPLETED,
    TIME_LIMIT,
    NODE_LIMIT,
    ITERATION_LIMIT,
    SOLUTION_LIMIT,
    OBJECTIVE_LIMIT,
    CANCELLED,
    INTERRUPTED,
    NUMERICAL_FAILURE,
    BACKEND_FAILURE
}

/** 远程解存在性 / Remote solution presence */
@Serializable
enum class RemoteSolutionPresence {
    NONE,
    INCUMBENT,
    OPTIMAL
}

/**
 * 执行句柄。 / Execution handle.
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
)

/**
 * 切片结果。 / Slice result.
 *
 * @property sliceId 切片 ID / Slice ID
 * @property completed 是否完成 / Whether completed
 * @property feasible 是否可行 / Whether feasible
 * @property objectiveValue 目标值 / Objective value
 * @property gap 最优间隙 / Optimality gap
 * @property elapsed 耗时 / Elapsed
 * @property message 结果消息 / Result message
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
    val message: String? = null
)

/**
 * 求解结果。 / Solve result.
 *
 * @property feasible 是否可行 / Whether feasible
 * @property optimal 是否最优 / Whether optimal
 * @property objectiveValue 目标值 / Objective value
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
 * @property provenance 脱敏执行来源 / Redacted execution provenance
 * @property fingerprints 审计指纹 / Audit fingerprints
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
    val provenance: Map<String, String> = emptyMap(),
    val fingerprints: Map<String, String> = emptyMap()
)
