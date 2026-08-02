package fuookami.ospf.kotlin.core.solver.report

import java.util.concurrent.Executors
import kotlinx.coroutines.runBlocking
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import org.junit.jupiter.api.Test
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.model.basic.*
import fuookami.ospf.kotlin.core.model.basic.ConstraintRelation as ModelConstraintRelation
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.variable.Continuous

class ModelElementIdentityRegistryTest {
    @Test
    fun registryShouldKeepTypedIdentitiesAndRejectDuplicateIds() {
        val registry = ModelElementIdentityRegistry(namespace = "test-model", schemaVersion = "1.0")
        val variable = Any()
        val constraint = Any()
        val objective = Any()

        assertTrue(registry.registerVariable(variable, VariableId("variable:x")) is Ok)
        assertTrue(registry.registerConstraint(constraint, ConstraintId("constraint:c")) is Ok)
        assertTrue(registry.registerObjective(objective, ObjectiveId("objective:z")) is Ok)
        assertTrue(registry.validate() is Ok)
        assertEquals("variable:x", registry.variableId(variable, 9).value)
        assertEquals("constraint:c", registry.constraintId(constraint, 9).value)
        assertEquals("objective:z", registry.objectiveId(objective, 9).value)
        assertEquals(3, registry.entries().size)

        assertTrue(registry.registerVariable(Any(), VariableId("variable:x")) is Failed)
        assertTrue(registry.registerVariable(variable, VariableId("variable:other")) is Failed)
    }

    @Test
    fun unregisteredElementsShouldUseExplicitModelLocalFallbacks() {
        val registry = ModelElementIdentityRegistry()
        val element = Any()

        assertEquals("model-local-variable:4", registry.variableId(element, 4).value)
        assertEquals("model-local-constraint:4", registry.constraintId(element, 4).value)
        assertEquals("model-local-objective:4", registry.objectiveId(element, 4).value)
        assertNull(registry.identity(element))
    }

    @Test
    fun fallbackCollisionsAndReservedStableIdsAreRejected() {
        val registry = ModelElementIdentityRegistry()
        val first = Any()
        val second = Any()

        val firstId = registry.variableId(first, 0)
        val secondId = registry.variableId(second, 0)
        assertTrue(firstId != secondId)
        assertTrue(registry.validate() is Ok)
        assertTrue(registry.registerVariable(Any(), VariableId("model-local-variable:0")) is Failed)
    }

    @Test
    fun explicitRegistrationReplacesAnEarlierModelLocalFallback() {
        val registry = ModelElementIdentityRegistry()
        val variable = Any()

        assertEquals("model-local-variable:0", registry.variableId(variable, 0).value)
        assertTrue(
            registry.registerVariable(
                variable,
                VariableId("stable:variable"),
                ModelElementOrigin("pipeline", "variable")
            ) is Ok
        )

        assertEquals("stable:variable", registry.variableId(variable, 0).value)
        assertEquals(1, registry.entries().size)
        assertTrue(registry.validate() is Ok)
    }

    @Test
    fun failedStableRegistrationMustNotDiscardAnEarlierFallback() {
        val registry = ModelElementIdentityRegistry()
        val first = Any()
        val second = Any()
        val fallback = registry.variableId(first, 0)

        assertTrue(registry.registerVariable(second, VariableId("stable:conflict")) is Ok)
        assertTrue(registry.registerVariable(first, VariableId("stable:conflict")) is Failed)
        assertEquals(fallback, registry.variableId(first, 0))
        assertTrue(registry.validate() is Ok)
    }

    @Test
    fun blankNamespaceOrSchemaMustBeRejected() {
        assertTrue(ModelElementIdentityRegistry(namespace = "", schemaVersion = "1.0").validate() is Failed)
        assertTrue(ModelElementIdentityRegistry(namespace = "model", schemaVersion = "").validate() is Failed)
    }

    /**
     * 验证机制模型工厂在展开前传播身份注册表错误。
     * Verifies that mechanism factories propagate registry errors before model expansion.
     */
    @Test
    fun mechanismFactoriesRejectInvalidIdentityRegistryBeforeExpansion() = runBlocking {
        val invalidRegistry = ModelElementIdentityRegistry(namespace = "", schemaVersion = "1.0")
        val linear = LinearMetaModel(
            name = "invalid-linear-identity",
            converter = fuookami.ospf.kotlin.core.test.flt64TestConverter,
            identityRegistry = invalidRegistry
        )
        val linearResult = LinearMechanismModel(linear, concurrent = false, blocking = true)
        assertTrue(linearResult is Failed)
        linear.close()

        val quadratic = QuadraticMetaModel(
            name = "invalid-quadratic-identity",
            converter = fuookami.ospf.kotlin.core.test.flt64TestConverter,
            identityRegistry = invalidRegistry
        )
        val quadraticResult = QuadraticMechanismModel(quadratic, concurrent = false, blocking = true)
        assertTrue(quadraticResult is Failed)
        quadratic.close()
    }

