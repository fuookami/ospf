package fuookami.ospf.kotlin.core.solver.constraint_programming

import kotlin.test.Test
import kotlin.test.assertEquals
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolution
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.math.algebra.number.Int64

class ConstraintProgrammingSolverOutputTest {
    @Test
    fun solutionShouldProvideStableIdLookupAndTypedOutput() {
        val id = VariableId("amount")
        val solution = ConstraintProgrammingSolution(values = mapOf(id to Int64(9)))
        assertEquals(Int64(9), solution.value(id).value)
        val output = ConstraintProgrammingFeasibleOutput(solution = solution)
        assertEquals(solution, output.solution)
    }
}
