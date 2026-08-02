package fuookami.ospf.kotlin.core.solver.report

import kotlin.test.*
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.model.basic.ConstraintSource
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.basic.Objective
import fuookami.ospf.kotlin.core.model.basic.ConstraintRelation as ModelConstraintRelation
import fuookami.ospf.kotlin.core.model.intermediate.BasicLinearTriadModel
import fuookami.ospf.kotlin.core.model.intermediate.BasicQuadraticTetradModel
import fuookami.ospf.kotlin.core.model.intermediate.LinearConstraintBatch
import fuookami.ospf.kotlin.core.model.intermediate.LinearObjectiveCell
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModel
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticObjectiveCell
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModel
import fuookami.ospf.kotlin.core.model.intermediate.SparseMatrix
import fuookami.ospf.kotlin.core.model.intermediate.SparseQuadraticMatrix
import fuookami.ospf.kotlin.core.model.intermediate.SparseQuadraticVector
import fuookami.ospf.kotlin.core.model.intermediate.SparseVector
import fuookami.ospf.kotlin.core.variable.Continuous
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.ok

class SolveReportTest {
    @Test
    fun shouldKeepProblemTerminationAndSolutionPresenceOrthogonal() {
        val report = SolveReport<Double>(
            problemStatus = ProblemStatus.Feasible,
            terminationReason = TerminationReason.TimeLimit,
            solutionPresence = SolutionPresence.Incumbent,
            solution = SolveSolution(
                values = listOf(1.0),
                objective = 1.0
            )
        )

        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(TerminationReason.TimeLimit, report.terminationReason)
        assertEquals(SolutionPresence.Incumbent, report.solutionPresence)
    }

    @Test
    fun shouldCancelOnlyOnceAndKeepFirstCancellationFact() {
        var interruptionCount = 0
        val handle = SolveHandle.create {
            interruptionCount += 1
            ok
        }

        handle.cancel(CancellationSource.Remote, "用户取消")
        handle.cancel(CancellationSource.Timeout, "超时")

        assertEquals(1, interruptionCount)
        assertTrue(handle.token.isCancellationRequested)
        assertEquals(CancellationSource.Remote, handle.token.record?.source)
        assertEquals("用户取消", handle.token.record?.reason)
    }

    @Test
    fun shouldProduceStableCryptographicFingerprints() {
        val first = SolveFingerprinting.configuration(mapOf("threads" to "4", "seed" to "7"))
        val reordered = SolveFingerprinting.configuration(mapOf("seed" to "7", "threads" to "4"))
        val changed = SolveFingerprinting.configuration(mapOf("seed" to "8", "threads" to "4"))

        assertEquals(first, reordered)
        assertNotEquals(first, changed)
        assertEquals("SHA-256", first.algorithm)
        assertEquals(64, first.value.length)
    }

    /**
     * 验证规范化模型的身份元数据参与稳定指纹，并可跨独立重建复现。
     * Verifies normalized identity metadata participates in stable fingerprints and survives independent rebuilds.
     */
    @Test
    fun normalizedModelFingerprintCarriesIdentityMetadata() {
        fun model(origin: ModelElementOrigin): NormalizedMathematicalModel {
            return NormalizedMathematicalModel(
                modelType = SolverModelType.MIP,
                identityNamespace = "stable-model",
                identitySchemaVersion = "1.0",
                variables = listOf(
                    NormalizedVariable(
                        id = VariableId("variable:x"),
                        type = "Integer",
                        scope = ModelElementScope.Stable,
                        origin = origin
                    )
                ),
                constraints = emptyList(),
                objective = NormalizedObjective(
                    id = ObjectiveId("objective:cost"),
                    category = "Minimum",
                    constant = "0",
                    linearTerms = emptyList()
                )
            )
        }

        val first = model(ModelElementOrigin("pipeline", "capacity")).fingerprint()
        val rebuilt = model(ModelElementOrigin("pipeline", "capacity")).fingerprint()
        val changed = model(ModelElementOrigin("pipeline", "other")).fingerprint()

        assertEquals(first, rebuilt)
        assertNotEquals(first, changed)
    }

