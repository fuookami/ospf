package fuookami.ospf.kotlin.core.solver

import kotlin.test.*
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.QuadraticInequalityOf
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModel
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModelView
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput
import fuookami.ospf.kotlin.core.solver.output.SolvingStatusCallBack
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.ModelElementIdentityRegistry
import fuookami.ospf.kotlin.core.solver.report.ModelElementScope
import fuookami.ospf.kotlin.core.solver.report.ObjectiveId
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.token.QuadraticFlattenData
import fuookami.ospf.kotlin.core.variable.RealVar

/**
 * 二次机制模型稳定身份传播回归测试。 / Quadratic mechanism model stable-identity propagation regression tests.
 */
class QuadraticMetaModelDumpIdentityPropagationTest {
    /**
     * Verify that an explicit identity registry propagates from a quadratic meta model to the tetrad dump.
     * 验证显式身份注册表从二次元模型传播到四元组转储。
     */
    @Test
    fun identityRegistryShouldPropagateFromQuadraticMetaModelToTetradDump() = runBlocking {
        val registry = ModelElementIdentityRegistry(
            namespace = "quadratic-identity-propagation",
            schemaVersion = "1.0"
        )
        val x = RealVar("identity-quad-x")
        val metaModel = QuadraticMetaModel(
            name = "identity-propagation-quadratic-model",
            identityRegistry = registry
        )

        try {
            assertTrue(metaModel.add(x) is Ok)
            val constraint = QuadraticInequalityOf(
                lhs = QuadraticPolynomial(
                    monomials = listOf(QuadraticMonomial.linear(Flt64.one, x)),
                    constant = Flt64.zero
                ),
                rhs = QuadraticPolynomial(emptyList(), Flt64.one),
                comparison = Comparison.LE
            )
            assertTrue(
                metaModel.addConstraint(relation = constraint, name = "identity-quadratic-constraint") is Ok
            )
            assertTrue(
                metaModel.addObject(
                    category = ObjectCategory.Minimum,
                    flattenData = QuadraticFlattenData(
                        monomials = listOf(QuadraticMonomial.linear(Flt64.one, x)),
                        constant = Flt64.zero
                    ),
                    name = "identity-quadratic-objective"
                ) is Ok
            )
            assertTrue(registry.registerVariable(x, VariableId("source:identity-quad-x")) is Ok)
            assertTrue(
                registry.registerConstraint(
                    metaModel.constraints.single(),
                    ConstraintId("source:identity-quad-constraint")
                ) is Ok
            )
            assertTrue(
                registry.registerObjective(
                    metaModel.flattenSubObjects.single(),
                    ObjectiveId("source:identity-quad-objective")
                ) is Ok
            )

            val solver = DumpOnlyQuadraticSolver()
            val mechanism = (solver.dump(metaModel, null, null) as Ok).value
            val tetrad = QuadraticTetradModel(
                model = mechanism,
                dumpConstraintsToBounds = false
            )

            assertEquals("quadratic-identity-propagation", tetrad.constraints.identityNamespace)
            assertEquals("source:identity-quad-x", tetrad.variables.single().id?.value)
            assertEquals("source:identity-quad-constraint", tetrad.constraints.ids.single().value)
            assertEquals(ModelElementScope.Stable, tetrad.constraints.identityScopeAt(0))
            assertEquals("source:identity-quad-objective", tetrad.objective.id?.value)
            assertTrue(tetrad.identityValidation is Ok)
            mechanism.close()
        } finally {
            metaModel.close()
        }
    }
}

/** Dump-only quadratic solver used to build the mechanism model without solving. / 仅转储的二次求解器。 */
private class DumpOnlyQuadraticSolver : AbstractQuadraticSolver {
    override val name: String = "dump-only-quadratic"

    override suspend fun invoke(
        model: QuadraticTetradModelView,
        solvingStatusCallBack: SolvingStatusCallBack?
    ): Ret<FeasibleSolverOutput<Flt64>> {
        fail("DumpOnlyQuadraticSolver should not solve a model")
    }

    override suspend fun invoke(
        model: QuadraticTetradModelView,
        solutionAmount: UInt64,
        solvingStatusCallBack: SolvingStatusCallBack?
    ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
        fail("DumpOnlyQuadraticSolver should not solve a model")
    }
}
