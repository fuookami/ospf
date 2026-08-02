/**
 * 组合求解器身份门禁测试。 / Combinatorial solver identity gate tests.
 *
 * 验证串行/并行组合求解器在模型身份校验失败时返回结构化错误，
 * 且不会启动任何 backend 尝试。 / Verifies that serial and parallel combinatorial
 * solvers return a structured error when model identity validation fails,
 * without starting any backend attempt.
 */
package fuookami.ospf.kotlin.framework.solver

import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.runBlocking
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.intermediate.BasicLinearTriadModel
import fuookami.ospf.kotlin.core.model.intermediate.BasicQuadraticTetradModel
import fuookami.ospf.kotlin.core.model.intermediate.LinearConstraintBatch
import fuookami.ospf.kotlin.core.model.intermediate.LinearObjective
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModel
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticConstraintBatch
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticObjective
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModel
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModelView
import fuookami.ospf.kotlin.core.model.intermediate.SparseMatrix
import fuookami.ospf.kotlin.core.model.intermediate.SparseQuadraticMatrix
import fuookami.ospf.kotlin.core.solver.AbstractLinearSolver
import fuookami.ospf.kotlin.core.solver.AbstractQuadraticSolver
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput
import fuookami.ospf.kotlin.core.solver.output.SolvingStatusCallBack

/**
 * 组合求解器身份门禁测试。 / Combinatorial solver identity gate tests.
 */
class CombinatorialSolverIdentityTest {
    /**
     * 串行组合线性求解器在身份校验失败时不启动 backend。
     * Serial combinatorial linear solver must not start a backend when identity validation fails.
     */
    @Test
    fun serialLinearSolverPropagatesIdentityFailureWithoutInvokingBackend() = runBlocking {
        val backend = RecordingLinearSolver()
        val solver = SerialCombinatorialLinearSolver(listOf(backend))

        val result = solver.invoke(linearModelWithFailedIdentity(), null)

        assertTrue(result is Failed)
        assertEquals(0, backend.calls.get())
    }

    /**
     * 串行组合二次求解器在身份校验失败时不启动 backend。
     * Serial combinatorial quadratic solver must not start a backend when identity validation fails.
     */
    @Test
    fun serialQuadraticSolverPropagatesIdentityFailureWithoutInvokingBackend() = runBlocking {
        val backend = RecordingQuadraticSolver()
        val solver = SerialCombinatorialQuadraticSolver(listOf(backend))

        val result = solver.invoke(quadraticModelWithFailedIdentity(), null)

        assertTrue(result is Failed)
        assertEquals(0, backend.calls.get())
    }

    /**
     * 并行组合线性求解器在身份校验失败时不启动 backend，且返回组合报告错误。
     * Parallel combinatorial linear solver must not start a backend when identity validation fails.
     */
    @Test
    fun parallelLinearSolverPropagatesIdentityFailureWithoutInvokingBackend() = runBlocking {
        val backend = RecordingLinearSolver()
        val solver = ParallelCombinatorialLinearSolver(listOf(backend))

        val reportResult = solver.solveCombinatorialReport(linearModelWithFailedIdentity())
        assertTrue(reportResult is Failed)
        assertEquals(0, backend.calls.get())

        val invokeResult = solver.invoke(linearModelWithFailedIdentity(), null)
        assertTrue(invokeResult is Failed)
        assertEquals(0, backend.calls.get())
    }

    /**
     * 并行组合二次求解器在身份校验失败时不启动 backend，且返回组合报告错误。
     * Parallel combinatorial quadratic solver must not start a backend when identity validation fails.
     */
    @Test
    fun parallelQuadraticSolverPropagatesIdentityFailureWithoutInvokingBackend() = runBlocking {
        val backend = RecordingQuadraticSolver()
        val solver = ParallelCombinatorialQuadraticSolver(listOf(backend))

        val invokeResult = solver.invoke(quadraticModelWithFailedIdentity(), null)
        assertTrue(invokeResult is Failed)
        assertEquals(0, backend.calls.get())
    }

    private class RecordingLinearSolver : AbstractLinearSolver {
        val calls = AtomicInteger(0)
        override val name: String = "recording-linear"

        override suspend fun invoke(
            model: LinearTriadModelView,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<FeasibleSolverOutput<Flt64>> {
            calls.incrementAndGet()
            return Failed(ErrorCode.ORModelInfeasible)
        }

        override suspend fun invoke(
            model: LinearTriadModelView,
            solutionAmount: UInt64,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
            calls.incrementAndGet()
            return Failed(ErrorCode.ORModelInfeasible)
        }
    }

    private class RecordingQuadraticSolver : AbstractQuadraticSolver {
        val calls = AtomicInteger(0)
        override val name: String = "recording-quadratic"

        override suspend fun invoke(
            model: QuadraticTetradModelView,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<FeasibleSolverOutput<Flt64>> {
            calls.incrementAndGet()
            return Failed(ErrorCode.ORModelInfeasible)
        }

        override suspend fun invoke(
            model: QuadraticTetradModelView,
            solutionAmount: UInt64,
            solvingStatusCallBack: SolvingStatusCallBack?
        ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
            calls.incrementAndGet()
            return Failed(ErrorCode.ORModelInfeasible)
        }
    }

    private fun linearModelWithFailedIdentity(): LinearTriadModel {
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
                name = "invalid-identity-linear"
            ),
            tokensInSolver = emptyList(),
            objective = LinearObjective(ObjectCategory.Minimum, emptyList()),
            identityValidation = Failed(ErrorCode.IllegalArgument, "invalid identity")
        )
    }

    private fun quadraticModelWithFailedIdentity(): QuadraticTetradModel {
        return QuadraticTetradModel(
            impl = BasicQuadraticTetradModel(
                variables = emptyList(),
                constraints = QuadraticConstraintBatch(
                    sparseLhs = SparseQuadraticMatrix(),
                    signs = emptyList(),
                    rhs = emptyList(),
                    names = emptyList(),
                    sources = emptyList()
                ),
                name = "invalid-identity-quadratic"
            ),
            tokensInSolver = emptyList(),
            objective = QuadraticObjective(ObjectCategory.Minimum, emptyList()),
            identityValidation = Failed(ErrorCode.IllegalArgument, "invalid identity")
        )
    }
}
