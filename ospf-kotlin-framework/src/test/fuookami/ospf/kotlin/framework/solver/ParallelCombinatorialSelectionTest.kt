/**
 * 并行组合求解器选择依据测试。 / Parallel combinatorial solver selection tests.
 */
package fuookami.ospf.kotlin.framework.solver

import kotlinx.coroutines.runBlocking
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.intermediate.BasicLinearTriadModel
import fuookami.ospf.kotlin.core.model.intermediate.LinearConstraintBatch
import fuookami.ospf.kotlin.core.model.intermediate.LinearObjective
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModel
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.model.intermediate.SparseMatrix
import fuookami.ospf.kotlin.core.solver.AbstractLinearSolver
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressContext
import fuookami.ospf.kotlin.core.solver.output.SolvingStatusCallBack
import fuookami.ospf.kotlin.core.solver.report.AuditFingerprint
import fuookami.ospf.kotlin.core.solver.report.ProblemStatus
import fuookami.ospf.kotlin.core.solver.report.ProofStatus
import fuookami.ospf.kotlin.core.solver.report.SolveFingerprints
import fuookami.ospf.kotlin.core.solver.report.SolveProof
import fuookami.ospf.kotlin.core.solver.report.SolveReport
import fuookami.ospf.kotlin.core.solver.report.SolveSelectionReason
import fuookami.ospf.kotlin.core.solver.report.SolveSolution
import fuookami.ospf.kotlin.core.solver.report.SolutionPresence
import fuookami.ospf.kotlin.core.solver.report.SolverCapabilities
import fuookami.ospf.kotlin.core.solver.report.SolverDescriptor
import fuookami.ospf.kotlin.core.solver.report.SolverModelType
import fuookami.ospf.kotlin.core.solver.report.SolverProvenance
import fuookami.ospf.kotlin.core.solver.report.TerminationReason

/**
 * 并行组合求解器选择依据测试。 / Parallel combinatorial solver selection tests.
 */
class ParallelCombinatorialSelectionTest {
    /**
     * First 模式选择第一个成功 backend。
     * First mode selects the first successful backend.
     */
    @Test
    fun firstModeSelectsFirstSuccessfulAttempt() = runBlocking {
        val solver = ParallelCombinatorialLinearSolver(
            listOf(
                ObjectiveStubLinearSolver("first-backend", Flt64(3.0)),
                ObjectiveStubLinearSolver("second-backend", Flt64(1.0))
            ),
            mode = ParallelCombinatorialMode.First
        )

        val result = solver.solveCombinatorialReport(model(ObjectCategory.Minimum))
        val report = when (val unwrapped = result) { is Ok -> unwrapped.value; else -> error("Expected Ok but was $unwrapped") }

        assertEquals("parallel-0", report.selectedAttemptId?.value)
        assertEquals(SolveSelectionReason.FirstFeasible, report.selectionReason)
        assertEquals(Flt64(3.0), report.finalReport?.solution?.objective)
        assertEquals(2, report.attempts.size)
        assertEquals("parallel-0", report.attempts[0].attemptId.value)
        assertEquals("first-backend", report.attempts[0].backendId)
        assertEquals("parallel-1", report.attempts[1].attemptId.value)
        assertEquals("second-backend", report.attempts[1].backendId)
        assertNotNull(report.attempts[0].report?.provenance)
        assertEquals("stub-backend", report.attempts[0].report?.provenance?.descriptor?.backendName)
        assertEquals("stub-model-fingerprint", report.attempts[0].report?.fingerprints?.model?.value)
        assertEquals(report.selectedAttemptId, report.attempts[0].attemptId)
        assertEquals(report.finalReport, report.attempts[0].report)
    }

    /**
     * Best 模式在最小化方向上选择最小目标。
     * Best mode selects the minimum objective for minimization.
     */
    @Test
    fun bestModeSelectsMinimumForMinimization() = runBlocking {
        val solver = ParallelCombinatorialLinearSolver(
            listOf(
                ObjectiveStubLinearSolver("first-backend", Flt64(3.0)),
                ObjectiveStubLinearSolver("second-backend", Flt64(1.0))
            ),
            mode = ParallelCombinatorialMode.Best
        )

        val result = solver.solveCombinatorialReport(model(ObjectCategory.Minimum))
        val report = when (val unwrapped = result) { is Ok -> unwrapped.value; else -> error("Expected Ok but was $unwrapped") }

        assertEquals("parallel-1", report.selectedAttemptId?.value)
        assertEquals(SolveSelectionReason.BestObjective, report.selectionReason)
        assertEquals(Flt64(1.0), report.finalReport?.solution?.objective)
    }