    @Test
    fun concurrentFallbackResolutionMustRemainUniqueAndValid() {
        val registry = ModelElementIdentityRegistry(namespace = "parallel-model", schemaVersion = "1.0")
        val elements = List(256) { Any() }
        val executor = Executors.newFixedThreadPool(8)
        try {
            val futures = elements.map { element ->
                executor.submit<String> { registry.variableId(element, 0).value }
            }
            val ids = futures.map { it.get() }.toSet()
            assertEquals(elements.size, ids.size)
            assertTrue(ids.all { it.startsWith("model-local-variable:0") })
            assertTrue(registry.validate() is Ok)
        } finally {
            executor.shutdownNow()
        }
    }

    @Test
    fun stableIdsMustBeUniqueAcrossElementKinds() {
        val registry = ModelElementIdentityRegistry()
        assertTrue(registry.registerVariable(Any(), VariableId("shared:id")) is Ok)
        assertTrue(registry.registerConstraint(Any(), ConstraintId("shared:id")) is Failed)
        assertTrue(registry.registerObjective(Any(), ObjectiveId("shared:id")) is Failed)
    }

    /**
     * 验证独立模型重建只要使用相同显式身份就保持相同元数据。
     * Verifies that independent model rebuilds preserve identity metadata when explicit identities match.
     */
    @Test
    fun independentRebuildsPreserveExplicitIdentityMetadata() {
        val firstRegistry = ModelElementIdentityRegistry(namespace = "stable-model", schemaVersion = "1.0")
        val secondRegistry = ModelElementIdentityRegistry(namespace = "stable-model", schemaVersion = "1.0")
        val firstVariable = Any()
        val secondVariable = Any()
        val firstOrigin = ModelElementOrigin("pipeline", "capacity")
        val secondOrigin = ModelElementOrigin("pipeline", "capacity")

        assertTrue(firstRegistry.registerVariable(firstVariable, VariableId("variable:capacity"), firstOrigin) is Ok)
        assertTrue(secondRegistry.registerVariable(secondVariable, VariableId("variable:capacity"), secondOrigin) is Ok)
        assertTrue(firstRegistry.validate() is Ok)
        assertTrue(secondRegistry.validate() is Ok)

        val first = firstRegistry.identity(firstVariable)!!
        val second = secondRegistry.identity(secondVariable)!!
        assertEquals(first.id, second.id)
        assertEquals(first.scope, second.scope)
        assertEquals(first.origin, second.origin)
        assertEquals(firstRegistry.namespace, secondRegistry.namespace)
        assertEquals(firstRegistry.schemaVersion, secondRegistry.schemaVersion)
    }

    @Test
    fun variableDiagnosticIdShouldPreferExplicitStableId() {
        val variable = Variable(
            index = 0,
            lowerBound = Flt64.zero,
            upperBound = Flt64.one,
            type = Continuous,
            origin = null,
            name = "x",
            id = VariableId("stable:x")
        )

        assertEquals("stable:x", variable.diagnosticVariableId().value)
    }

    @Test
    fun linearAndQuadraticDiagnosticIdsShouldUseBatchIds() {
        val linearBatch = LinearConstraintBatch(
            sparseLhs = SparseMatrix<Flt64>().also { matrix ->
                matrix.addRow(SparseVector<Flt64>())
            },
            signs = listOf(ModelConstraintRelation.Equal),
            rhs = listOf(Flt64.zero),
            names = listOf("duplicate-display-name"),
            sources = listOf(ConstraintSource.Origin),
            ids = listOf(ConstraintId("stable:linear-row"))
        )
        val linear = LinearTriadModel(
            impl = BasicLinearTriadModel(emptyList(), linearBatch, "linear"),
            tokensInSolver = emptyList(),
            objective = Objective(ObjectCategory.Minimum, emptyList())
        )

        val quadraticBatch = QuadraticConstraintBatch(
            sparseLhs = SparseQuadraticMatrix().also { matrix ->
                matrix.addRow(SparseQuadraticVector())
            },
            signs = listOf(ModelConstraintRelation.Equal),
            rhs = listOf(Flt64.zero),
            names = listOf("duplicate-display-name"),
            sources = listOf(ConstraintSource.Origin),
            ids = listOf(ConstraintId("stable:quadratic-row"))
        )
        val quadratic = QuadraticTetradModel(
            impl = BasicQuadraticTetradModel(emptyList(), quadraticBatch, "quadratic"),
            tokensInSolver = emptyList(),
            objective = Objective(ObjectCategory.Minimum, emptyList())
        )

        assertEquals("stable:linear-row", linear.diagnosticConstraintId(0).value)
        assertEquals("stable:quadratic-row", quadratic.diagnosticConstraintId(0).value)
    }

