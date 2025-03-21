package fuookami.ospf.kotlin.framework.solver

import org.apache.logging.log4j.kotlin.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.*
import fuookami.ospf.kotlin.core.backend.intermediate_model.*
import fuookami.ospf.kotlin.core.backend.solver.*
import fuookami.ospf.kotlin.core.backend.solver.output.*

class SerialCombinatorialLinearSolver(
    private val solvers: List<Lazy<AbstractLinearSolver>>,
    private val stopErrorCode: Set<ErrorCode> = setOf(ErrorCode.ORModelNoSolution, ErrorCode.ORModelUnbounded)
): AbstractLinearSolver {
    private val logger = logger()

    companion object {
        @JvmName("constructBySolvers")
        operator fun invoke(
            solvers: List<AbstractLinearSolver>,
            stopErrorCode: Set<ErrorCode> = setOf(ErrorCode.ORModelNoSolution, ErrorCode.ORModelUnbounded)
        ): SerialCombinatorialLinearSolver {
            return SerialCombinatorialLinearSolver(solvers.map { lazy { it } }, stopErrorCode)
        }

        @JvmName("constructBySolverExtractors")
        operator fun invoke(
            solvers: List<() -> AbstractLinearSolver>,
            stopErrorCode: Set<ErrorCode> = setOf(ErrorCode.ORModelNoSolution, ErrorCode.ORModelUnbounded)
        ): SerialCombinatorialLinearSolver {
            return SerialCombinatorialLinearSolver(solvers.map { lazy { it() } }, stopErrorCode)
        }
    }

    override val name: String by lazy { "SerialCombinatorial(${solvers.joinToString(",") { it.value.name }})" }

    override suspend operator fun invoke(
        model: LinearTriadModelView,
        statusCallBack: SolvingStatusCallBack?
    ): Ret<SolverOutput> {
        for (solver in solvers) {
            when (val result = solver.value.invoke(model, statusCallBack)) {
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
            }
        }
        return Failed(ErrorCode.SolverNotFound, "No solver valid.")
    }

    override suspend operator fun invoke(
        model: LinearTriadModelView,
        solutionAmount: UInt64,
        statusCallBack: SolvingStatusCallBack?
    ): Ret<Pair<SolverOutput, List<Solution>>> {
        for (solver in solvers) {
            when (val result = solver.value.invoke(model, solutionAmount, statusCallBack)) {
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
            }
        }
        return Failed(ErrorCode.SolverNotFound, "No solver valid.")
    }
}
