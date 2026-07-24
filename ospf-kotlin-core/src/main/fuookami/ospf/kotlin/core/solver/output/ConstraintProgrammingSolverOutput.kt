/**
 * CP 求解器解、冲突与输出。 / CP solver solutions, conflicts, and outputs.
 */
package fuookami.ospf.kotlin.core.solver.output

import fuookami.ospf.kotlin.core.model.basic.Solution
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.IntervalId
import fuookami.ospf.kotlin.core.model.constraint_programming.IntervalValue
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.EvidenceValidity
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.ProofStatus
import fuookami.ospf.kotlin.core.solver.report.SolveReport
import fuookami.ospf.kotlin.core.solver.report.TerminationReason
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.variable.AbstractVariableItem
import fuookami.ospf.kotlin.core.variable.BinVariable
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok

/** CP 精确整数解。 / Exact integer CP solution. */
data class ConstraintProgrammingSolution(
    val values: Map<VariableId, Int64> = emptyMap(),
    val intervals: Map<IntervalId, IntervalValue> = emptyMap()
) {
    /** 按稳定变量 ID 获取值。 / Get a value by stable variable ID. */
    fun value(id: VariableId): Ret<Int64> {
        return values[id]?.let(::ok) ?: Failed(
            ErrorCode.DataNotFound,
            "缺少 CP 解变量：$id / CP solution does not contain variable: $id"
        )
    }

    /** 按 OSPF 变量获取值。 / Get a value by an OSPF variable. */
    fun value(variable: AbstractVariableItem<*, *>): Ret<Int64> {
        return value(VariableId("${variable.identifier}:${variable.index}"))
    }

    /** 获取二值变量值。 / Get a binary variable value. */
    fun boolean(variable: BinVariable): Ret<Boolean> {
        return value(variable).map { it == Int64.one }
    }

    /** 获取 interval 解。 / Get an interval value. */
    fun interval(id: IntervalId): Ret<IntervalValue> {
        return intervals[id]?.let(::ok) ?: Failed(
            ErrorCode.DataNotFound,
            "缺少 CP 解 interval：$id / CP solution does not contain interval: $id"
        )
    }

    /** 返回线性求解器兼容的值列表。 / Return a linear-solver-compatible value list. */
    fun asList(order: List<VariableId>): Solution<Int64> {
        return order.mapNotNull { values[it] }
    }
}

/** CP conflict 的最小公共表示。 / Minimal backend-neutral CP conflict representation. */
enum class ConstraintProgrammingConflictMinimality {
    /** 未执行必要性检查。 / No irreducibility checks were performed. */
    NotChecked,

    /** 所有候选均通过删除复验。 / Every remaining candidate passed deletion checks. */
    Irreducible,

    /** 至少一次删除复验未能得到证明终态。 / At least one deletion check lacked a proven terminal state. */
    Partial
}

/** CP conflict 的最小公共表示。 / Minimal backend-neutral CP conflict representation. */
data class ConstraintProgrammingConflict(
    val constraintIds: Set<ConstraintId> = emptySet(),
    val variableIds: Set<VariableId> = emptySet(),
    val message: String? = null,
    val assumptions: List<BooleanLiteral> = emptyList(),
    val minimality: ConstraintProgrammingConflictMinimality = ConstraintProgrammingConflictMinimality.NotChecked,
    /** 原始模型成员；不包含 solver 辅助约束。 / Original model members, excluding solver auxiliaries. */
    val members: Set<InfeasibilityMember> = emptySet(),
    /** activation 的稳定 ID。 / Stable activation IDs. */
    val activationIds: Set<String> = emptySet(),
    /** activation 到原始成员的稳定投影。 / Stable activation-to-member projection. */
    val activationMembers: Map<String, InfeasibilityMember> = emptyMap(),
    /** 当前 conflict 成员集合的不可行性复验状态。 / Verification status of the current conflict set. */
    val validity: EvidenceValidity = EvidenceValidity.Verified,
    /** 删除复验次数。 / Number of deletion checks. */
    val verificationChecks: UInt64? = null,
    /** 最终复验或删除复验的终止原因。 / Termination reason of verification. */
    val terminationReason: TerminationReason? = null
)

/** CP 求解输出。 / CP solver output. */
sealed interface ConstraintProgrammingSolverOutput : SolverOutput {
    /** 统一求解报告。 / Unified solve report. */
    val report: SolveReport<Int64>?
}

/** CP 可行/最优输出。 / CP feasible/optimal output. */
data class ConstraintProgrammingFeasibleOutput(
    val solution: ConstraintProgrammingSolution,
    val objective: Flt64? = null,
    val bestBound: Flt64? = null,
    val status: SolverStatus = SolverStatus.Optimal,
    val proofStatus: ProofStatus = ProofStatus.None,
    override val report: SolveReport<Int64>? = null
) : ConstraintProgrammingSolverOutput

/** CP 已证明不可行输出。 / CP proven-infeasible output. */
data class ConstraintProgrammingInfeasibleOutput(
    val conflict: ConstraintProgrammingConflict? = null,
    val proofStatus: ProofStatus = ProofStatus.Verified,
    override val report: SolveReport<Int64>? = null
) : ConstraintProgrammingSolverOutput

/** CP 未知/达到限制输出。 / CP unknown/limit output. */
data class ConstraintProgrammingUnknownOutput(
    val terminationReason: TerminationReason,
    override val report: SolveReport<Int64>? = null
) : ConstraintProgrammingSolverOutput
