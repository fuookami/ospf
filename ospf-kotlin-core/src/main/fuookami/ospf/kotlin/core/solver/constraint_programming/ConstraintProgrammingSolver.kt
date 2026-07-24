/**
 * CP solver/session/options SPI。 / CP solver, session, and options SPI.
 */
package fuookami.ospf.kotlin.core.solver.constraint_programming

import kotlin.time.Duration
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressContext
import fuookami.ospf.kotlin.core.solver.report.CancellationToken
import fuookami.ospf.kotlin.core.solver.report.BackendConfiguration
import fuookami.ospf.kotlin.core.solver.report.SolverDescriptor
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.utils.functional.Ret

/** CP 求解选项。 / CP solve options. */
data class ConstraintProgrammingSolveOptions(
    val timeLimit: Duration? = null,
    val nodeLimit: UInt64? = null,
    val solutionLimit: UInt64? = null,
    val randomSeed: Long? = null,
    val cancellationToken: CancellationToken? = null,
    val progressContext: SolverProgressContext? = null,
    val collectConflict: Boolean = false,
    val shrinkConflict: Boolean = false,
    val conflictShrinkLimit: UInt64? = null,
    /**
     * 诊断复验时强制激活的原始成员 ID；为空表示全部激活。 / / Original-member activation IDs forced on during diagnostic verification; null means all.
     */
    val conflictActivationIds: Set<String>? = null,
    val deterministic: Boolean = false,
    val relativeObjectiveGap: Flt64? = null,
    val absoluteObjectiveGap: Flt64? = null,
    val logEnabled: Boolean = false,
    val backendConfiguration: BackendConfiguration? = null
)

/**
 * CP session，允许在同一模型上重复使用编译计划。 / CP session for reusing a compiled plan on one model.
 */
interface ConstraintProgrammingSession : AutoCloseable {
    /** 关联的模型。 / Associated model. */
    val model: ConstraintProgrammingModel

    /** 会话选项。 / Session options. */
    val options: ConstraintProgrammingSolveOptions

    /** 会话是否已关闭。 / Whether the session is closed. */
    val isClosed: Boolean

    /**
     * 求解一次；assumptions 只改变本次求解的激活文字。 / Solve once; assumptions affect only this solve.
     *
     * @param assumptions 激活文字 / Assumption literals
     * @param fixedValues 本轮必须固定的整数变量值 / Integer values fixed for this solve
     * @param hints 可选整数解提示 / Optional integer solution hint
     * @return CP 输出或结构化错误 / CP output or a structured error
     */
    suspend fun solve(
        assumptions: List<BooleanLiteral> = emptyList(),
        fixedValues: Map<VariableId, Int64> = emptyMap(),
        hints: ConstraintProgrammingSolution? = null
    ): Ret<ConstraintProgrammingSolverOutput>
}

/** CP 求解器通用接口。 / Common CP solver interface. */
interface ConstraintProgrammingSolver {
    /** 求解器描述符。 / Solver descriptor. */
    val descriptor: SolverDescriptor

    /** 一次性求解。 / One-shot solve. */
    suspend fun solve(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions = ConstraintProgrammingSolveOptions()
    ): Ret<ConstraintProgrammingSolverOutput>

    /** 创建可复用 session。 / Create a reusable session. */
    fun createSession(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions = ConstraintProgrammingSolveOptions()
    ): Ret<ConstraintProgrammingSession>
}