    @Test
    fun normalizedModelFactoriesPreserveLinearAndQuadraticIdentityMetadata() {
        val variable = fuookami.ospf.kotlin.core.model.basic.Variable(
            index = 0,
            lowerBound = Flt64.zero,
            upperBound = Flt64.one,
            type = Continuous,
            origin = null,
            name = "x",
            id = VariableId("variable:x"),
            identityScope = ModelElementScope.Stable,
            identityOrigin = ModelElementOrigin("pipeline", "x")
        )
        val linearConstraints = LinearConstraintBatch(
            sparseLhs = SparseMatrix(),
            signs = emptyList(),
            rhs = emptyList(),
            names = emptyList(),
            sources = emptyList(),
            identityNamespace = "stable-model",
            identitySchemaVersion = "1.0"
        )
        val linear = LinearTriadModel(
            impl = BasicLinearTriadModel(listOf(variable), linearConstraints, "linear"),
            tokensInSolver = emptyList(),
            objective = Objective(
                category = ObjectCategory.Minimum,
                objective = listOf(LinearObjectiveCell(0, Flt64.one)),
                constant = Flt64(2.0),
                id = ObjectiveId("objective:cost"),
                identityScope = ModelElementScope.Stable,
                identityOrigin = ModelElementOrigin("pipeline", "cost")
            )
        )
        val normalizedLinear = linear.toNormalizedMathematicalModel()
        assertEquals("stable-model", normalizedLinear.identityNamespace)
        assertEquals(ModelElementScope.Stable, normalizedLinear.variables.single().scope)
        assertEquals("variable:x", normalizedLinear.objective.linearTerms.single().variableId.value)

        val quadraticConstraints = fuookami.ospf.kotlin.core.model.intermediate.QuadraticConstraintBatch(
            sparseLhs = SparseQuadraticMatrix().also { it.addRow(SparseQuadraticVector()) },
            signs = listOf(ModelConstraintRelation.Equal),
            rhs = listOf(Flt64.zero),
            names = listOf("c"),
            sources = listOf(ConstraintSource.Origin),
            ids = listOf(ConstraintId("constraint:c")),
            identityNamespace = "stable-model",
            identitySchemaVersion = "1.0"
        )
        val quadratic = QuadraticTetradModel(
            impl = BasicQuadraticTetradModel(listOf(variable), quadraticConstraints, "quadratic"),
            tokensInSolver = emptyList(),
            objective = Objective(
                category = ObjectCategory.Maximum,
                objective = listOf(QuadraticObjectiveCell(0, 0, Flt64.one)),
                id = ObjectiveId("objective:profit")
            )
        )
        val normalizedQuadratic = quadratic.toNormalizedMathematicalModel()
        assertEquals(SolverModelType.QP, normalizedQuadratic.modelType)
        assertEquals("Maximum", normalizedQuadratic.objective.category)
        assertEquals(1, normalizedQuadratic.objective.quadraticTerms.size)
    }

    @Test
    fun canonicalEncodingEscapesNestedTermDelimiters() {
        val model = NormalizedMathematicalModel(
            modelType = SolverModelType.LP,
            variables = listOf(
                NormalizedVariable(VariableId("variable:a,b"), "Continuous")
            ),
            constraints = emptyList(),
            objective = NormalizedObjective(
                id = ObjectiveId("objective:cost"),
                category = "Minimum",
                constant = "0",
                linearTerms = listOf(
                    NormalizedLinearTerm(VariableId("variable:a,b"), "coefficient:c:d")
                )
            )
        )

        val canonical = model.canonicalText()
        assertTrue("variable\\:a\\,b" in canonical)
        assertTrue("coefficient\\:c\\:d" in canonical)
        assertEquals(model.fingerprint(), model.copy().fingerprint())
    }

    @Test
    fun constraintEvaluationReportsSlackViolationAndMissingValues() {
        val linearConstraints = LinearConstraintBatch(
            sparseLhs = SparseMatrix<Flt64>().also {
                it.addRow(SparseVector<Flt64>().also { row -> row.add(0, Flt64.one) })
            },
            signs = listOf(ModelConstraintRelation.LessEqual),
            rhs = listOf(Flt64.one),
            names = listOf("bound"),
            sources = listOf(ConstraintSource.Origin),
            ids = listOf(ConstraintId("constraint:bound"))
        )
        val linear = LinearTriadModel(
            impl = BasicLinearTriadModel(emptyList(), linearConstraints, "evaluation-linear"),
            tokensInSolver = emptyList(),
            objective = Objective(ObjectCategory.Minimum, emptyList())
        )

        val violated = linear.evaluateConstraints(listOf(Flt64(2.0)))
        assertTrue(violated is Ok)
        assertEquals(Flt64(2.0), violated.value.single().lhs)
        assertEquals(Flt64.one, violated.value.single().violation)
        assertFalse(violated.value.single().satisfied)
        assertTrue(linear.evaluateConstraints(emptyList()) is fuookami.ospf.kotlin.utils.functional.Failed)

        val quadraticConstraints = fuookami.ospf.kotlin.core.model.intermediate.QuadraticConstraintBatch(
            sparseLhs = SparseQuadraticMatrix().also {
                it.addRow(SparseQuadraticVector().also { row -> row.add(0, 1, Flt64.one) })
            },
            signs = listOf(ModelConstraintRelation.Equal),
            rhs = listOf(Flt64(6.0)),
            names = listOf("product"),
            sources = listOf(ConstraintSource.Origin),
            ids = listOf(ConstraintId("constraint:product"))
        )
        val quadratic = QuadraticTetradModel(
            impl = BasicQuadraticTetradModel(emptyList(), quadraticConstraints, "evaluation-quadratic"),
            tokensInSolver = emptyList(),
            objective = Objective(ObjectCategory.Minimum, emptyList())
        )
        val satisfied = quadratic.evaluateConstraints(listOf(Flt64(2.0), Flt64(3.0)))
        assertTrue(satisfied is Ok)
        assertEquals(Flt64(6.0), satisfied.value.single().lhs)
        assertTrue(satisfied.value.single().satisfied)
    }
}