    @Test
    fun constraintBatchShouldPreserveIdentityScopeAndOriginAcrossCopies() {
        val origin = ModelElementOrigin("pipeline", "capacity")
        val batch = LinearConstraintBatch(
            sparseLhs = SparseMatrix<Flt64>().also { matrix ->
                matrix.addRow(SparseVector<Flt64>())
                matrix.addRow(SparseVector<Flt64>())
            },
            signs = listOf(ModelConstraintRelation.Equal, ModelConstraintRelation.LessEqual),
            rhs = listOf(Flt64.zero, Flt64.one),
            names = listOf("a", "b"),
            sources = listOf(ConstraintSource.Origin, ConstraintSource.Origin),
            ids = listOf(ConstraintId("stable:a"), ConstraintId("stable:b")),
            identityScopes = listOf(ModelElementScope.Stable, ModelElementScope.ModelLocal),
            identityOrigins = listOf(origin, null)
        )

        val filtered = batch.filter { it == 0 }
        val copied = batch.copy()
        assertEquals(ModelElementScope.Stable, filtered.identityScopeAt(0))
        assertEquals(origin, filtered.identityOriginAt(0))
        assertEquals(ModelElementScope.ModelLocal, copied.identityScopeAt(1))
        assertEquals(null, copied.identityOriginAt(1))
    }

    @Test
    fun feasibilityArtifactsShouldCarryIndependentIdentityMetadata() {
        val sourceOrigin = ModelElementOrigin("pipeline", "capacity")
        val sourceVariable = Variable(
            index = 0,
            lowerBound = Flt64.zero,
            upperBound = Flt64.one,
            type = Continuous,
            origin = null,
            name = "x",
            id = VariableId("stable:x"),
            identityScope = ModelElementScope.Stable,
            identityOrigin = ModelElementOrigin("pipeline", "x"),
            identityNamespace = "model",
            identitySchemaVersion = "1.0"
        )
        val sourceConstraints = LinearConstraintBatch(
            sparseLhs = SparseMatrix<Flt64>().also { matrix ->
                matrix.addRow(
                    SparseVector<Flt64>().also { row ->
                        row.add(0, Flt64.one)
                    }
                )
            },
            signs = listOf(ModelConstraintRelation.Equal),
            rhs = listOf(Flt64.zero),
            names = listOf("capacity"),
            sources = listOf(ConstraintSource.Origin),
            ids = listOf(ConstraintId("stable:capacity")),
            identityNamespace = "model",
            identitySchemaVersion = "1.0",
            identityScopes = listOf(ModelElementScope.Stable),
            identityOrigins = listOf(sourceOrigin)
        )
        val model = LinearTriadModel(
            impl = BasicLinearTriadModel(listOf(sourceVariable), sourceConstraints, "artifact-model"),
            tokensInSolver = emptyList(),
            objective = Objective(ObjectCategory.Minimum, emptyList())
        )

        val feasibility = model.feasibility()
        val auxiliary = feasibility.variables.drop(1)
        assertTrue(auxiliary.isNotEmpty())
        assertEquals("model", feasibility.identityNamespace)
        assertEquals("1.0", feasibility.identitySchemaVersion)
        assertTrue(auxiliary.all { it.id?.value?.startsWith("artifact:feasibility:") == true })
        assertTrue(auxiliary.all { it.identityNamespace == "model" && it.identitySchemaVersion == "1.0" })
        assertTrue(auxiliary.all { it.identityOrigin == ModelElementOrigin("constraint", "stable:capacity") })
        assertEquals("artifact:feasibility:constraint:stable:capacity", feasibility.constraints.ids.single().value)
        assertEquals(ModelElementScope.Stable, feasibility.constraints.identityScopeAt(0))
        assertEquals(sourceOrigin, feasibility.constraints.identityOriginAt(0))
    }
}
