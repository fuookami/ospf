package fuookami.ospf.kotlin.core.solver.scip

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import fuookami.ospf.kotlin.core.solver.iis.CapabilityAwareInfeasibilityAnalyzer
import fuookami.ospf.kotlin.core.solver.iis.IISConfig
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityEvidenceSource
import fuookami.ospf.kotlin.core.solver.report.SolverModelType

/** Verify the SCIP diagnostic capability contract. / 验证 SCIP 诊断能力契约。 */
class ScipDiagnosticCapabilityTest {
    @Test
    fun linearDescriptorExposesOnlyContinuousFarkas() {
        val solver = ScipLinearSolver()
        assertFalse(solver.descriptor.capabilities.nativeIIS)
        assertTrue(solver.descriptor.capabilities.farkas)

        val analyzers = solver.diagnosticAnalyzers(IISConfig())
        assertEquals(listOf(InfeasibilityEvidenceSource.Farkas), analyzers.map { it.source })
        val capability = (analyzers.single() as CapabilityAwareInfeasibilityAnalyzer<*>).capabilities
        assertTrue(capability.exact)
        assertEquals(setOf(SolverModelType.LP), capability.modelTypes)
    }
}
