/**
 * 串行组合线性求解器 / Serial Combinatorial Linear Solver
 *
 * 将多个线性求解器串行运行，第一个成功即返回。 / Runs multiple linear solvers serially, returning on first success.
*/
package fuookami.ospf.kotlin.framework.solver

import org.apache.logging.log4j.kotlin.logger
import fuookami.ospf.kotlin.core.error.SolverNotFoundError
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.solver.AbstractLinearSolver
import fuookami.ospf.kotlin.core.solver.output.*
import fuookami.ospf.kotlin.core.solver.report.*
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressContext
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.*

/**
 * 串行组合线性求解器 / Serial combinatorial linear solver
 *
 * @property solvers 线性求解器列表（懒加载） / Linear solver list (lazy loaded)
 * @property stopErrorCode 遇到即停止的错误码 / Error codes that stop execution
*/
class SerialCombinatorialLinearSolver(
    private val solvers: List<Lazy<AbstractLinearSolver>>,
    private val stopErrorCode: Set<ErrorCode> = setOf(ErrorCode.ORModelInfeasible, ErrorCode.ORModelUnbounded)
) : AbstractLinearSolver {
    private val logger = logger()

    companion object {
        @JvmName("constructBySolvers")
        operator fun invoke(
            solvers: List<AbstractLinearSolver>,
            stopErrorCode: Set<ErrorCode> = setOf(ErrorCode.ORModelInfeasible, ErrorCode.ORModelUnbounded)
        ): SerialCombinatorialLinearSolver {
            return SerialCombinatorialLinearSolver(solvers.map { lazy { it } }, stopErrorCode)
        }

        @JvmName("constructBySolverExtractors")
        operator fun invoke(
            solvers: List<() -> AbstractLinearSolver>,
            stopErrorCode: Set<ErrorCode> = setOf(ErrorCode.ORModelInfeasible, ErrorCode.ORModelUnbounded)
        ): SerialCombinatorialLinearSolver {
            return SerialCombinatorialLinearSolver(solvers.map { lazy { it() } }, stopErrorCode)
        }
    }

    override val name: String by lazy { "SerialCombinatorial(${solvers.joinToString(",") { it.value.name }})" }

    /**
     * 求解并保留每个 backend 的完整尝试轨迹。 / Solve while preserving every backend attempt trace.
     *
     * @param model 线性模型 / Linear model
     * @param progressContext 进度上下文 / Progress context
     * @return 组合求解报告 / Combinatorial solve report
     */
    suspend fun solveCombinatorialReport(
        model: LinearTriadModelView,
        progressContext: SolverProgressContext? = null
    ): Ret<CombinatorialSolveReport<Flt64>> {
        val attempts = mutableListOf<SolveAttemptTrace<Flt64>>()
        for ((index, lazySolver) in solvers.withIndex()) {
            val solver = lazySolver.value
            val attemptId = SolveAttemptId("serial-$index")
            when (val result = solver.solveReport(model, progressContext)) {
                is Ok -> {
                    attempts += SolveAttemptTrace(
                        attemptId = attemptId,
                        backendId = solver.name,
                        report = result.value
                    )
                    return Ok(
                        CombinatorialSolveReport(
                            finalReport = result.value,
                            attempts = attempts,
                            selectedAttemptId = attemptId,
                            selectionReason = SolveSelectionReason.FirstFeasible
                        )
                    )
                }
                is Failed -> {
                    attempts += SolveAttemptTrace(
                        attemptId = attemptId,
                        backendId = solver.name,
                        errors = listOf(
                            SolveIssue(
                                code = result.error.code.toString(),
                                category = SolveIssueCategory.Backend,
                                message = result.error.message
                            )
                        )
                    )
                    if (stopErrorCode.contains(result.error.code)) {
                        break
                    }
                }
                is Fatal -> {
                    attempts += SolveAttemptTrace(
                        attemptId = attemptId,
                        backendId = solver.name,
                        errors = result.errors.map { error ->
                            SolveIssue(
                                code = error.code.toString(),
                                category = SolveIssueCategory.Backend,
                                message = error.message
                            )
                        }
                    )
                    break
                }
            }
        }
        return Ok(
            CombinatorialSolveReport(
                finalReport = null,
                attempts = attempts,
                selectedAttemptId = null,
                selectionReason = SolveSelectionReason.NoSuccessfulAttempt
            )
        )
    }

    override suspend operator fun invoke(
        model: LinearTriadModelView,
        solvingStatusCallBack: SolvingStatusCallBack?
    ): Ret<FeasibleSolverOutput<Flt64>> {
        for (solver in solvers) {
            when (val result = solver.value.invoke(model, solvingStatusCallBack)) {
                is Ok -> {
                    return Ok(result.value)
                }

                is Failed -> {
                    if (stopErrorCode.contains(result.error.code)) {
                        return Failed(result.error.code, result.error.message)
                    } else {
                        logger.warn { "Solver ${solver.value.name} failed with error ${result.error.code}: ${result.error.message}" }
                    }
                }

                is Fatal -> {
                    return Fatal(result.errors)
                }
            }
        }
        return Failed(SolverNotFoundError())
    }

    override suspend operator fun invoke(
        model: LinearTriadModelView,
        solutionAmount: UInt64,
        solvingStatusCallBack: SolvingStatusCallBack?
    ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
        for (solver in solvers) {
            when (val result = solver.value.invoke(model, solutionAmount, solvingStatusCallBack)) {
                is Ok -> {
                    return Ok(result.value)
                }

                is Failed -> {
                    if (stopErrorCode.contains(result.error.code)) {
                        return Failed(result.error.code, result.error.message)
                    } else {
                        logger.warn { "Solver ${solver.value.name} failed with error ${result.error.code}: ${result.error.message}" }
                    }
                }

                is Fatal -> {
                    return Fatal(result.errors)
                }
            }
        }
        return Failed(SolverNotFoundError())
    }
}