    /**
     * Best 模式在最大化方向上选择最大目标。
     * Best mode selects the maximum objective for maximization.
     */
    @Test
    fun bestModeSelectsMaximumForMaximization() = runBlocking {
        val solver = ParallelCombinatorialLinearSolver(
            listOf(
                ObjectiveStubLinearSolver("first-backend", Flt64(1.0)),
                ObjectiveStubLinearSolver("second-backend", Flt64(3.0))
            ),
            mode = ParallelCombinatorialMode.Best
        )

        val result = solver.solveCombinatorialReport(model(ObjectCategory.Maximum))
        val report = when (val unwrapped = result) { is Ok -> unwrapped.value; else -> error("Expected Ok but was $unwrapped") }

        assertEquals("parallel-1", report.selectedAttemptId?.value)
        assertEquals(SolveSelectionReason.BestObjective, report.selectionReason)
        assertEquals(Flt64(3.0), report.finalReport?.solution?.objective)
    }

    /**
     * 全部 backend 失败时返回空选择依据。
     * No successful attempt yields NoSuccessfulAttempt and a null final report.
     */
    @Test
    fun noSuccessfulAttemptReturnsNullReport() = runBlocking {
        val solver = ParallelCombinatorialLinearSolver(
            listOf(
                FailingStubLinearSolver("first-backend"),
                FailingStubLinearSolver("second-backend")
            ),
            mode = ParallelCombinatorialMode.Best
        )

        val result = solver.solveCombinatorialReport(model(ObjectCategory.Minimum))
        val report = when (val unwrapped = result) { is Ok -> unwrapped.value; else -> error("Expected Ok but was $unwrapped") }

        assertNull(report.selectedAttemptId)
        assertEquals(SolveSelectionReason.NoSuccessfulAttempt, report.selectionReason)
        assertNull(report.finalReport)
    }

    private fun model(category: ObjectCategory): LinearTriadModel {
        return LinearTriadModel(
            impl = BasicLinearTriadModel(
                variables = emptyList(),
                constraints = LinearConstraintBatch(
                    sparseLhs = SparseMatrix(),
                    signs = emptyList(),
                    rhs = emptyList(),
                    names = emptyList(),
                    sources = emptyList()
                ),
                name = "selection-test"
            ),
            tokensInSolver = emptyList(),
            objective = LinearObjective(category, emptyList())
        )
    }

    private class ObjectiveStubLinearSolver(
        override val name: String,
        private val objective: Flt64
    ) : AbstractLinearSolver {
        override suspend fun solveReport(
            model: LinearTriadModelView,
            progressContext: SolverProgressContext?
        ): Ret<SolveReport<Flt64>> {
            return Ok(
                SolveReport(
                    problemStatus = ProblemStatus.Feasible,
                    terminationReason = TerminationReason.Completed,
                    solutionPresence = SolutionPresence.Optimal,
                    solution = SolveSolution(values = emptyList(), objective = objective),
                    proof = SolveProof(ProofStatus.Verified),
                    provenance = SolverProvenance(
                        descriptor = SolverDescriptor(
                            solverId = "stub",
                            backendName = "stub-backend",
                            capabilities = SolverCapabilities(modelTypes = setOf(SolverModelType.LP))
                        ),
                        deterministic = true
                    ),
                    fingerprints = SolveFingerprints(
                        model = AuditFingerprint(
                            schemaVersion = "1.0",
                            algorithm = "sha256",
                            value = "stub-model-fingerprint"
                        )
                    )
                )
            )
        }

        override suspend fun invoke(
            model: LinearTriadModelView,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<FeasibleSolverOutput<Flt64>> {
            return Failed(ErrorCode.ORModelInfeasible)
        }

        override suspend fun invoke(
            model: LinearTriadModelView,
            solutionAmount: UInt64,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
            return Failed(ErrorCode.ORModelInfeasible)
        }
    }

    private class FailingStubLinearSolver(
        override val name: String
    ) : AbstractLinearSolver {
        override suspend fun solveReport(
            model: LinearTriadModelView,
            progressContext: SolverProgressContext?
        ): Ret<SolveReport<Flt64>> {
            return Failed(ErrorCode.ORModelInfeasible)
        }

        override suspend fun invoke(
            model: LinearTriadModelView,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<FeasibleSolverOutput<Flt64>> {
            return Failed(ErrorCode.ORModelInfeasible)
        }

        override suspend fun invoke(
            model: LinearTriadModelView,
            solutionAmount: UInt64,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
            return Failed(ErrorCode.ORModelInfeasible)
        }
    }
}
